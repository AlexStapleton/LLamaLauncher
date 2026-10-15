use serde::{Deserialize, Serialize};

use serde_json::{json, Value};

use sha2::{Digest, Sha256};

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadFile {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadPlan {
    pub id: String,
    pub repo: String,
    pub revision: String,
    pub root: String,
    pub files: Vec<DownloadFile>,
}

#[derive(Clone)]
pub struct State {
    pub plans: Arc<Mutex<Vec<DownloadPlan>>>,
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub active: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl State {
    pub fn load() -> Self {
        let plans: Vec<DownloadPlan> = crate::storage::load_optional(&path(), 4 * 1024 * 1024)
            .ok()
            .flatten()
            .unwrap_or_default();

        let plans = if plans.len() > 128 || plans.iter().any(|p| validate_plan(p).is_err()) {
            crate::storage::damaged(&path(), "Invalid download library");
            Vec::new()
        } else {
            plans
        };
        Self {
            plans: Arc::new(Mutex::new(plans)),
            semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
            active: Arc::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        crate::storage::write_private_atomic(
            &path(),
            &serde_json::to_vec(&*self.plans.lock().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?,
        )
    }
}

pub fn validate_plan(plan: &DownloadPlan) -> Result<(), String> {
    crate::hub::segments(&plan.repo)?;
    if plan.revision.len() != 40
        || !plan.revision.chars().all(|c| c.is_ascii_hexdigit())
        || plan.files.is_empty()
        || plan.files.len() > 1024
        || !Path::new(&plan.root).is_absolute()
    {
        return Err("Invalid download plan".into());
    }
    let mut names = std::collections::HashSet::new();
    let mut total = 0u64;
    for item in &plan.files {
        safe_name(&item.name)?;
        let key = if cfg!(windows) {
            item.name.to_lowercase()
        } else {
            item.name.clone()
        };
        if !names.insert(key)
            || item.size == 0
            || (!item.sha256.is_empty()
                && (item.sha256.len() != 64 || !item.sha256.chars().all(|c| c.is_ascii_hexdigit())))
        {
            return Err("Invalid or duplicate download file".into());
        }
        total = total
            .checked_add(item.size)
            .ok_or("Download size overflow")?;
    }
    Ok(())
}
pub fn group(plan: &DownloadPlan) -> String {
    let root = fs::canonicalize(&plan.root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| plan.root.clone());
    let value = format!("{root}:{}:{}", plan.repo, plan.revision);
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

fn valid_range(status: u16, header: &str, start: u64, total: u64) -> bool {
    if status != 206 {
        return false;
    }

    let Some(header) = header.strip_prefix("bytes ") else {
        return false;
    };

    let Some((range, length)) = header.split_once('/') else {
        return false;
    };

    let Some((a, b)) = range.split_once('-') else {
        return false;
    };

    matches!((a.parse::<u64>(),b.parse::<u64>(),length.parse::<u64>()),(Ok(a),Ok(b),Ok(n))if a==start&&b>=a&&b<n&&n==total)
}

pub fn reserve(state: &State, plan: &DownloadPlan) -> Result<(), String> {
    if !state
        .active
        .lock()
        .map_err(|e| e.to_string())?
        .insert(group(plan))
    {
        return Err(
            "This repository revision already has an active download. Pause it before resuming."
                .into(),
        );
    }

    Ok(())
}

fn path() -> PathBuf {
    crate::storage::profile_directory().join("downloads.bin")
}

pub(crate) fn safe_name(name: &str) -> Result<(), String> {
    crate::hub::segments(name)?;

    if name.split('/').any(|p| {
        p.ends_with(['.', ' '])
            || p.contains(['<', '>', '"', '|', '?', '*'])
            || matches!(
                p.split('.')
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase()
                    .as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "LPT1"
            )
    }) {
        return Err("Unsafe model file name".into());
    }

    Ok(())
}

pub fn directory(plan: &DownloadPlan) -> Result<PathBuf, String> {
    let root = fs::canonicalize(&plan.root).map_err(|_| "Select an existing models folder")?;

    let parts = crate::hub::segments(&plan.repo)?;

    if parts.len() != 2
        || plan.revision.len() != 40
        || !plan.revision.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Invalid download revision".into());
    }

    let mut d = root.clone();

    for part in [".launcher-downloads", parts[0], parts[1], &plan.revision] {
        safe_name(part)?;

        d.push(part);

        if d.exists() {
            if fs::symlink_metadata(&d)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Download directory cannot be a symlink".into());
            }
        } else {
            fs::create_dir(&d).map_err(|_| "Cannot create download directory")?;
        }

        if !fs::canonicalize(&d)
            .map_err(|e| e.to_string())?
            .starts_with(&root)
        {
            return Err("Download escaped models folder".into());
        }
    }

    Ok(d)
}

pub fn destination(dir: &Path, name: &str) -> Result<PathBuf, String> {
    safe_name(name)?;

    let mut parent = dir.to_owned();

    let parts: Vec<_> = name.split('/').collect();

    for p in &parts[..parts.len() - 1] {
        parent.push(p);

        if parent.exists() {
            if fs::symlink_metadata(&parent)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Download path contains symlink".into());
            }
        } else {
            fs::create_dir(&parent).map_err(|e| e.to_string())?;
        }
    }

    let path = parent.join(parts[parts.len() - 1]);

    if path.exists()
        && fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("Download target is a symlink".into());
    }

    Ok(path)
}

pub async fn plan(
    hub: &crate::hub::State,
    repo: String,
    root: String,
    mut names: Vec<String>,
) -> Result<DownloadPlan, String> {
    names.sort();
    names.dedup();
    if names.is_empty() || names.len() > 1024 {
        return Err("Select GGUF files to download".into());
    }

    let info = crate::hub::details(hub, &repo).await?;

    let mut files = Vec::new();

    for name in &names {
        safe_name(name)?;

        let f = info["files"]
            .as_array()
            .and_then(|a| a.iter().find(|f| f["rfilename"].as_str() == Some(name)))
            .ok_or("Selected file no longer exists")?;

        let size = f["lfs"]["size"]
            .as_u64()
            .or_else(|| f["size"].as_u64())
            .ok_or("Hub did not provide file size")?;

        if size == 0 {
            return Err("Empty model file".into());
        }

        let sha = f["lfs"]["sha256"]
            .as_str()
            .or_else(|| f["lfs"]["oid"].as_str())
            .unwrap_or("")
            .to_owned();

        files.push(DownloadFile {
            name: name.clone(),
            size,
            sha256: sha,
        });
    }

    for name in &names {
        if let Some((stem, tail)) = name.rsplit_once("-of-") {
            if let Some((prefix, index)) = stem.rsplit_once('-') {
                if index.len() == 5 && index.chars().all(|c| c.is_ascii_digit()) {
                    let count = tail
                        .trim_end_matches(".gguf")
                        .parse::<usize>()
                        .map_err(|_| "Invalid split GGUF name")?;

                    if count > 1024 || count == 0 {
                        return Err("Invalid shard count".into());
                    }

                    for i in 1..=count {
                        if !names.contains(&format!("{prefix}-{i:05}-of-{count:05}.gguf")) {
                            return Err("Select every shard in the split model".into());
                        }
                    }
                }
            }
        }
    }

    let p = DownloadPlan {
        id: format!("hf-{}", crate::monitor::now()),
        repo,
        revision: info["sha"].as_str().ok_or("Missing revision")?.into(),
        root,
        files,
    };

    directory(&p)?;

    Ok(p)
}

pub fn hash(path: &Path, cancel: &AtomicBool) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|_| "Cannot verify download")?;

    let mut h = Sha256::new();

    let mut buf = vec![0; 1024 * 1024];

    loop {
        crate::jobs::check(cancel)?;

        let n = f.read(&mut buf).map_err(|_| "Cannot verify download")?;

        if n == 0 {
            break;
        }

        h.update(&buf[..n]);
    }

    Ok(format!("{:x}", h.finalize()))
}

