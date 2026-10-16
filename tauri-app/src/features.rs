//! Native commands for hardware, model hub and managed jobs.
use crate::{config::Config, downloads, hub, jobs, monitor, registry, server, session};
use tauri::Emitter;

use serde_json::{json, Value};

use tauri::{Manager, State};

#[tauri::command]
pub async fn working_load() -> Result<Option<Config>, String> {
    tauri::async_runtime::spawn_blocking(working_load_inner)
        .await
        .map_err(|_| "Settings reader failed".to_string())?
}
fn working_load_inner() -> Result<Option<Config>, String> {
    let path = crate::storage::profile_directory().join("working-settings.bin");
    if !path.exists() {
        return Ok(None);
    }
    let config: Config =
        crate::storage::load_optional(&path, 1024 * 1024)?.ok_or("Working settings disappeared")?;
    if config.schema_version > 3 {
        return Err(crate::storage::damaged(
            &path,
            "Working settings need a newer launcher",
        ));
    }

    Ok(Some(config))
}
#[tauri::command]
pub async fn working_save(config: Config) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || working_save_inner(config))
        .await
        .map_err(|_| "Settings writer failed".to_string())?
}
fn working_save_inner(config: Config) -> Result<(), String> {
    if config.schema_version > 3 {
        return Err("Unsupported settings version".into());
    }
    let bytes = serde_json::to_vec(&config).map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("Working settings exceed 1 MiB".into());
    }
    crate::storage::write_private_atomic(
        &crate::storage::profile_directory().join("working-settings.bin"),
        &bytes,
    )
}
#[tauri::command]
pub fn storage_location() -> String {
    crate::storage::profile_directory().to_string_lossy().into()
}
#[tauri::command]
pub async fn storage_issues() -> Result<Vec<Value>, String> {
    tauri::async_runtime::spawn_blocking(crate::storage::issues)
        .await
        .map_err(|_| "Saved-record inspection failed".into())
}
#[tauri::command]
pub async fn storage_recover(
    app: tauri::AppHandle,
    state: State<'_, server::State>,
    jobs: State<'_, jobs::State>,
    name: String,
    action: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    let jobs = jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let manager = state.0.lock().map_err(|_| "Server state unavailable")?;
        if manager.instance.is_some()
            || manager.benchmark
            || jobs
                .0
                .lock()
                .map_err(|_| "Job state unavailable")?
                .iter()
                .any(|j| j.report.phase == "running")
        {
            return Err(
                "Stop the server and finish/cancel jobs before recovering saved records".into(),
            );
        }
        crate::storage::recover(&name, &action)?;
        let client = app
            .state::<crate::mcp::Manager>()
            .connection
            .lock()
            .map_err(|_| "Connection state unavailable")?
            .client
            .take();
        drop(client);
        app.restart()
    })
    .await
    .map_err(|_| "Recovery worker failed".to_string())?
}
#[tauri::command]
pub fn close_ready(state: State<'_, crate::CloseGuard>) {
    state.ready.store(true, std::sync::atomic::Ordering::SeqCst);
}
#[tauri::command]
pub fn finish_close(
    window: tauri::WebviewWindow,
    state: State<'_, crate::CloseGuard>,
) -> Result<(), String> {
    state.allow.store(true, std::sync::atomic::Ordering::SeqCst);
    window.close().map_err(|_| "Cannot close launcher".into())
}

#[tauri::command]
pub fn monitoring_get(
    state: State<'_, monitor::State>,
) -> Result<monitor::MonitoringPreferences, String> {
    Ok(state
        .inner
        .lock()
        .map_err(|e| e.to_string())?
        .preferences
        .clone())
}

#[tauri::command]
pub async fn monitoring_set(
    state: State<'_, monitor::State>,
    preferences: monitor::MonitoringPreferences,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || monitor::set(&state, preferences))
        .await
        .map_err(|_| "Monitoring writer failed".to_string())?
}

