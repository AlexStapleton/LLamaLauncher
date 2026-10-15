use serde::{Deserialize, Serialize};

use serde_json::{json, Value};

use sha2::{Digest, Sha256};

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

static LOCK: Mutex<()> = Mutex::new(());
static ACTIVE_STAGES: std::sync::LazyLock<Mutex<std::collections::HashSet<PathBuf>>> =
    std::sync::LazyLock::new(Default::default);

#[derive(Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub path: String,
    pub version: String,
    pub managed: bool,
    pub details: Value,
}

fn path() -> PathBuf {
    crate::storage::profile_directory().join("artifacts.bin")
}

pub fn list() -> Result<Vec<Artifact>, String> {
    if !path().exists() {
        return Ok(Vec::new());
    }

    serde_json::from_slice(&crate::storage::read_private(&path(), 4 * 1024 * 1024)?)
        .map_err(|_| crate::storage::damaged(&path(), "Artifact registry damaged"))
}

pub fn add(mut artifact: Artifact) -> Result<String, String> {
    let _lock = LOCK.lock().map_err(|e| e.to_string())?;

    let mut artifacts = list()?;

    let hash = Sha256::digest(
        format!("{}:{}:{}", artifact.kind, artifact.path, artifact.version).as_bytes(),
    );

    artifact.id = format!("{}-{:x}", artifact.kind, hash);

    let id = artifact.id.clone();

    artifacts.retain(|a| a.id != id);

    if artifacts.len() >= 1024 {
        return Err("Artifact registry full".into());
    }

    artifacts.push(artifact);

    let bytes = serde_json::to_vec(&artifacts).map_err(|e| e.to_string())?;

    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Artifact registry exceeded 4 MiB; reduce very large libraries".into());
    }

    crate::storage::write_private_atomic(&path(), &bytes)?;

    Ok(id)
}

pub fn register_model(
    plan: &crate::downloads::DownloadPlan,
    model: &Path,
) -> Result<String, String> {
    add(Artifact {
        id: String::new(),
        kind: "model".into(),
        name: plan.repo.clone(),
        path: model.to_string_lossy().into(),
        version: plan.revision.clone(),
        managed: true,
        details: json!({
        "repo":plan.repo,"root":plan.root,"files":plan.files}
        ),
    })
}

pub fn resolve(id: &str, kind: &str, path: &str) -> Result<(), String> {
    if id.is_empty() {
        return Ok(());
    }

    let artifact = list()?
        .into_iter()
        .find(|a| a.id == id && a.kind == kind)
        .ok_or("Saved artifact no longer exists. Relink this profile explicitly.")?;

    let actual =
        fs::canonicalize(path).map_err(|_| "Saved artifact is missing. Relink this profile.")?;

    let registered = fs::canonicalize(artifact.path)
        .map_err(|_| "Registered artifact is missing. Relink this profile.")?;

    if actual != registered {
        return Err(
            "Path does not match registered artifact. Relink or select the path manually.".into(),
        );
    }

    Ok(())
}

pub fn runtimes_dir() -> PathBuf {
    crate::storage::profile_directory().join("runtimes")
}