pub fn disk_space(path: &Path, required: u64) -> Result<(), String> {
    let disks = sysinfo::Disks::new_with_refreshed_list();

    if let Some(d) = disks
        .list()
        .iter()
        .filter(|d| path.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
    {
        if d.available_space() < required.saturating_add(64 * 1024 * 1024) {
            return Err("Insufficient free disk space for download".into());
        }
    }

    Ok(())
}

async fn response(
    url: reqwest::Url,
    token: &str,
    offset: u64,
    validator: &str,
) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Cannot create download client")?;

    let mut url = url;

    for _ in 0..10 {
        if url.scheme() != "https" {
            return Err("Download redirect must use HTTPS".into());
        }

        let mut r = client.get(url.clone());

        if url.host_str() == Some("huggingface.co") && !token.is_empty() {
            r = r.bearer_auth(token);
        }

        if offset > 0 {
            r = r.header("Range", format!("bytes={offset}-"));

            if !validator.is_empty() {
                r = r.header("If-Range", validator);
            }
        }

        let response = r
            .send()
            .await
            .map_err(|_| "Download connection failed; retry to resume")?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or("Invalid download redirect")?;

            url = url
                .join(location)
                .map_err(|_| "Invalid download redirect")?;

            continue;
        }

        return Ok(response);
    }

    Err("Too many download redirects".into())
}