#[tauri::command]
pub fn monitoring_history(
    state: State<'_, monitor::State>,
) -> Result<Vec<monitor::TelemetrySample>, String> {
    Ok(state
        .inner
        .lock()
        .map_err(|e| e.to_string())?
        .history
        .clone())
}

#[tauri::command]
pub async fn hardware_snapshot() -> Result<monitor::HardwareSnapshot, String> {
    let mut h = tauri::async_runtime::spawn_blocking(monitor::hardware)
        .await
        .map_err(|e| e.to_string())?;

    h.gpus = monitor::gpus().await;

    if h.gpus.is_empty() {
        h.gpus = monitor::adapters().await;
    }

    Ok(h)
}

#[tauri::command]
pub async fn model_inspect(
    path: String,
    context: u32,
    parallel: u32,
) -> Result<crate::gguf::ModelMetadata, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::gguf::inspect(
            &crate::checked_path(&path)?,
            context as u64,
            parallel as u64,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn model_inspect_config(config: Config) -> Result<crate::gguf::ModelMetadata, String> {
    tauri::async_runtime::spawn_blocking(move || crate::gguf::inspect_config(&config))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn launch_preflight(config: Config) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move||{

        let (binary,model,hosts)=crate::validate(&config)?;

        for host in hosts{
drop(std::net::TcpListener::bind((host,config.port)).map_err(|_|"Selected port is in use")?);
}

        crate::validate_capabilities(&config,&crate::runtime_help(&config.llama_cpp_path,false)?)?;
        let version=registry::probe(&binary,"--version")?;

        let devices=registry::probe(&binary,"--list-devices").unwrap_or("Device discovery unavailable in this runtime".into());

        let hardware=monitor::hardware();
let mut warnings=Vec::new();

        let metadata=if let Some(path)=model{

            let mut inspected=config.clone();
inspected.model_file=path.to_string_lossy().into();
let m=crate::gguf::inspect_config(&inspected)?;

            if matches!(m.architecture.as_str(),"flux"|"stable-diffusion"|"sd1"|"sd2"|"sdxl"|"clip"){
return Err("Selected file is a diffusion model or companion, not a standalone text model".into());
}

            warnings.extend(m.warnings.clone());

            let ram=m.resource_estimate["ram_low"].as_u64().unwrap_or(m.estimate_low);

            if ram>hardware.available_ram{
warnings.push("Estimated RAM demand exceeds currently available memory. Review context and offload settings.".into());
}
Some(m)
        }
else{
None}
;

        if session::option(&config,&["--gpu-layers","--n-gpu-layers","-ngl"]).is_some_and(|s|s!="0")&&!devices.contains("unavailable")&&!["CUDA","Vulkan","SYCL","HIP","ROCm","Metal"].iter().any(|name|devices.contains(name)){
warnings.push("GPU offload requested, but this runtime reported no recognized GPU backend.".into());
}

        Ok(json!({
"version":version,"devices":devices,"hardware":hardware,"model":metadata,"warnings":warnings,"suggestions":[{"flag":"--threads","values":[sysinfo::System::physical_core_count().unwrap_or(hardware.logical_cores as usize).max(1).to_string()],"reason":"Start with physical CPU cores; more threads can hurt throughput when memory bandwidth or scheduling becomes the limit. Benchmark before choosing a final value."}]}
))
    }
).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn server_models(state: State<'_, server::State>) -> Result<Value, String> {
    session::json_request(&session::current(&state)?, "GET", "/models", None).await
}

#[tauri::command]
pub async fn router_model(
    state: State<'_, server::State>,
    model: String,
    load: bool,
) -> Result<Value, String> {
    let s = session::current(&state)?;

    if !s.router {
        return Err("Start in router mode first".into());
    }

    if model.is_empty() || model.len() > 1024 {
        return Err("Select a router model ID".into());
    }

    session::json_request(
        &s,
        "POST",
        if load {
            "/models/load"
        } else {
            "/models/unload"
        },
        Some(json!({
        "model":model}
        )),
    )
    .await
}

#[tauri::command]
pub fn open_server_chat(state: State<'_, server::State>) -> Result<(), String> {
    let s = session::current(&state)?;

    if s.endpoint.starts_with("unix:") {
        return Err(
            "Browsers cannot open Unix sockets; use a TCP listen address for browser chat.".into(),
        );
    }

    open_browser(&format!("{}{}/", s.endpoint, s.prefix))
}

#[tauri::command]
pub fn hub_open_tokens() -> Result<(), String> {
    open_browser("https://huggingface.co/settings/tokens")
}

#[tauri::command]
pub fn open_hub_page(repo: String) -> Result<(), String> {
    let parts = hub::segments(&repo)?;

    if parts.len() != 2 {
        return Err("Invalid repository".into());
    }

    open_browser(&format!("https://huggingface.co{}", hub::url_path(&parts)?))
}

#[tauri::command]
pub fn jobs_list(state: State<'_, jobs::State>) -> Result<Vec<jobs::JobReport>, String> {
    Ok(state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|j| j.report.clone())
        .collect())
}