pub fn probe(binary: &Path, flag: &str) -> Result<String, String> {
    probe_cancel(binary, flag, &std::sync::atomic::AtomicBool::new(false))
}
fn probe_cancel(
    binary: &Path,
    flag: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<String, String> {
    let cache = crate::storage::profile_directory().join("probes");

    fs::create_dir_all(&cache).map_err(|_| "Cannot create probe directory")?;

    let output_path = cache.join(format!(
        "{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));

    let _guard = crate::server::Snapshot(output_path.clone());

    let output = fs::File::create(&output_path).map_err(|_| "Cannot create probe output")?;

    let mut command = std::process::Command::new(binary);

    command
        .arg(flag)
        .current_dir(binary.parent().ok_or("Missing runtime folder")?)
        .stdin(std::process::Stdio::null())
        .stdout(output.try_clone().map_err(|e| e.to_string())?)
        .stderr(output);

    crate::configure_runtime_environment(&mut command);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(0x08000000);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.process_group(0);
    }

    let mut child = crate::server::TemporaryProcess::new(command.spawn().map_err(|_| {
        "Cannot start runtime. Check its dependencies and executable permissions."
    })?)?;

    let start = std::time::Instant::now();

    loop {
        crate::jobs::check(cancel)?;
        if fs::metadata(&output_path).is_ok_and(|m| m.len() > 1024 * 1024) {
            return Err("Runtime probe exceeded 1 MiB".into());
        }

        if let Some(exit) = child.try_wait().map_err(|e| e.to_string())? {
            if !exit.success() {
                return Err("Runtime probe failed. Check its shared libraries, drivers and platform compatibility.".into());
            }

            return fs::read_to_string(&output_path)
                .map_err(|_| "Cannot read runtime capabilities".into());
        }

        if start.elapsed().as_secs() >= 15 {
            return Err("Runtime probe timed out".into());
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

pub fn import(binary: &Path, managed: bool, version: Option<String>) -> Result<Artifact, String> {
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut a = inspect_runtime(binary, version, managed, &cancel, false)?;
    a.id = add(a.clone())?;

    Ok(a)
}

pub async fn releases() -> Result<Value, String> {
    let c = crate::hub::client()?;

    let response = c
        .get("https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=10")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Cannot fetch official releases")?;

    let v = crate::hub::decode(response).await?;

    let os = if cfg!(windows) { "win" } else { "ubuntu" };

    let rows: Vec<_> = v
        .as_array()
        .ok_or("Invalid release list")?
        .iter()
        .map(|r| {
            let assets: Vec<_> = r["assets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|a| {
                    let n = a["name"].as_str().unwrap_or("").to_ascii_lowercase();

                    n.starts_with("llama-")
                        && n.contains(os)
                        && n.contains("x64")
                        && (n.ends_with(".zip") || n.ends_with(".tar.gz"))
                })
                .cloned()
                .collect();

            json!({
            "tag":r["tag_name"],"assets":assets}
            )
        })
        .collect();

    Ok(json!({
    "releases":rows}
    ))
}

async fn github_release(path: &str) -> Result<Value, String> {
    let response = crate::hub::client()?
        .get(format!(
            "https://api.github.com/repos/ggml-org/llama.cpp/{path}"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Cannot reach llama.cpp GitHub releases")?;
    crate::hub::decode(response)
        .await
        .map_err(|e| e.replace("Hugging Face", "GitHub"))
}

fn channel_name(channel: &str) -> Result<&str, String> {
    match channel {
        "stable" => Ok("Stable"),
        "beta" => Ok("Beta"),
        _ => Err("Choose Stable or Beta".into()),
    }
}

fn choose_asset(release: &Value, os: &str, vulkan: bool) -> Result<(Value, &'static str), String> {
    let assets = release["assets"]
        .as_array()
        .ok_or("Release has no download files")?;
    let candidates: Vec<_> = assets
        .iter()
        .filter(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name.starts_with("llama-")
                && name.contains(&format!("-bin-{os}-"))
                && name.contains("x64")
                && (name.ends_with(".zip") || name.ends_with(".tar.gz"))
        })
        .collect();
    if vulkan {
        if let Some(a) = candidates.iter().find(|a| {
            a["name"]
                .as_str()
                .is_some_and(|n| n.contains("-vulkan-x64."))
        }) {
            return Ok(((*a).clone(), "Vulkan GPU"));
        }
    }
    candidates
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .is_some_and(|n| n.contains("-cpu-x64.") || n.contains("-bin-ubuntu-x64."))
        })
        .map(|a| ((*a).clone(), "CPU"))
        .ok_or_else(|| {
            "This official release has no compatible x64 binary for your platform".into()
        })
}

fn stable_binary_tag(text: &str) -> Result<&str, String> {
    let tag = text.trim();
    if tag.starts_with('b')
        && tag.len() > 1
        && tag.len() < 32
        && tag[1..].chars().all(|c| c.is_ascii_digit())
    {
        Ok(tag)
    } else {
        Err("The stable release has an invalid binary tag; no beta build was substituted".into())
    }
}

pub async fn channel_plan(channel: &str) -> Result<Value, String> {
    let label = channel_name(channel)?;
    if !cfg!(target_arch = "x86_64") {
        return Err("Automatic installation currently supports Windows/Linux x64".into());
    }
    let os = if cfg!(windows) { "win" } else { "ubuntu" };
    let mut release = if channel == "stable" {
        github_release("releases/latest").await?
    } else {
        let releases = github_release("releases?per_page=30").await?;
        releases
            .as_array()
            .ok_or("Invalid release list")?
            .iter()
            .find(|r| {
                r["prerelease"].as_bool() == Some(true)
                    && r["draft"].as_bool() != Some(true)
                    && choose_asset(r, os, false).is_ok()
            })
            .cloned()
            .ok_or("No official beta binary is available for this platform")?
    };
    if channel == "stable"
        && (release["draft"].as_bool() == Some(true)
            || release["prerelease"].as_bool() == Some(true))
    {
        return Err("GitHub did not return an official stable release".into());
    }
    let version = release["tag_name"]
        .as_str()
        .ok_or("Release has no version")?
        .to_owned();
    if channel == "stable" && choose_asset(&release, os, false).is_err() {
        let marker=release["assets"].as_array().and_then(|a|a.iter().find(|a|a["name"]=="nightly-tag.txt"))
            .ok_or("The stable release does not publish compatible binaries or a binary tag; no beta build was substituted")?;
        let url = marker["browser_download_url"]
            .as_str()
            .ok_or("Missing stable binary tag URL")?;
        if !url.starts_with("https://github.com/ggml-org/llama.cpp/releases/download/")
            || marker["size"].as_u64().is_none_or(|n| n > 1024)
        {
            return Err("Invalid official stable binary tag file".into());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|_| "Cannot create release client")?;
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|_| "Cannot download the stable binary tag")?;
        if !response.status().is_success() {
            return Err("Stable binary tag download was rejected".into());
        }
        let mut text = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Stable binary tag was interrupted")?
        {
            if text.len() + chunk.len() > 1024 {
                return Err("Stable binary tag is oversized".into());
            }
            text.extend(chunk);
        }
        if text.len() as u64 != marker["size"].as_u64().unwrap_or_default() {
            return Err("Stable binary tag size mismatch".into());
        }
        if let Some(digest) = marker["digest"]
            .as_str()
            .and_then(|d| d.strip_prefix("sha256:"))
        {
            if format!("{:x}", Sha256::digest(&text)) != digest {
                return Err("Stable binary tag checksum mismatch".into());
            }
        }
        let tag = stable_binary_tag(
            std::str::from_utf8(&text).map_err(|_| "Invalid stable binary tag text")?,
        )?;
        release = github_release(&format!("releases/tags/{tag}")).await?;
        if release["tag_name"] != tag {
            return Err("Stable binary release tag mismatch".into());
        }
    }
    let gpus = crate::monitor::adapters().await;
    let detected = gpus.iter().any(|a| {
        a["name"].as_str().is_some_and(|s| {
            let n = s.to_ascii_lowercase();
            ["nvidia", "amd", "radeon", "intel"]
                .iter()
                .any(|brand| n.contains(brand))
        })
    });
    #[cfg(windows)]
    let loader = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .is_some_and(|p| p.join("System32/vulkan-1.dll").is_file());
    #[cfg(not(windows))]
    let loader = [
        "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
        "/usr/lib64/libvulkan.so.1",
        "/usr/lib/libvulkan.so.1",
    ]
    .iter()
    .any(|p| Path::new(p).is_file());
    let (asset, backend) = choose_asset(&release, os, detected && loader)?;
    let mut candidates = Vec::new();
    if gpus.iter().any(|g| {
        g["name"]
            .as_str()
            .is_some_and(|n| n.to_ascii_lowercase().contains("nvidia"))
    }) {
        for a in release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|a| {
                a["name"].as_str().is_some_and(|n| {
                    n.starts_with("llama-")
                        && n.contains(&format!("-bin-{os}-cuda-"))
                        && n.contains("x64")
                        && n.ends_with(".zip")
                })
            })
        {
            candidates.push(json!({"asset_id":a["id"],"backend":"CUDA GPU"}));
        }
    }
    if backend != "CPU" {
        candidates.push(json!({"asset_id":asset["id"],"backend":backend}));
    }
    let (cpu, _) = choose_asset(&release, os, false)?;
    candidates.push(json!({"asset_id":cpu["id"],"backend":"CPU"}));
    Ok(
        json!({"channel":channel,"label":label,"version":version,"tag":release["tag_name"],"asset_id":asset["id"],"asset":asset["name"],"size":asset["size"],"backend":backend,"candidates":candidates,
        "reason":if backend=="CPU"{"CPU selected because a compatible GPU/driver could not be detected or this release has no Vulkan build."}else{"Vulkan selected for the detected GPU and installed graphics driver."}}),
    )
}

#[derive(Serialize, Deserialize)]
struct InstallReceipt {
    version: u32,
    artifact: Artifact,
    archives: Vec<Value>,
    files: Vec<(String, String)>,
    gpu_required: bool,
}
fn gpu_devices(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        ["Vulkan", "CUDA", "SYCL", "Metal"].iter().any(|backend| {
            line.strip_prefix(backend)
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
    })
}
fn inspect_runtime(
    binary: &Path,
    version: Option<String>,
    managed: bool,
    cancel: &std::sync::atomic::AtomicBool,
    require_gpu: bool,
) -> Result<Artifact, String> {
    let binary = fs::canonicalize(binary).map_err(|_| "Runtime executable missing")?;
    let detected = probe_cancel(&binary, "--version", cancel)?;
    let devices = probe_cancel(&binary, "--list-devices", cancel);
    if require_gpu && !devices.as_ref().is_ok_and(|v| gpu_devices(v)) {
        return Err("Downloaded GPU backend cannot enumerate a usable local device; trying the next backend".into());
    }
    Ok(Artifact {
        id: String::new(),
        kind: "runtime".into(),
        name: binary
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path: binary.to_string_lossy().into(),
        version: version.unwrap_or(detected.clone()),
        managed,
        details: json!({"version_output":detected,"devices":devices.unwrap_or_else(|_|"Device enumeration unsupported; GPU compatibility unknown".into()),"gpu_verified":require_gpu}),
    })
}
fn file_manifest(
    folder: &Path,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<(String, String)>, String> {
    let root = fs::canonicalize(folder).map_err(|_| "Cannot inspect installed files")?;
    let mut pending = vec![folder.to_owned()];
    let mut files = Vec::new();
    let mut count = 0;
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(|_| "Cannot inspect installed files")? {
            crate::jobs::check(cancel)?;
            count += 1;
            if count > 20000 {
                return Err("Installed file manifest exceeds limits".into());
            }
            let entry = entry.map_err(|_| "Cannot inspect installed file")?;
            let kind = entry
                .file_type()
                .map_err(|_| "Cannot inspect installed file")?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() || kind.is_symlink() {
                let resolved =
                    fs::canonicalize(entry.path()).map_err(|_| "Cannot inspect installed file")?;
                if !resolved.starts_with(&root) || !resolved.is_file() {
                    return Err("Installed file escapes runtime folder".into());
                }
                let relative = entry
                    .path()
                    .strip_prefix(folder)
                    .map_err(|_| "Invalid installed path")?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((relative, crate::downloads::hash(&resolved, cancel)?));
            }
        }
    }
    Ok(files)
}
fn reconcile(
    folder: &Path,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Option<Artifact>, String> {
    let receipt_path = folder.join("install-receipt.bin");
    if !receipt_path.is_file() {
        return Ok(None);
    }
    let receipt: InstallReceipt = crate::storage::load_optional(&receipt_path, 4 * 1024 * 1024)?
        .ok_or("Missing install receipt")?;
    if receipt.version != 1 || receipt.files.is_empty() {
        return Err("Unsupported install receipt; files preserved".into());
    }
    let root = fs::canonicalize(folder).map_err(|_| "Cannot inspect install folder")?;
    for (name, expected) in &receipt.files {
        crate::downloads::safe_name(name)?;
        let file = fs::canonicalize(folder.join(name))
            .map_err(|_| "Installed files changed; preserved")?;
        if !file.starts_with(&root)
            || expected.len() != 64
            || !expected.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Installed files failed receipt verification; preserved".into());
        }
    }
    let mut actual = file_manifest(folder, cancel)?;
    actual.retain(|(name, _)| name != "install-receipt.bin" && name != "install-receipt.bin.bak");
    let mut expected = receipt.files.clone();
    actual.sort();
    expected.sort();
    if actual != expected {
        if actual
            .iter()
            .map(|p| &p.0)
            .eq(expected.iter().map(|p| &p.0))
        {
            return Err("Installed files failed receipt verification; preserved".into());
        }
        return Err("Installed file set changed; original files preserved".into());
    }
    let binary =
        fs::canonicalize(&receipt.artifact.path).map_err(|_| "Installed executable missing")?;
    if !binary.starts_with(&root) {
        return Err("Receipt executable escapes install folder".into());
    }
    let mut artifact = inspect_runtime(
        &binary,
        Some(receipt.artifact.version),
        true,
        cancel,
        receipt.gpu_required,
    )?;
    artifact.details["archives"] = json!(receipt.archives);
    artifact.details["receipt_verified"] = json!(true);
    artifact.id = add(artifact.clone())?;
    Ok(Some(artifact))
}
async fn install_asset(
    app: &tauri::AppHandle,
    jobs: &crate::jobs::State,
    job_id: &str,
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    tag: &str,
    asset_id: u64,
    require_gpu: bool,
) -> Result<Value, String> {
    if tag.is_empty()
        || !tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return Err("Invalid release tag".into());
    }
    crate::jobs::check(cancel)?;
    let response = tokio::select! {_=crate::session::cancelled(cancel)=>return Err("Cancelled".into()),r=crate::hub::client()?
    .get(format!(
        "https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/{tag}"
    ))
    .send()
    =>r.map_err(|_| "Cannot verify official release")?};
    let release = tokio::select! {_=crate::session::cancelled(cancel)=>return Err("Cancelled".into()),r=crate::hub::decode(response)=>r?};
    let asset = release["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|a| a["id"].as_u64() == Some(asset_id)))
        .ok_or("Asset missing from official release")?;
    let name = asset["name"].as_str().ok_or("Missing asset name")?;
    let os = if cfg!(windows) { "win" } else { "ubuntu" };
    if !name.starts_with("llama-") || !name.contains(os) || !name.contains("x64") {
        return Err("Asset is not a server build for this platform".into());
    }
    let mut archives = vec![asset.clone()];
    if let Some((_, suffix)) = name.rsplit_once("-cuda-") {
        if let Some(dependency) = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n| n.starts_with("cudart-") && n.ends_with(suffix))
            })
        {
            archives.push(dependency.clone());
        }
    }
    let root = runtimes_dir();
    fs::create_dir_all(&root).map_err(|_| "Cannot create runtimes folder")?;
    let dest = root.join(format!("{tag}-{asset_id}"));
    if dest.exists() {
        let d = dest.clone();
        let c = cancel.clone();
        let artifact = tauri::async_runtime::spawn_blocking(move || reconcile(&d, &c))
            .await
            .map_err(|_| "Recovery worker failed")??;
        return artifact
            .map(|a| json!({"artifact":a,"already_installed":true}))
            .ok_or(
                "Unregistered install has no verified receipt; its files were preserved".into(),
            );
    }
    let total = archives
        .iter()
        .try_fold(0u64, |n, a| n.checked_add(a["size"].as_u64()?))
        .ok_or("Invalid archive sizes")?;
    crate::downloads::disk_space(&root, total.saturating_mul(4))?;
    let stage = root.join(format!(".{job_id}-{asset_id}.part"));
    fs::create_dir(&stage).map_err(|_| "Cannot create staging folder")?;
    let _stage = Staging(stage.clone());
    ACTIVE_STAGES
        .lock()
        .map_err(|_| "Stage state unavailable")?
        .insert(stage.clone());
    crate::storage::write_private_atomic(
        &stage.join("stage-owner.bin"),
        &serde_json::to_vec(&json!({"version":1,"tag":tag,"asset_id":asset_id}))
            .map_err(|_| "Cannot record staging ownership")?,
    )?;
    let folder = stage.join("files");
    fs::create_dir(&folder).map_err(|_| "Cannot extract runtime")?;
    let mut verified = Vec::new();
    for (index, archive) in archives.iter().enumerate() {
        crate::jobs::check(cancel)?;
        let file = stage.join(format!("archive-{index}"));
        let digest = download_archive(archive, &file, cancel, |done, total, name| {
            jobs.update(app, job_id, |r| {
                r.completed = done;
                r.total = total;
                r.message = name.into();
            })
        })
        .await?;
        let extracted = if index == 0 {
            folder.clone()
        } else {
            let d = stage.join(format!("dependencies-{index}"));
            fs::create_dir(&d).map_err(|_| "Cannot extract dependencies")?;
            d
        };
        let (f, d, c) = (file, extracted.clone(), cancel.clone());
        let zip = archive["name"]
            .as_str()
            .is_some_and(|n| n.ends_with(".zip"));
        tauri::async_runtime::spawn_blocking(move || extract(&f, &d, zip, &c))
            .await
            .map_err(|_| "Extraction worker failed")??;
        if index > 0 {
            let dest = find_binary(&folder)?
                .ok_or("Server archive has no executable")?
                .parent()
                .ok_or("Missing executable folder")?
                .to_path_buf();
            let c = cancel.clone();
            tauri::async_runtime::spawn_blocking(move || copy_dependencies(&extracted, &dest, &c))
                .await
                .map_err(|_| "Dependency worker failed")??;
        }
        verified.push(json!({"asset_id":archive["id"],"source":archive["browser_download_url"],"name":archive["name"],"sha256":digest,"publisher_hash_verified":archive["digest"].as_str().is_some_and(|d|d.starts_with("sha256:"))}));
    }
    let (f, d, c, t, archives) = (
        folder.clone(),
        dest.clone(),
        cancel.clone(),
        tag.to_owned(),
        verified.clone(),
    );
    let artifact = tauri::async_runtime::spawn_blocking(move || {
        let binary = find_binary(&f)?.ok_or("Archive has no llama-server")?;
        let mut artifact = inspect_runtime(&binary, Some(t), true, &c, require_gpu)?;
        let canonical_binary = fs::canonicalize(&binary).map_err(|_| "Invalid executable path")?;
        let relative = canonical_binary
            .strip_prefix(fs::canonicalize(&f).map_err(|_| "Invalid staging path")?)
            .map_err(|_| "Executable escaped staging")?;
        artifact.path = d.join(relative).to_string_lossy().into();
        artifact.details["archives"] = json!(archives.clone());
        artifact.details["receipt_verified"] = json!(true);
        let files = file_manifest(&f, &c)?;
        let receipt = InstallReceipt {
            version: 1,
            artifact: artifact.clone(),
            archives,
            files,
            gpu_required: require_gpu,
        };
        crate::storage::write_private_atomic(
            &f.join("install-receipt.bin"),
            &serde_json::to_vec(&receipt).map_err(|_| "Cannot encode install receipt")?,
        )?;
        crate::jobs::check(&c)?;
        crate::storage::publish_new(&f, &d)?;
        // A durable receipt precedes publication. Any registration failure is recoverable on retry.
        artifact.id = add(artifact.clone())?;
        Ok::<_, String>(artifact)
    })
    .await
    .map_err(|_| "Installation worker failed")??;
    Ok(json!({"artifact":artifact,"archives":verified}))
}
pub async fn install(
    app: tauri::AppHandle,
    jobs: crate::jobs::State,
    tag: String,
    asset_id: u64,
) -> Result<String, String> {
    let (id, cancel) = jobs.begin("runtime")?;
    let job_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let result = install_asset(&app, &jobs, &job_id, &cancel, &tag, asset_id, false).await;
        jobs.finish(&app, &job_id, result);
    });
    Ok(id)
}
pub fn install_channel(
    app: tauri::AppHandle,
    jobs: crate::jobs::State,
    channel: String,
) -> Result<String, String> {
    channel_name(&channel)?;
    let (id, cancel) = jobs.begin("runtime")?;
    let job_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let result=async{
            jobs.update(&app,&job_id,|r|r.message="Finding the official release and verifying backends…".into());
            let plan=tokio::select!{_=crate::session::cancelled(&cancel)=>return Err("Cancelled".into()),p=channel_plan(&channel)=>p?};
            let candidates=plan["candidates"].as_array().ok_or("Missing backend candidates")?;let mut failures=Vec::new();
            for candidate in candidates {
                crate::jobs::check(&cancel)?;let backend=candidate["backend"].as_str().ok_or("Missing backend")?;
                jobs.update(&app,&job_id,|r|{r.result["selection"]=plan.clone();r.result["selection"]["backend"]=json!(backend);r.message=format!("Verifying {backend} runtime…");});
                match install_asset(&app,&jobs,&job_id,&cancel,plan["tag"].as_str().ok_or("Missing tag")?,candidate["asset_id"].as_u64().ok_or("Missing asset")?,backend!="CPU").await {
                    Ok(mut value)=>{jobs.update(&app,&job_id,|r|{r.result["selection"]["reason"]=json!(if failures.is_empty(){format!("{backend} runtime passed executable and device validation")}else{format!("{backend} verified after {} failed candidate(s)",failures.len())});});value["fallbacks"]=json!(failures);return Ok(value);},Err(error)=>{if cancel.load(std::sync::atomic::Ordering::Relaxed){return Err("Cancelled".into());}failures.push(format!("{backend}: {error}"));}
                }
            }Err(format!("No backend passed validation: {}",failures.join("; ")))
        }.await;
        jobs.finish(&app, &job_id, result);
    });
    Ok(id)
}
/// Called while no installation jobs are running. Only marked, direct-child staging folders are removed.
pub fn recover_installs() -> Result<(), String> {
    let root = runtimes_dir();
    if !root.exists() {
        return Ok(());
    }
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for entry in fs::read_dir(&root).map_err(|_| "Cannot inspect managed runtime folders")? {
        let entry = entry.map_err(|_| "Cannot inspect managed runtime folder")?;
        if !entry
            .file_type()
            .map_err(|_| "Cannot inspect managed runtime folder")?
            .is_dir()
        {
            continue;
        }
        let path = entry.path();
        if ACTIVE_STAGES
            .lock()
            .map_err(|_| "Stage state unavailable")?
            .contains(&path)
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && name.ends_with(".part") {
            if crate::storage::load_optional::<Value>(&path.join("stage-owner.bin"), 4096)?
                .is_some_and(|v| v["version"] == 1 && v["asset_id"].is_u64())
            {
                let root = fs::canonicalize(&root).map_err(|_| "Invalid runtime root")?;
                let resolved = fs::canonicalize(&path).map_err(|_| "Invalid staging folder")?;
                if resolved.parent() == Some(root.as_path()) {
                    fs::remove_dir_all(resolved)
                        .map_err(|_| "Cannot clean interrupted staging folder")?;
                }
            }
        } else if path.join("install-receipt.bin").is_file() {
            let root = fs::canonicalize(&path).map_err(|_| "Invalid runtime folder")?;
            if !list()?
                .iter()
                .any(|a| a.managed && fs::canonicalize(&a.path).is_ok_and(|p| p.starts_with(&root)))
            {
                reconcile(&path, &cancel)?;
            }
        }
    }
    Ok(())
}

struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        if let Ok(mut stages) = ACTIVE_STAGES.lock() {
            stages.remove(&self.0);
        }
        let Ok(root) = fs::canonicalize(runtimes_dir()) else {
            return;
        };

        let Ok(stage) = fs::canonicalize(&self.0) else {
            return;
        };

        if stage.parent() == Some(root.as_path())
            && stage.file_name().is_some_and(|n| {
                n.to_string_lossy().starts_with('.') && n.to_string_lossy().ends_with(".part")
            })
        {
            let _ = fs::remove_dir_all(stage);
        }
    }
}

async fn download_archive(
    asset: &Value,
    path: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    mut progress: impl FnMut(u64, u64, &str),
) -> Result<String, String> {
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("Missing official release URL")?;

    if !url.starts_with("https://github.com/ggml-org/llama.cpp/releases/download/") {
        return Err("Untrusted release URL".into());
    }

    let total = asset["size"].as_u64().ok_or("Missing archive size")?;

    if total > 2 * 1024 * 1024 * 1024 {
        return Err("Runtime archive exceeds 2 GiB".into());
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| "Cannot create release client")?;

    let mut response = tokio::select! {
    _=crate::session::cancelled(cancel)=>return Err("Runtime install cancelled".into()),r=tokio::time::timeout(std::time::Duration::from_secs(45),client.get(url).send())=>r.map_err(|_|"Runtime connection timed out")?.map_err(|_|"Cannot download runtime")?};

    if !response.status().is_success() {
        return Err("Runtime download rejected".into());
    }

    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|_| "Cannot create runtime archive")?;

    let mut completed = 0u64;

    let mut hash = Sha256::new();

    let mut last = std::time::Instant::now();

    loop {
        let chunk = tokio::select! {
        _=crate::session::cancelled(cancel)=>return Err("Runtime install cancelled".into()),c=tokio::time::timeout(std::time::Duration::from_secs(60),response.chunk())=>c.map_err(|_|"Runtime download stalled")?.map_err(|_|"Runtime download interrupted")?};

        let Some(c) = chunk else { break };

        completed += c.len() as u64;

        if completed > total {
            return Err("Runtime exceeded published size".into());
        }

        use tokio::io::AsyncWriteExt;

        file.write_all(&c)
            .await
            .map_err(|_| "Cannot write runtime archive")?;

        hash.update(&c);

        if last.elapsed().as_millis() > 250 {
            progress(
                completed,
                total,
                asset["name"].as_str().unwrap_or("Runtime"),
            );

            last = std::time::Instant::now();
        }
    }

    use tokio::io::AsyncWriteExt;
    file.flush()
        .await
        .map_err(|_| "Cannot flush runtime archive")?;
    if completed != total {
        return Err("Runtime archive incomplete".into());
    }

    let digest = format!("{:x}", hash.finalize());

    if let Some(expected) = asset["digest"]
        .as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
    {
        if expected != digest {
            return Err("Runtime archive SHA256 mismatch".into());
        }
    }

    Ok(digest)
}