async fn fetch(
    url: reqwest::Url,
    token: &str,
    offset: u64,
    validator: &str,
    cancel: &AtomicBool,
) -> Result<reqwest::Response, String> {
    for attempt in 0..3 {
        let response = tokio::select! {
        _=crate::session::cancelled(cancel)=>return Err("Paused; partial download kept".into()),r=tokio::time::timeout(std::time::Duration::from_secs(45),response(url.clone(),token,offset,validator))=>r.map_err(|_|"Download connection timed out; retry to resume")??};

        if response.status().as_u16() == 429 && attempt < 2 {
            let wait = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(30)
                .clamp(1, 60);

            tokio::select! {
            _=crate::session::cancelled(cancel)=>return Err("Paused during rate-limit wait".into()),_=tokio::time::sleep(std::time::Duration::from_secs(wait))=>{
            }
            }

            continue;
        }

        if response.status().as_u16() == 403
            && response.url().host_str() != Some("huggingface.co")
            && attempt < 2
        {
            continue;
        }

        return Ok(response);
    }

    Err("Download could not be resumed; retry later".into())
}

async fn hash_async(path: PathBuf, cancel: Arc<AtomicBool>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || hash(&path, &cancel))
        .await
        .map_err(|_| "File verification worker failed".to_string())?
}