#[tauri::command]
pub fn job_cancel(
    app: tauri::AppHandle,
    state: State<'_, jobs::State>,
    id: String,
) -> Result<(), String> {
    let sign_in = state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|j| j.report.id == id && j.report.kind == "sign-in");

    if sign_in {
        app.state::<hub::State>()
            .1
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    state.cancel(&id)?;

    state.update(&app, &id, |r| {
        r.message = "Cancellation requested; waiting for current work to stop.".into()
    });
    Ok(())
}

#[tauri::command]
pub fn inference_test(
    app: tauri::AppHandle,
    state: State<'_, server::State>,
    jobs: State<'_, jobs::State>,
    prompt: String,
    model: String,
    max_tokens: u32,
) -> Result<String, String> {
    if prompt.is_empty() || prompt.len() > 65536 || !(1..=4096).contains(&max_tokens) {
        return Err("Enter a prompt up to 64 KiB and 1–4096 output tokens".into());
    }

    let s = session::current(&state)?;

    let (id, cancel) = jobs.begin("inference")?;

    let jobs = jobs.inner().clone();

    let job_id = id.clone();

    tauri::async_runtime::spawn(async move {
        let result = session::completion(&s, &prompt, &model, max_tokens, &cancel, |text| {
            let _ = app.emit(
                "inference-delta",
                json!({
                "id":job_id,"text":text}
                ),
            );
        })
        .await;

        jobs.finish(&app, &job_id, result);
    });

    Ok(id)
}

#[tauri::command]
pub fn hub_account(state: State<'_, hub::State>) -> Result<Value, String> {
    let a = state.0.lock().map_err(|e| e.to_string())?;

    Ok(json!({
    "connected":!a.token.is_empty(),"name":a.name,"client_id":a.client_id}
    ))
}

#[tauri::command]
pub async fn hub_connect(state: State<'_, hub::State>, token: String) -> Result<Value, String> {
    hub::connect(&state, token).await
}

#[tauri::command]
pub async fn hub_test(state: State<'_, hub::State>) -> Result<Value, String> {
    let token = state.token();

    hub::connect(&state, token).await
}

#[tauri::command]
pub fn hub_disconnect(
    app: tauri::AppHandle,
    state: State<'_, hub::State>,
    jobs: State<'_, jobs::State>,
) -> Result<(), String> {
    let ids: Vec<_> = jobs
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .filter(|j| j.report.kind == "sign-in" && j.report.phase == "running")
        .map(|j| j.report.id.clone())
        .collect();
    for id in ids {
        jobs.cancel(&id)?;
        jobs.update(&app, &id, |r| {
            r.message = "Sign-in cancelled by disconnect.".into()
        });
    }
    let client_id = state.0.lock().map_err(|e| e.to_string())?.client_id.clone();

    state.save(hub::Account {
        client_id,
        ..Default::default()
    })
}