fn copy_dependencies(
    source: &Path,
    dest: &Path,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let mut pending = vec![source.to_owned()];

    let mut count = 0;

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            crate::jobs::check(cancel)?;

            count += 1;

            if count > 20000 {
                return Err("Dependency folder exceeds limits".into());
            }

            let e = entry.map_err(|e| e.to_string())?;

            let t = e.file_type().map_err(|e| e.to_string())?;

            if t.is_dir() {
                pending.push(e.path());
            } else if t.is_file() || t.is_symlink() {
                let resolved =
                    fs::canonicalize(e.path()).map_err(|_| "Missing dependency target")?;

                if !resolved.starts_with(fs::canonicalize(source).map_err(|e| e.to_string())?) {
                    return Err("Dependency link escaped archive".into());
                }

                let name = e.file_name().to_string_lossy().into_owned();

                let lower = name.to_ascii_lowercase();

                let out = if lower.ends_with(".dll") || lower.contains(".so") {
                    dest.join(&name)
                } else {
                    let notices = dest.join("dependency-notices");

                    fs::create_dir_all(&notices).map_err(|e| e.to_string())?;

                    notices.join(&name)
                };

                if !out.exists() {
                    fs::copy(e.path(), out).map_err(|_| "Cannot install runtime dependencies")?;
                }
            }
        }
    }

    Ok(())
}