pub async fn file(
    plan: &DownloadPlan,
    item: &DownloadFile,
    token: &str,
    cancel: &Arc<AtomicBool>,
    mut progress: impl FnMut(u64, u64, f64),
) -> Result<PathBuf, String> {
    let dir = directory(plan)?;

    let dest = destination(&dir, &item.name)?;

    let part = dest.with_file_name(format!(
        "{}.part",
        dest.file_name()
            .ok_or("Missing file name")?
            .to_string_lossy()
    ));

    let manifest = part.with_extension("resume.bin");

    let identity = format!("{}:{}:{}", plan.repo, plan.revision, item.name);

    let receipt = dest.with_file_name(format!(
        "{}.verified.bin",
        dest.file_name()
            .ok_or("Missing file name")?
            .to_string_lossy()
    ));

    if dest.exists() {
        let verified = crate::storage::read_private(&receipt, 8192)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok());

        let expected = if !item.sha256.is_empty() {
            item.sha256.clone()
        } else {
            verified
                .as_ref()
                .filter(|v| v["identity"].as_str() == Some(&identity))
                .and_then(|v| v["sha256"].as_str())
                .unwrap_or("")
                .to_owned()
        };

        if fs::metadata(&dest).map_err(|e| e.to_string())?.len() == item.size
            && !expected.is_empty()
        {
            let d = dest.clone();

            let c = cancel.load(std::sync::atomic::Ordering::Relaxed);

            if !c && hash_async(d, cancel.clone()).await? == expected {
                return Ok(dest);
            }
        }

        return Err("Destination already exists. Choose another folder; unrelated files are never overwritten.".into());
    }

    if part.exists()
        && fs::symlink_metadata(&part)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("Partial file cannot be a symlink".into());
    }

    let saved = crate::storage::read_private(&manifest, 8192)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());

    let valid = saved
        .as_ref()
        .is_some_and(|v| v["identity"].as_str() == Some(&identity));

    if part.exists() && !valid {
        return Err("Unrecognized partial file; remove it explicitly before downloading".into());
    }

    let mut offset = if valid {
        fs::metadata(&part).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    if offset > item.size {
        offset = 0;
    }
    if offset == item.size && valid {
        let digest = hash_async(part.clone(), cancel.clone()).await?;
        if !item.sha256.is_empty() && digest != item.sha256 {
            fs::remove_file(&part).map_err(|_| "Cannot remove corrupt partial")?;
            return Err("SHA256 mismatch. Corrupt partial removed; retry.".into());
        }
        crate::storage::write_private_atomic(
            &receipt,
            &serde_json::to_vec(&json!({"identity":identity,"sha256":digest}))
                .map_err(|e| e.to_string())?,
        )?;
        crate::jobs::check(cancel)?;
        crate::storage::publish_new(&part, &dest)?;
        let _ = fs::remove_file(&manifest);
        progress(item.size, item.size, 0.0);
        return Ok(dest);
    }

    let validator = saved
        .as_ref()
        .and_then(|v| v["etag"].as_str())
        .unwrap_or("");

    disk_space(&dir, item.size - offset)?;

    let mut parts = crate::hub::segments(&plan.repo)?;

    parts.extend(["resolve", plan.revision.as_str()]);

    parts.extend(crate::hub::segments(&item.name)?);

    let url = reqwest::Url::parse(&format!(
        "https://huggingface.co{}",
        crate::hub::url_path(&parts)?
    ))
    .map_err(|_| "Invalid download URL")?;

    let mut response = fetch(url.clone(), token, offset, validator, cancel).await?;

    if offset > 0 {
        let range_ok = valid_range(
            response.status().as_u16(),
            response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            offset,
            item.size,
        );

        if !range_ok {
            offset = 0;

            response = fetch(url, token, 0, "", cancel).await?;
        }
    }

    let status = response.status().as_u16();

    if status == 401 || status == 403 {
        return Err("Authentication or gated access required. Open the model page and sign in/request access.".into());
    }

    if status == 429 {
        return Err("Download rate limited; wait and retry".into());
    }

    if !matches!(status, 200 | 206) {
        return Err(format!("Download HTTP {status}; retry to resume"));
    }

    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();

    crate::storage::write_private_atomic(
        &manifest,
        &serde_json::to_vec(&json!({
        "identity":identity,"etag":etag}
        ))
        .map_err(|e| e.to_string())?,
    )?;

    use tokio::io::AsyncWriteExt;
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(offset == 0)
        .append(offset > 0)
        .open(&part)
        .await
        .map_err(|_| "Cannot create partial download")?;

    let start = Instant::now();

    let initial = offset;

    let mut last = Instant::now();

    loop {
        let c = tokio::select! {
        _=crate::session::cancelled(cancel)=>return Err("Paused; partial download kept for resume".into()),c=tokio::time::timeout(std::time::Duration::from_secs(60),response.chunk())=>c.map_err(|_|"Download stalled; retry to resume")?.map_err(|_|"Download interrupted; retry to resume")?};

        let Some(c) = c else { break };

        if offset.saturating_add(c.len() as u64) > item.size {
            return Err("Download exceeded published file size".into());
        }

        f.write_all(&c)
            .await
            .map_err(|_| "Cannot write download; check disk space")?;

        offset += c.len() as u64;

        if last.elapsed().as_millis() > 250 {
            progress(
                offset,
                item.size,
                (offset - initial) as f64 / start.elapsed().as_secs_f64().max(0.001),
            );

            last = Instant::now();
        }
    }

    f.flush().await.map_err(|_| "Cannot flush download")?;
    f.sync_all().await.map_err(|_| "Cannot flush download")?;

    drop(f);

    if offset != item.size {
        return Err("Download incomplete; resume to finish".into());
    }

    crate::jobs::check(cancel)?;

    {
        let expected = item.sha256.clone();
        let digest = hash_async(part.clone(), cancel.clone()).await?;

        if !expected.is_empty() && digest != expected {
            fs::remove_file(&part).map_err(|_| "Corrupt download could not be removed")?;

            return Err("SHA256 mismatch. Corrupt partial removed; retry.".into());
        }

        crate::storage::write_private_atomic(
            &receipt,
            &serde_json::to_vec(&json!({
            "identity":identity,"sha256":digest}
            ))
            .map_err(|e| e.to_string())?,
        )?;
    }

    crate::storage::publish_new(&part, &dest)?;

    let _ = fs::remove_file(manifest);

    progress(offset, item.size, 0.0);

    Ok(dest)
}