#[tauri::command]
pub async fn hub_search(
    state: State<'_, hub::State>,
    search: String,
    cursor: String,
) -> Result<Value, String> {
    hub::search(&state, &search, &cursor).await
}

#[tauri::command]
pub async fn hub_read_card(
    state: State<'_, hub::State>,
    repo: String,
    revision: String,
) -> Result<String, String> {
    hub::read_card(&state, &repo, &revision).await
}

#[tauri::command]
pub async fn hub_details(state: State<'_, hub::State>, repo: String) -> Result<Value, String> {
    hub::details(&state, &repo).await
}

#[tauri::command]
pub async fn hub_sign_in(
    app: tauri::AppHandle,
    state: State<'_, hub::State>,
    jobs: State<'_, jobs::State>,
    client_id: String,
) -> Result<String, String> {
    hub::oauth(app, state.inner().clone(), jobs.inner().clone(), client_id).await
}

#[tauri::command]
pub fn hub_open_sign_in(state: State<'_, jobs::State>, id: String) -> Result<(), String> {
    let jobs = state.0.lock().map_err(|e| e.to_string())?;

    let job = jobs
        .iter()
        .find(|j| j.report.id == id && j.report.kind == "sign-in")
        .ok_or("No sign-in request")?;

    let url = job.report.result["url"]
        .as_str()
        .ok_or("Missing sign-in URL")?;

    open_browser(url)
}

#[tauri::command]
pub async fn download_start(
    app: tauri::AppHandle,
    state: State<'_, downloads::State>,
    hub: State<'_, hub::State>,
    jobs: State<'_, jobs::State>,
    repo: String,
    root: String,
    files: Vec<String>,
) -> Result<String, String> {
    let p = downloads::plan(&hub, repo, root, files).await?;

    downloads::start(
        app,
        state.inner().clone(),
        hub.inner().clone(),
        jobs.inner().clone(),
        p,
    )
}