fn extract(
    archive: &Path,
    dest: &Path,
    zip: bool,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let mut total = 0u64;

    let mut count = 0;

    if zip {
        let mut archive = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
            .map_err(|_| "Invalid ZIP")?;

        if archive.len() > 20000 {
            return Err("Archive has too many files".into());
        }

        for i in 0..archive.len() {
            crate::jobs::check(cancel)?;

            let mut file = archive.by_index(i).map_err(|_| "Invalid ZIP entry")?;

            if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
                return Err("Runtime archive links are not accepted".into());
            }

            let path = file.enclosed_name().ok_or("Archive contains unsafe path")?;

            let path = path.to_string_lossy().replace('\\', "/");

            if file.is_dir() {
                continue;
            }

            total = total
                .checked_add(file.size())
                .ok_or("Archive size overflow")?;

            if total > 4 * 1024 * 1024 * 1024 {
                return Err("Extracted runtime exceeds 4 GiB".into());
            }

            let out = crate::downloads::destination(dest, &path)?;

            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(out)
                .map_err(|_| "Cannot extract runtime")?;

            std::io::copy(&mut file, &mut output).map_err(|_| "Cannot extract runtime")?;
        }
    } else {
        let decoder =
            flate2::read::GzDecoder::new(fs::File::open(archive).map_err(|e| e.to_string())?);

        let mut tar = tar::Archive::new(decoder);

        let mut links = Vec::new();

        for entry in tar.entries().map_err(|_| "Invalid runtime archive")? {
            crate::jobs::check(cancel)?;

            let mut e = entry.map_err(|_| "Invalid tar entry")?;

            count += 1;

            if count > 20000 {
                return Err("Too many archive entries".into());
            }

            if e.header().entry_type().is_dir() {
                continue;
            }

            let entry_type = e.header().entry_type();

            if entry_type.is_symlink() || entry_type.is_hard_link() {
                let name = e
                    .path()
                    .map_err(|_| "Invalid link path")?
                    .to_string_lossy()
                    .trim_start_matches("./")
                    .to_owned();

                let out = crate::downloads::destination(dest, &name)?;

                let target = e
                    .link_name()
                    .map_err(|_| "Invalid archive link")?
                    .ok_or("Missing archive link target")?
                    .to_path_buf();

                if target.is_absolute() {
                    return Err("Absolute archive links are not accepted".into());
                }

                links.push((out, target, entry_type.is_hard_link()));

                continue;
            }

            if !entry_type.is_file() {
                return Err("Unsupported runtime archive entry".into());
            }

            total = total.checked_add(e.size()).ok_or("Archive size overflow")?;

            if total > 4 * 1024 * 1024 * 1024 {
                return Err("Extracted runtime exceeds 4 GiB".into());
            }

            let name = e
                .path()
                .map_err(|_| "Invalid tar path")?
                .to_string_lossy()
                .trim_start_matches("./")
                .to_owned();

            let out = crate::downloads::destination(dest, &name)?;

            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out)
                .map_err(|_| "Cannot extract runtime")?;

            std::io::copy(&mut e, &mut f).map_err(|_| "Cannot extract runtime")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(
                    out,
                    fs::Permissions::from_mode(e.header().mode().unwrap_or(0o644) & 0o777),
                )
                .map_err(|_| "Cannot set runtime permissions")?;
            }
        }

        finish_links(dest, links)?;
    }

    Ok(())
}