pub fn start(
    app: tauri::AppHandle,
    state: State,
    hub: crate::hub::State,
    jobs: crate::jobs::State,
    mut plan: DownloadPlan,
) -> Result<String, String> {
    validate_plan(&plan)?;
    reserve(&state, &plan)?;

    let (id, cancel) = match jobs.begin("download") {
        Ok(v) => v,
        Err(e) => {
            state
                .active
                .lock()
                .map_err(|e| e.to_string())?
                .remove(&group(&plan));

            return Err(e);
        }
    };

    plan.id = id.clone();

    {
        let mut plans = state.plans.lock().map_err(|e| e.to_string())?;

        if plans.len() >= 128 {
            state
                .active
                .lock()
                .map_err(|e| e.to_string())?
                .remove(&group(&plan));

            jobs.finish(&app, &id, Err("Download library full".into()));

            return Err("Download library full".into());
        }

        plans.push(plan.clone());
    }

    if let Err(e) = state.save() {
        state
            .plans
            .lock()
            .map_err(|e| e.to_string())?
            .retain(|p| p.id != id);
        state
            .active
            .lock()
            .map_err(|e| e.to_string())?
            .remove(&group(&plan));

        jobs.finish(&app, &id, Err(e.clone()));

        return Err(e);
    }

    launch(app, state, hub, jobs, plan, cancel);

    Ok(id)
}

pub fn launch(
    app: tauri::AppHandle,
    state: State,
    hub: crate::hub::State,
    jobs: crate::jobs::State,
    plan: DownloadPlan,
    cancel: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        struct GroupGuard(State, String);

        impl Drop for GroupGuard {
            fn drop(&mut self) {
                if let Ok(mut active) = self.0.active.lock() {
                    active.remove(&self.1);
                }
            }
        }

        let _group_guard = GroupGuard(state.clone(), group(&plan));

        let result=async {
            let lease_dir=directory(&plan)?;
            let _lease=crate::storage::acquire_named_lock(&lease_dir,"download.lock")?;
            let shared=Arc::new(plan.clone());
            let token=Arc::new(hub.token());
            let next=Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let progress=Arc::new(Mutex::new(vec![(0u64,0f64);shared.files.len()]));
            let total=shared.files.iter().try_fold(0u64,|n,f|n.checked_add(f.size)).ok_or("Download size overflow")?;
            let mut handles=Vec::with_capacity(2);
            for _ in 0..2 {
                let (s,a,j,p,c,t,next,progress)=(state.clone(),app.clone(),jobs.clone(),shared.clone(),cancel.clone(),token.clone(),next.clone(),progress.clone());
                handles.push(tokio::spawn(async move {
                    let mut paths=Vec::new();
                    loop {
                        crate::jobs::check(&c)?;
                        let index=next.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
                        let Some(item)=p.files.get(index) else { break; };
                        let permit=tokio::select!{_=crate::session::cancelled(&c)=>return Err("Paused".into()),r=s.semaphore.acquire()=>r.map_err(|_|"Download queue closed")?};
                        let update=|done:u64,speed:f64| {
                            if let Ok(mut progress)=progress.lock() {
                                progress[index]=(done,speed);
                                let completed=progress.iter().map(|p|p.0).sum::<u64>();
                                let rate=progress.iter().map(|p|p.1).sum::<f64>();
                                j.update(&a,&p.id,|r| { r.completed=completed; r.total=total; r.message=format!("{} · {:.1} MiB/s · {} files",item.name,rate/1048576.0,p.files.len()); });
                            }
                        };
                        let result=file(&p,item,&t,&c,|done,_,speed|update(done,speed)).await;
                        drop(permit);
                        match result { Ok(path)=>{update(item.size,0.0);paths.push((index,path));},Err(error)=>{c.store(true,std::sync::atomic::Ordering::Relaxed);return Err(error);} }
                    }
                    Ok::<_,String>(paths)
                }));
            }
            let mut paths=Vec::new(); let mut error=None;
            for handle in handles {
                match handle.await.map_err(|_|"Download worker failed")? {
                    Ok(done)=>paths.extend(done), Err(e)=>{ if error.as_ref().is_none_or(|old:&String|old.starts_with("Cancelled")||old.starts_with("Paused")) { error=Some(e); } }
                }
            }
            if let Some(error)=error { return Err(error); }
            paths.sort_by_key(|p|p.0); let paths=paths.into_iter().map(|p|p.1).collect::<Vec<_>>();

            let plan=plan.clone();let cancelled=cancel.clone();
            tauri::async_runtime::spawn_blocking(move||{
            crate::jobs::check(&cancelled)?;
            let mut models=Vec::new();

            for path in &paths {
crate::jobs::check(&cancelled)?;

                let name=path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();

                if name.contains("mmproj")||(name.contains("-of-")&&!name.contains("-00001-of-")){
continue;
}

                let inspected=crate::gguf::inspect(path,4096,1)?;

                if inspected.architecture=="clip"{
continue;
}

                let id=crate::registry::register_model(&plan,path)?;

                models.push(json!({
"model":path,"model_id":id,"name":name}
));

            }

            let main=models.first().ok_or("Download contains no main model")?;

            let result=json!({
"model":main["model"],"directory":plan.root,"model_id":main["model_id"],"models":models}
);

            let dir=directory(&plan)?;

            let marker=dir.join("complete.json");

            let mut complete=plan.clone();

            if fs::metadata(&marker).is_ok_and(|m|m.len()<=4*1024*1024){
            if let Ok(previous)=fs::read(&marker){
if let Ok(previous)=serde_json::from_slice::<DownloadPlan>(&previous){
for item in previous.files{
if !complete.files.iter().any(|f|f.name==item.name){
complete.files.push(item);
}
}
}
}

            }
            crate::storage::write_public_atomic(&marker,&serde_json::to_vec(&complete).map_err(|e|e.to_string())?)?;

            Ok::<_,String>(result)
            }).await.map_err(|_|"Model finalization worker failed")?

        }
.await;

        jobs.finish(&app, &plan.id, result);
    });
}