#[tauri::command]
pub fn downloads_list(
    state: State<'_, downloads::State>,
) -> Result<Vec<downloads::DownloadPlan>, String> {
    Ok(state.plans.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub fn download_resume(
    app: tauri::AppHandle,
    state: State<'_, downloads::State>,
    hub: State<'_, hub::State>,
    jobs: State<'_, jobs::State>,
    id: String,
) -> Result<String, String> {
    let p = state
        .plans
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or("Download not found")?;

    downloads::reserve(&state, &p)?;

    let (new_id, cancel) = match jobs.begin("download") {
        Ok(v) => v,
        Err(e) => {
            state
                .active
                .lock()
                .map_err(|e| e.to_string())?
                .remove(&downloads::group(&p));

            return Err(e);
        }
    };

    let mut p = p;

    p.id = new_id.clone();

    {
        let mut plans = state.plans.lock().map_err(|e| e.to_string())?;

        plans.retain(|p| p.id != id);

        plans.push(p.clone());
    }

    if let Err(e) = state.save() {
        state
            .active
            .lock()
            .map_err(|e| e.to_string())?
            .remove(&downloads::group(&p));

        jobs.finish(&app, &new_id, Err(e.clone()));

        return Err(e);
    }

    downloads::launch(
        app,
        state.inner().clone(),
        hub.inner().clone(),
        jobs.inner().clone(),
        p,
        cancel,
    );

    Ok(new_id)
}

#[tauri::command]
pub fn artifacts_list() -> Result<Vec<registry::Artifact>, String> {
    registry::list()
}

#[tauri::command]
pub async fn runtime_import(binary: String) -> Result<registry::Artifact, String> {
    tauri::async_runtime::spawn_blocking(move || {
        registry::import(&crate::checked_path(&binary)?, false, None)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn runtime_channel_plan(channel: String) -> Result<Value, String> {
    registry::channel_plan(&channel).await
}
#[tauri::command]
pub async fn runtime_install_channel(
    app: tauri::AppHandle,
    jobs: State<'_, jobs::State>,
    channel: String,
) -> Result<String, String> {
    registry::install_channel(app, jobs.inner().clone(), channel)
}

#[tauri::command]
pub async fn runtime_releases() -> Result<Value, String> {
    registry::releases().await
}

#[tauri::command]
pub async fn runtime_install(
    app: tauri::AppHandle,
    jobs: State<'_, jobs::State>,
    tag: String,
    asset_id: u64,
) -> Result<String, String> {
    registry::install(app, jobs.inner().clone(), tag, asset_id).await
}

#[tauri::command]
pub async fn runtime_remove(
    app: tauri::AppHandle,
    state: State<'_, server::State>,
    jobs: State<'_, jobs::State>,
    id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    let jobs = jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Keep startup serialized with deletion through the actual file removal.
        let manager = state.0.lock().map_err(|e| e.to_string())?;
        if manager.instance.is_some() || manager.benchmark {
            return Err(
                "Stop the server and finish/cancel benchmarks before removing runtimes".into(),
            );
        }
        let active_jobs = jobs.0.lock().map_err(|e| e.to_string())?;
        if active_jobs.iter().any(|j| {
            j.report.phase == "running"
                && matches!(j.report.kind.as_str(), "runtime" | "runtime-recovery")
        }) {
            return Err("Wait for runtime installation and recovery before removal".into());
        }
        let profiles = app.state::<crate::profiles::Manager>();
        let _profiles = profiles.0.lock().map_err(|e| e.to_string())?;
        registry::remove(&id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn results_list() -> Result<Vec<Value>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dir = crate::storage::profile_directory().join("benchmarks");

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut paths = std::fs::read_dir(dir)
            .map_err(|_| "Cannot read benchmark history")?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "bin"))
            .collect::<Vec<_>>();

        paths.sort();

        paths.reverse();

        paths
            .into_iter()
            .take(32)
            .map(|p| {
                serde_json::from_slice(&crate::storage::read_private(&p, 4 * 1024 * 1024)?)
                    .map_err(|_| "Invalid saved benchmark".into())
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_remove(
    state: State<'_, downloads::State>,
    jobs: State<'_, jobs::State>,
    id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    let jobs = jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || download_remove_inner(&state, &jobs, &id))
        .await
        .map_err(|_| "Download removal worker failed".to_string())?
}
fn download_remove_inner(
    state: &downloads::State,
    jobs: &jobs::State,
    id: &str,
) -> Result<(), String> {
    if jobs
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|j| j.report.id == id && j.report.phase == "running")
    {
        return Err("Pause the download before removing partial files".into());
    }

    let mut plans = state.plans.lock().map_err(|e| e.to_string())?;

    let plan = plans
        .iter()
        .find(|p| p.id == id)
        .ok_or("Download not found")?;

    if state
        .active
        .lock()
        .map_err(|e| e.to_string())?
        .contains(&downloads::group(plan))
    {
        return Err(
            "Wait for this repository download to stop before removing partial files".into(),
        );
    }

    let dir = downloads::directory(plan)?;

    for item in &plan.files {
        let dest = downloads::destination(&dir, &item.name)?;

        let partial = dest.with_file_name(format!(
            "{}.part",
            dest.file_name()
                .ok_or("Missing file name")?
                .to_string_lossy()
        ));

        let manifest = partial.with_extension("resume.bin");

        for path in [&partial, &manifest] {
            if path.exists() {
                let resolved = std::fs::canonicalize(path).map_err(|e| e.to_string())?;

                if !resolved.starts_with(&dir)
                    || std::fs::symlink_metadata(path)
                        .map_err(|e| e.to_string())?
                        .file_type()
                        .is_symlink()
                {
                    return Err("Unsafe partial-file removal path".into());
                }

                std::fs::remove_file(path).map_err(|_| "Cannot remove partial file")?;
            }
        }
    }

    plans.retain(|p| p.id != id);

    drop(plans);

    state.save()
}

#[tauri::command]
pub async fn benchmark_start(
    app: tauri::AppHandle,
    state: State<'_, server::State>,
    jobs: State<'_, jobs::State>,
    profiles: Vec<String>,
) -> Result<String, String> {
    if profiles.is_empty() || profiles.len() > 16 {
        return Err("Choose 1–16 profiles to compare".into());
    }

    let library = crate::profiles::read(&crate::profiles::path())?;

    let rows = profiles
        .iter()
        .map(|id| library.get(id))
        .collect::<Result<Vec<_>, _>>()?;

    {
        let mut m = state.0.lock().map_err(|e| e.to_string())?;

        if m.instance.is_some() || m.benchmark {
            return Err("Stop the active server and wait for existing benchmarks first".into());
        }

        for row in &rows {
            if row.config.model_file.is_empty() {
                return Err("Benchmark profiles need a selected local model".into());
            }

            let mut local_config = row.config.clone();

            local_config.router_mode = false;

            if crate::uses_alternate_source(&local_config) {
                return Err(
                    "Download the model locally before benchmarking remote-source profiles".into(),
                );
            }
        }

        m.benchmark = true;
    }

    let active = state.inner().clone();

    let jobs = jobs.inner().clone();

    let (id, cancel) = match jobs.begin("benchmark") {
        Ok(v) => v,
        Err(e) => {
            if let Ok(mut m) = active.0.lock() {
                m.benchmark = false;
            }

            return Err(e);
        }
    };

    let job_id = id.clone();

    tauri::async_runtime::spawn(async move {
        struct Reservation(crate::server::State);

        impl Drop for Reservation {
            fn drop(&mut self) {
                if let Ok(mut m) = self.0 .0.lock() {
                    m.benchmark = false;

                    m.benchmark_server = None;
                }
            }
        }

        let _reservation = Reservation(active.clone());

        let result=async{
let hardware=monitor::hardware();
let mut results=Vec::new();
for (index,row) in rows.iter().enumerate(){
crate::jobs::check(&cancel)?;
jobs.update(&app,&job_id,|r|{
r.completed=index as u64;
r.total=rows.len() as u64;
r.message=format!("Benchmarking {}",row.name);
}
);

            let mut config=row.config.clone();
config.router_mode=false;
config.mcp_profile=Default::default();
config.host="127.0.0.1".into();
let listener=std::net::TcpListener::bind("127.0.0.1:0").map_err(|_|"Cannot reserve benchmark port")?;
config.port=listener.local_addr().map_err(|e|e.to_string())?.port();
drop(listener);

            let unsafe_flags=["--agent","--tools","--mcp-servers-json","--mcp-servers-config","--api-key","--api-key-file","--ssl-key-file","--ssl-cert-file","--api-prefix","--hf-token","-hft","--models-dir","--models-preset","--models-max","--host","--port","--webui"];

            config.argument_overrides.retain(|a|!unsafe_flags.contains(&a.flag.as_str())&&!a.flag.starts_with("--mcp-"));

            // Custom arguments can hide aliases, values and tool enabling flags; require structured overrides for a reproducible benchmark.
            if !config.additional_args.is_empty(){
return Err(format!("{} has custom arguments. Move them into structured overrides before benchmarking.",row.name));
}

            let effective=crate::arguments(&config,Some(&crate::checked_path(&if std::path::Path::new(&config.model_file).is_absolute(){
config.model_file.clone()}
else{
std::path::Path::new(&config.models_path).join(&config.model_file).to_string_lossy().into()}
)?));
let hash=format!("{:x}",<sha2::Sha256 as sha2::Digest>::digest(serde_json::to_vec(&effective).map_err(|e|e.to_string())?));

            let temp=server::State::default();
active.0.lock().map_err(|e|e.to_string())?.benchmark_server=Some(temp.clone());
temp.0.lock().map_err(|e|e.to_string())?.isolated=true;

            struct Owned(server::State);
impl Drop for Owned{
fn drop(&mut self){
let _=server::stop(&self.0);
}
}
let _owned=Owned(temp.clone());
let a=app.clone();
let s=temp.clone();
let started=std::time::Instant::now();
tauri::async_runtime::spawn_blocking(move||crate::start_server_inner(a,&s,config)).await.map_err(|e|e.to_string())??;

            let session=session::current(&temp)?;
let sampler=monitor::State::default();
let mut peak=0u64;
let deadline=std::time::Instant::now()+std::time::Duration::from_secs(180);
loop{
crate::jobs::check(&cancel)?;
if std::time::Instant::now()>deadline{
return Err(format!("{} did not become ready",row.name));
}

let s=sampler.clone();let pid=session.pid;
if let Ok(Ok(sample))=tauri::async_runtime::spawn_blocking(move||monitor::sample(&s,Some(pid))).await {peak=peak.max(sample.server_ram.unwrap_or(0));}
if session::json_request(&session,"GET","/health",None).await.is_ok(){
break;
}
tokio::time::sleep(std::time::Duration::from_millis(250)).await;
}

            let startup=started.elapsed().as_secs_f64();
let mut runs=Vec::new();
for run in 0..4{
let future=session::completion(&session,"Explain how a computer sorts a list of numbers. Give a clear step-by-step explanation.","",256,&cancel,|_|{
}
);
tokio::pin!(future);
let result=loop{
tokio::select!{
r=&mut future=>break r?,_=tokio::time::sleep(std::time::Duration::from_millis(250))=>{
let s=sampler.clone();
let pid=session.pid;
if let Ok(Ok(sample))=tauri::async_runtime::spawn_blocking(move||monitor::sample(&s,Some(pid))).await{
peak=peak.max(sample.server_ram.unwrap_or(0));
}
}
}
}
;
if run>0{
let mut result=result;
if let Some(object)=result.as_object_mut(){
object.remove("text");
}
runs.push(result);
}
}

            let binary=row.config.llama_cpp_path.clone();
            let version=tauri::async_runtime::spawn_blocking(move||registry::probe(std::path::Path::new(&binary),"--version")).await.map_err(|e|e.to_string())??;
results.push(json!({
"profile":row.name,"profile_id":row.id,"startup_seconds":startup,"peak_server_ram":peak,"memory_measurement":"Sampled process-tree peak including startup; 250 ms intervals; shorter peaks may be missed","runtime_version":version,"model":row.config.model_file,"model_id":row.config.model_id,"effective_args":effective,"config_hash":hash,"isolation":"Single local model; MCP/tools, inherited LLAMA_ARG variables, API credentials and TLS disabled; loopback temporary port","runs":runs}
));
}

            let result=json!({
"timestamp":monitor::now(),"hardware":hardware,"results":results}
);
let path=crate::storage::profile_directory().join("benchmarks").join(format!("{job_id}.bin"));
let saved=result.clone();
tauri::async_runtime::spawn_blocking(move||{crate::storage::write_private_atomic(&path,&serde_json::to_vec(&saved).map_err(|e|e.to_string())?)?;retain_benchmarks(path.parent().ok_or("Missing history folder")?,32)}).await.map_err(|e|e.to_string())??;
Ok(result)}
.await;

        jobs.finish(&app, &job_id, result);
    });

    Ok(id)
}

fn csv_name(name: &str) -> String {
    let first = name.trim_start().chars().next();
    let prefix = if first.is_some_and(|c| "=+-@＝＋－＠".contains(c)) {
        "\t"
    } else {
        ""
    };
    format!("{prefix}{}", name.replace('"', "\"\""))
}
fn retain_benchmarks(dir: &std::path::Path, limit: usize) -> Result<(), String> {
    let mut files = std::fs::read_dir(dir)
        .map_err(|_| "Cannot read benchmark history")?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .collect::<Vec<_>>();
    files.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    files.reverse();
    for path in files.into_iter().skip(limit) {
        std::fs::remove_file(&path).map_err(|_| "Cannot prune old benchmark")?;
        let backup = path.with_extension("bin.bak");
        if backup.exists() {
            std::fs::remove_file(backup).map_err(|_| "Cannot prune old benchmark backup")?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn csv_names_cannot_start_spreadsheet_formulas() {
        for name in ["=SUM(A1:A2)", "  +1", "-1", "@cmd", "＝1"] {
            assert!(csv_name(name).starts_with('\t'));
        }
        assert_eq!(
            csv_name("ordinary,\"quoted\"\nname"),
            "ordinary,\"\"quoted\"\"\nname"
        );
    }
    #[test]
    fn benchmark_retention_preserves_unrelated_files() {
        let root = std::env::temp_dir().join(format!("bench-retention-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..35 {
            std::fs::write(root.join(format!("{i}.bin")), b"fixture").unwrap();
        }
        std::fs::write(root.join("notes.txt"), b"keep").unwrap();
        retain_benchmarks(&root, 32).unwrap();
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 33);
        assert!(root.join("notes.txt").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[tauri::command]
pub async fn results_export(result: Value, csv: bool) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move||{
let dialog=rfd::FileDialog::new().set_title("Export benchmark results").set_file_name(if csv{
"benchmark.csv"}
else{
"benchmark.json"}
);
let Some(path)=dialog.save_file()else{
return Ok(None)}
;
let data=if csv{
let mut text="profile,run,first_token_seconds,seconds,tokens,tokens_per_second,peak_server_ram\n".to_owned();
for row in result["results"].as_array().into_iter().flatten(){
for (i,r) in row["runs"].as_array().into_iter().flatten().enumerate(){
let name=csv_name(row["profile"].as_str().unwrap_or(""));
for value in [&r["first_token_seconds"],&r["seconds"],&r["tokens"],&r["tokens_per_second"],&row["peak_server_ram"]]{if !value.is_number(){return Err("Benchmark CSV requires numeric measurements".into());}}
text.push_str(&format!("\"{name}\",{},{},{},{},{},{}\n",i+1,r["first_token_seconds"],r["seconds"],r["tokens"],r["tokens_per_second"],row["peak_server_ram"]));
}
}
text.into_bytes()}
else{
serde_json::to_vec_pretty(&result).map_err(|e|e.to_string())?}
;
if data.len()>8*1024*1024{
return Err("Export exceeds 8 MiB".into());
}
std::fs::write(&path,data).map_err(|_|"Cannot write results")?;
Ok(Some(path.to_string_lossy().into()))}
).await.map_err(|e|e.to_string())?
}

fn open_browser(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "Invalid browser URL")?;

    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("Invalid browser URL".into());
    }

    #[cfg(windows)]
    {
        #[link(name = "shell32")]
        extern "system" {

            fn ShellExecuteW(
                window: *mut std::ffi::c_void,
                operation: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                show: i32,
            ) -> isize;

        }

        let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();

        let target: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();

        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };

        if result <= 32 {
            return Err("Cannot open default browser".into());
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "Cannot open default browser")?;

        Ok(())
    }
}
fn validate_documentation_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "Invalid documentation URL")?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.path().starts_with("/ggml-org/llama.cpp/blob/")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
    {
        return Err("Only official llama.cpp source documentation is allowed".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn open_llama_documentation(url: String) -> Result<(), String> {
    validate_documentation_url(&url)?;
    tauri::async_runtime::spawn_blocking(move || open_browser(&url))
        .await
        .map_err(|_| "Cannot open documentation browser")?
}
#[cfg(test)]
mod documentation_tests {
    #[test]
    fn only_official_source_documentation_can_be_opened() {
        assert!(super::validate_documentation_url(
            "https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md"
        )
        .is_ok());
        for value in [
            "file:///etc/passwd",
            "https://github.com.evil.test/ggml-org/llama.cpp/blob/master/test",
            "https://github.com/other/repo/blob/master/test",
            "https://user:pass@github.com/ggml-org/llama.cpp/blob/master/test",
            "https://github.com/ggml-org/llama.cpp/blob/master/test?token=private",
        ] {
            assert!(super::validate_documentation_url(value).is_err());
        }
    }
}