fn finish_links(dest: &Path, links: Vec<(PathBuf, PathBuf, bool)>) -> Result<(), String> {
    if links.is_empty() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        let _ = (dest, links);

        Err("Unix archive links cannot be installed on Windows".into())
    }

    #[cfg(unix)]
    {
        let mut links = links;

        let root = fs::canonicalize(dest).map_err(|e| e.to_string())?;

        while !links.is_empty() {
            let before = links.len();

            let mut pending = Vec::new();

            for (out, target, hard) in links {
                let candidate = if hard {
                    dest.join(&target)
                } else {
                    out.parent().ok_or("Missing link parent")?.join(&target)
                };

                let resolved = match fs::canonicalize(&candidate) {
                    Ok(p) => p,
                    Err(_) => {
                        pending.push((out, target, hard));

                        continue;
                    }
                };

                if !resolved.starts_with(&root) {
                    return Err("Runtime archive link escapes its directory".into());
                }

                if hard {
                    fs::hard_link(&resolved, &out)
                        .map_err(|_| "Cannot create runtime hard link")?;
                } else {
                    std::os::unix::fs::symlink(target, out)
                        .map_err(|_| "Cannot create runtime symlink")?;
                }
            }

            if pending.len() == before {
                return Err("Runtime archive has missing or circular links".into());
            }

            links = pending;
        }

        Ok(())
    }
}