#[cfg(test)]
mod tests {

    use super::*;
    #[tokio::test]
    async fn a_complete_owned_partial_is_verified_and_published_without_network() {
        let root = std::env::temp_dir().join(format!("complete-partial-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let bytes = b"complete fixture";
        let plan = DownloadPlan {
            id: "fixture".into(),
            repo: "owner/repo".into(),
            revision: "a".repeat(40),
            root: root.to_string_lossy().into(),
            files: vec![DownloadFile {
                name: "model.gguf".into(),
                size: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(bytes)),
            }],
        };
        let dir = directory(&plan).unwrap();
        let part = dir.join("model.gguf.part");
        fs::write(&part, bytes).unwrap();
        crate::storage::write_private_atomic(&part.with_extension("resume.bin"),&serde_json::to_vec(&json!({"identity":format!("{}:{}:{}",plan.repo,plan.revision,plan.files[0].name),"etag":"fixture"})).unwrap()).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let path = file(&plan, &plan.files[0], "", &cancel, |_, _, _| {})
            .await
            .unwrap();
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert!(!part.exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn many_file_plans_are_bounded_and_duplicate_selections_rejected() {
        let root = std::env::temp_dir();
        let mut plan = DownloadPlan {
            id: "fixture".into(),
            repo: "owner/repo".into(),
            revision: "a".repeat(40),
            root: root.to_string_lossy().into(),
            files: (0..1024)
                .map(|i| DownloadFile {
                    name: format!("{i}.gguf"),
                    size: 1024,
                    sha256: String::new(),
                })
                .collect(),
        };
        assert!(validate_plan(&plan).is_ok());
        plan.files.push(plan.files[0].clone());
        assert!(validate_plan(&plan).is_err());
        plan.files.pop();
        plan.files[1] = plan.files[0].clone();
        assert!(validate_plan(&plan).is_err());
    }

    #[test]
    fn unsafe_names_rejected() {
        for n in [
            "../x", "/x", "C:/x", "x\\y", "CON.gguf", "x/../y", "a?.gguf",
        ] {
            assert!(safe_name(n).is_err(), "{n}");
        }

        assert!(safe_name("sub/model.gguf").is_ok());
    }

    #[test]
    fn resuming_requires_exact_valid_content_range() {
        assert!(valid_range(206, "bytes 20-99/100", 20, 100));

        for (status, h) in [
            (200, "bytes 20-99/100"),
            (206, "bytes 0-99/100"),
            (206, "bytes 20-100/100"),
            (206, "bytes 20-19/100"),
            (206, "bytes 20-99/101"),
            (206, "bytes */100"),
        ] {
            assert!(!valid_range(status, h, 20, 100));
        }
    }

    #[test]
    fn verification_is_cancellable_and_detects_changed_bytes() {
        let p = std::env::temp_dir().join(format!("download-hash-{}", std::process::id()));

        fs::write(&p, b"model bytes").unwrap();

        let a = hash(&p, &AtomicBool::new(false)).unwrap();

        assert_eq!(a.len(), 64);

        fs::write(&p, b"changed bytes").unwrap();

        assert_ne!(a, hash(&p, &AtomicBool::new(false)).unwrap());

        assert!(hash(&p, &AtomicBool::new(true)).is_err());

        fs::remove_file(p).unwrap();
    }
}