pub fn find_binary(dir: &Path) -> Result<Option<PathBuf>, String> {
    let mut pending = vec![dir.to_owned()];

    let mut visited = 0;

    while let Some(d) = pending.pop() {
        for entry in fs::read_dir(d).map_err(|e| e.to_string())? {
            visited += 1;

            if visited > 20000 {
                return Err("Runtime folder exceeds limits".into());
            }

            let e = entry.map_err(|e| e.to_string())?;

            let t = e.file_type().map_err(|e| e.to_string())?;

            if t.is_dir() {
                pending.push(e.path());
            } else if t.is_file()
                && e.file_name()
                    == if cfg!(windows) {
                        "llama-server.exe"
                    } else {
                        "llama-server"
                    }
            {
                return Ok(Some(e.path()));
            }
        }
    }

    Ok(None)
}

pub fn remove(id: &str) -> Result<(), String> {
    let _lock = LOCK.lock().map_err(|e| e.to_string())?;

    let profiles = crate::profiles::read(&crate::profiles::path())?;

    let mut rows = list()?;

    let a = rows
        .iter()
        .find(|a| a.id == id && a.kind == "runtime")
        .ok_or("Runtime not found")?;

    if profiles
        .profiles
        .iter()
        .any(|p| p.config.runtime_id == id || p.config.llama_cpp_path == a.path)
    {
        return Err("Relink profiles using this runtime before removing it".into());
    }

    if a.managed {
        let root = fs::canonicalize(runtimes_dir()).map_err(|_| "Missing managed runtime root")?;

        let executable = fs::canonicalize(&a.path).map_err(|_| "Managed runtime missing")?;

        let relative = executable
            .strip_prefix(&root)
            .map_err(|_| "Runtime is outside managed directory")?;

        let first = relative.components().next().ok_or("Invalid managed path")?;

        let folder = root.join(first);

        let resolved = fs::canonicalize(&folder).map_err(|e| e.to_string())?;

        if resolved == root || !resolved.starts_with(&root) {
            return Err("Unsafe runtime removal path".into());
        }

        fs::remove_dir_all(&resolved).map_err(|_| "Cannot remove runtime; it may be running")?;
    }

    rows.retain(|a| a.id != id);

    crate::storage::write_private_atomic(
        &path(),
        &serde_json::to_vec(&rows).map_err(|e| e.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn device_probe_requires_an_actual_gpu_row() {
        assert!(!super::gpu_devices(
            "Available devices:\nCPU0: test\nRPC0: remote"
        ));
        assert!(!super::gpu_devices("compiled with CUDA support"));
        for backend in [
            "CUDA0: device",
            "Vulkan1: device",
            "SYCL0: device",
            "Metal0: device",
        ] {
            assert!(super::gpu_devices(backend));
        }
    }
    #[test]
    fn receipt_recovery_rejects_changed_and_unexpected_files_before_execution() {
        let root = std::env::temp_dir().join(format!("receipt-fixture-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let binary = root.join("llama-server.exe");
        fs::write(&binary, b"fixture executable bytes").unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let receipt = super::InstallReceipt {
            version: 1,
            artifact: super::Artifact {
                id: "fixture".into(),
                kind: "runtime".into(),
                name: "fixture".into(),
                path: binary.to_string_lossy().into(),
                version: "fixture".into(),
                managed: true,
                details: serde_json::json!({}),
            },
            archives: Vec::new(),
            files: super::file_manifest(&root, &cancel).unwrap(),
            gpu_required: false,
        };
        crate::storage::write_private_atomic(
            &root.join("install-receipt.bin"),
            &serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        fs::write(root.join("unexpected.dll"), b"extra").unwrap();
        assert!(super::reconcile(&root, &cancel)
            .err()
            .unwrap()
            .contains("file set changed"));
        fs::remove_file(root.join("unexpected.dll")).unwrap();
        fs::write(&binary, b"changed").unwrap();
        assert!(super::reconcile(&root, &cancel)
            .err()
            .unwrap()
            .contains("receipt verification"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn automatic_install_selects_the_platform_and_never_substitutes_beta_for_stable() {
        let release = serde_json::json!({"assets":[
            {"name":"llama-b12-bin-win-cpu-x64.zip","id":1},
            {"name":"llama-b12-bin-win-vulkan-x64.zip","id":2},
            {"name":"cudart-llama-bin-win-cuda-x64.zip","id":3},
            {"name":"llama-b12-bin-win-cpu-arm64.zip","id":4},
            {"name":"llama-b12-bin-ubuntu-x64.tar.gz","id":5}
        ]});
        assert_eq!(
            super::choose_asset(&release, "win", false).unwrap().0["id"],
            1
        );
        assert_eq!(
            super::choose_asset(&release, "win", true).unwrap().0["id"],
            2
        );
        assert_eq!(
            super::choose_asset(&release, "ubuntu", true).unwrap().0["id"],
            5
        );
        assert_eq!(super::stable_binary_tag("b11429\n").unwrap(), "b11429");
        assert!(super::stable_binary_tag("../../beta").is_err());
        assert!(super::channel_name("latest").is_err());
        assert!(super::choose_asset(&serde_json::json!({"assets":[]}), "win", false).is_err());
    }

    use super::*;

    #[test]
    fn archive_traversal_is_rejected_and_valid_runtime_is_found() {
        use std::io::Write;

        let root =
            std::env::temp_dir().join(format!("runtime-archive-test-{}", std::process::id()));

        fs::create_dir_all(&root).unwrap();

        let dir = root.join("files");

        fs::create_dir_all(&dir).unwrap();

        let archive = root.join("archive.zip");

        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());

        zip.start_file("../escaped", zip::write::SimpleFileOptions::default())
            .unwrap();

        zip.write_all(b"bad").unwrap();

        zip.finish().unwrap();

        assert!(extract(
            &archive,
            &dir,
            true,
            &std::sync::atomic::AtomicBool::new(false)
        )
        .is_err());

        assert!(!root.join("escaped").exists());

        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());

        let name = if cfg!(windows) {
            "bin/llama-server.exe"
        } else {
            "bin/llama-server"
        };

        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();

        zip.write_all(b"fixture executable bytes").unwrap();

        zip.finish().unwrap();

        extract(
            &archive,
            &dir,
            true,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();

        assert!(find_binary(&dir).unwrap().is_some());

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_preserve_internal_libraries_but_reject_escape() {
        let root = std::env::temp_dir().join(format!("runtime-links-test-{}", std::process::id()));

        fs::create_dir_all(&root).unwrap();

        fs::write(root.join("lib.so.1"), b"fixture").unwrap();

        finish_links(&root, vec![(root.join("lib.so"), "lib.so.1".into(), false)]).unwrap();

        assert_eq!(fs::read(root.join("lib.so")).unwrap(), b"fixture");

        assert!(finish_links(&root, vec![(root.join("escape"), "../".into(), false)]).is_err());

        fs::remove_dir_all(root).unwrap();
    }
}
