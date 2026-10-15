use serde::{Deserialize, Serialize};

use serde_json::{json, Value};

use std::sync::{Arc, Mutex};

use sysinfo::{Pid, ProcessesToUpdate, System};

use tauri::Manager;

#[derive(Clone, Serialize, Deserialize)]
pub struct MonitoringPreferences {
    pub enabled: bool,
    pub interval_seconds: u32,
}

impl Default for MonitoringPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 2,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct HardwareSnapshot {
    pub cpu: String,
    pub logical_cores: u32,
    pub total_ram: u64,
    pub available_ram: u64,
    pub gpus: Vec<serde_json::Value>,
}

#[derive(Clone, Serialize)]
pub struct TelemetrySample {
    pub timestamp: u64,
    pub cpu_percent: f64,
    pub server_cpu_percent: Option<f64>,
    pub server_ram: Option<u64>,
    pub available_ram: u64,
    pub gpus: Vec<serde_json::Value>,
    pub metrics: serde_json::Value,
}

pub struct Inner {
    pub preferences: MonitoringPreferences,
    pub generation: u64,
    pub shutdown: bool,
    pub history: Vec<TelemetrySample>,
}

#[derive(Clone)]
pub struct State {
    pub inner: Arc<Mutex<Inner>>,
    pub notify: Arc<tokio::sync::Notify>,
    system: Arc<Mutex<System>>,
}

impl Default for State {
    fn default() -> Self {
        let path = crate::storage::profile_directory().join("monitoring.bin");

        let preferences = crate::storage::load_optional::<MonitoringPreferences>(&path, 4096)
            .ok()
            .flatten()
            .filter(|p| {
                if (1..=60).contains(&p.interval_seconds) {
                    true
                } else {
                    crate::storage::damaged(&path, "Invalid monitoring interval");
                    false
                }
            })
            .unwrap_or_default();

        Self {
            inner: Arc::new(Mutex::new(Inner {
                preferences,
                generation: 0,
                shutdown: false,
                history: Vec::new(),
            })),
            notify: Arc::new(tokio::sync::Notify::new()),
            system: Arc::new(Mutex::new(System::new())),
        }
    }
}

pub fn set(state: &State, p: MonitoringPreferences) -> Result<(), String> {
    if !(1..=60).contains(&p.interval_seconds) {
        return Err("Polling interval must be 1–60 seconds".into());
    }

    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;

    crate::storage::write_private_atomic(
        &crate::storage::profile_directory().join("monitoring.bin"),
        &serde_json::to_vec(&p).map_err(|e| e.to_string())?,
    )?;

    inner.preferences = p;

    inner.generation += 1;

    drop(inner);

    state.notify.notify_one();

    Ok(())
}

pub fn hardware() -> HardwareSnapshot {
    let mut s = System::new();

    s.refresh_memory();

    s.refresh_cpu_all();

    HardwareSnapshot {
        cpu: s
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or("Unknown".into()),
        logical_cores: s.cpus().len() as u32,
        total_ram: s.total_memory(),
        available_ram: s.available_memory(),
        gpus: Vec::new(),
    }
}

pub fn sample(state: &State, pid: Option<u32>) -> Result<TelemetrySample, String> {
    let mut s = state.system.lock().map_err(|e| e.to_string())?;

    s.refresh_cpu_all();

    s.refresh_memory();

    // Refresh all entries so new router workers can be discovered and old ones removed.
    if pid.is_some() {
        s.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory(),
        );
    }
    let owned = pid
        .map(|root| {
            descendants(
                root,
                s.processes()
                    .iter()
                    .map(|(id, p)| (id.as_u32(), p.parent().map(|p| p.as_u32()))),
            )
        })
        .unwrap_or_default();
    let cpu = owned
        .iter()
        .filter_map(|id| s.process(Pid::from_u32(*id)))
        .map(|p| p.cpu_usage() as f64)
        .sum();
    let ram = owned
        .iter()
        .filter_map(|id| s.process(Pid::from_u32(*id)))
        .map(|p| p.memory())
        .sum();

    Ok(TelemetrySample {
        timestamp: now(),
        cpu_percent: s.global_cpu_usage() as f64,
        server_cpu_percent: pid.map(|_| cpu),
        server_ram: pid.map(|_| ram),
        available_ram: s.available_memory(),
        gpus: Vec::new(),
        metrics: Value::Null,
    })
}
fn descendants(root: u32, rows: impl IntoIterator<Item = (u32, Option<u32>)>) -> Vec<u32> {
    let mut children = std::collections::HashMap::<u32, Vec<u32>>::new();
    let mut present = std::collections::HashSet::new();
    for (id, parent) in rows {
        present.insert(id);
        if let Some(parent) = parent {
            children.entry(parent).or_default().push(id);
        }
    }
    if !present.contains(&root) {
        return Vec::new();
    }
    let mut queue = vec![root];
    let mut seen = std::collections::HashSet::new();
    let mut owned = Vec::new();
    while let Some(id) = queue.pop() {
        if seen.insert(id) {
            owned.push(id);
            if let Some(children) = children.get(&id) {
                queue.extend(children);
            }
        }
    }
    owned
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub async fn gpus() -> Vec<Value> {
    let executable = if cfg!(windows) {
        std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:/Windows".into())
            .join("System32/nvidia-smi.exe")
    } else {
        "nvidia-smi".into()
    };

    let mut command = tokio::process::Command::new(executable);

    command
        .args([
            "--query-gpu=name,memory.total,memory.used,utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .kill_on_drop(true);

    #[cfg(windows)]
    command.creation_flags(0x08000000);

    match tokio::time::timeout(std::time::Duration::from_secs(2),command.output()).await{

        Ok(Ok(output))if output.status.success() && output.stdout.len()<65536=>String::from_utf8_lossy(&output.stdout).lines().filter_map(|line|{
let p:Vec<_>=line.split(',').map(str::trim).collect();
if p.len()!=4{
return None;
}
Some(json!({
"name":p[0],"total":p[1].parse::<u64>().ok().map(|n|n*1024*1024),"used":p[2].parse::<u64>().ok().map(|n|n*1024*1024),"utilization":p[3].parse::<f64>().ok(),"scope":"whole device"}
))}
).collect(),
        _=>Vec::new()
    }
}

pub async fn adapters() -> Vec<Value> {
    #[cfg(windows)]
    {
        let executable = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:/Windows".into())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");

        let mut command = tokio::process::Command::new(executable);

        command.args(["-NoProfile","-NonInteractive","-Command","[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; @(Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name) | ConvertTo-Json -Compress"]).creation_flags(0x08000000).kill_on_drop(true);

        if let Ok(Ok(output)) =
            tokio::time::timeout(std::time::Duration::from_secs(4), command.output()).await
        {
            if output.status.success() && output.stdout.len() < 65536 {
                if let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) {
                    let names = if let Some(a) = value.as_array() {
                        a.clone()
                    } else {
                        vec![value]
                    };

                    return names
                        .into_iter()
                        .filter_map(|v| {
                            v.as_str().map(|name|json!({
"name":name,"scope":"adapter detection only","total":null,"used":null,"utilization":null}
))
                        })
                        .collect();
                }
            }
        }
    }

    #[cfg(unix)]
    {
        let mut result = Vec::new();

        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();

                if !name.starts_with("card") || name.contains('-') {
                    continue;
                }

                let vendor =
                    std::fs::read_to_string(e.path().join("device/vendor")).unwrap_or_default();

                let brand = match vendor.trim() {
                    "0x1002" => "AMD",
                    "0x8086" => "Intel",
                    "0x10de" => "NVIDIA",
                    _ => "GPU",
                };

                result.push(json!({
"name":format!("{brand} ({name})"),"scope":"adapter detection only","total":null,"used":null,"utilization":null}
));
            }
        }

        result
    }

    #[cfg(windows)]
    Vec::new()
}

pub fn start(app: tauri::AppHandle, state: State, server: crate::server::State) {
    tauri::async_runtime::spawn(run(state, server, move |sample| {
        let _ = app.emit_all("launcher-telemetry", sample);
    }));
}

async fn run(
    state: State,
    server: crate::server::State,
    emit: impl Fn(&TelemetrySample) + Send + 'static,
) {
    loop {
        let (p, g, shutdown) = match state.inner.lock() {
            Ok(i) => (i.preferences.clone(), i.generation, i.shutdown),
            Err(_) => break,
        };

        if shutdown {
            break;
        }

        if !p.enabled {
            state.notify.notified().await;

            continue;
        }

        let session = crate::session::current(&server).ok();

        let pid = session.as_ref().map(|s| s.pid);

        let worker = state.clone();

        let cycle_started = std::time::Instant::now();

        let sampling = async {
            let mut sample = tauri::async_runtime::spawn_blocking(move || sample(&worker, pid))
                .await
                .map_err(|e| e.to_string())??;

            sample.gpus = gpus().await;

            if let Some(s) = session {
                let metrics_future = async {
                    let mut paths = vec!["/metrics".to_owned()];

                    if s.router {
                        paths.clear();

                        if let Ok(models) =
                            crate::session::json_request(&s, "GET", "/models", None).await
                        {
                            for model in models["data"].as_array().into_iter().flatten().take(16) {
                                let status = model["status"]["value"]
                                    .as_str()
                                    .or_else(|| model["status"].as_str());

                                if matches!(status, Some("loaded" | "ready" | "running")) {
                                    if let Some(id) = model["id"].as_str() {
                                        let mut url =
                                            reqwest::Url::parse("http://localhost/metrics")
                                                .map_err(|_| "Invalid metrics URL")?;

                                        url.query_pairs_mut().append_pair("model", id);

                                        paths.push(format!(
                                            "{}?{}",
                                            url.path(),
                                            url.query().unwrap_or("")
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    let mut metrics = serde_json::Map::new();

                    for path in paths {
                        if let Ok(mut body) = crate::session::request(&s, "GET", &path, None).await
                        {
                            let mut bytes = Vec::new();

                            while let Ok(Some(c)) = body.chunk().await {
                                if bytes.len() + c.len() > 128 * 1024 {
                                    break;
                                }

                                bytes.extend(c);
                            }

                            metrics.insert(
                                path,
                                json!(parse_metrics(&String::from_utf8_lossy(&bytes))),
                            );
                        }
                    }

                    Ok::<_, String>(Value::Object(metrics))
                };

                if let Ok(Ok(metrics)) =
                    tokio::time::timeout(std::time::Duration::from_millis(1500), metrics_future)
                        .await
                {
                    sample.metrics = metrics;
                }
            }

            Ok::<_, String>(sample)
        };

        tokio::select! {
        _=state.notify.notified()=>continue,result=tokio::time::timeout(std::time::Duration::from_secs(4),sampling)=>{
        if let Ok(Ok(sample))=result{
        if let Ok(mut i)=state.inner.lock(){
        if i.generation==g&&i.preferences.enabled{
        i.history.retain(|s|s.timestamp+300>=sample.timestamp);
        if i.history.len()>=300{
        i.history.remove(0);
        }
        i.history.push(sample.clone());
        emit(&sample);
        }
        }
        }
        }
        }

        tokio::select! {
        _=state.notify.notified()=>{
        }
        ,_=tokio::time::sleep(std::time::Duration::from_secs(p.interval_seconds as u64).saturating_sub(cycle_started.elapsed()))=>{
        }
        }
    }
}

fn parse_metrics(text: &str) -> std::collections::BTreeMap<String, f64> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut p = l.split_whitespace();

            let name = p.next()?;

            let value = p.next()?.parse().ok()?;

            Some((name.into(), value))
        })
        .take(256)
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn router_workers_helpers_and_grandchildren_are_counted_without_unrelated_processes() {
        let mut owned = super::descendants(
            10,
            [
                (10, None),
                (11, Some(10)),
                (12, Some(11)),
                (13, Some(10)),
                (20, None),
                (21, Some(20)),
            ],
        );
        owned.sort();
        assert_eq!(owned, [10, 11, 12, 13]);
        assert!(super::descendants(99, [(10, None)]).is_empty());
    }

    use super::*;

    #[test]
    fn validates_polling_interval() {
        let state = State::default();

        assert!(set(
            &state,
            MonitoringPreferences {
                enabled: false,
                interval_seconds: 0
            }
        )
        .is_err());

        assert!(set(
            &state,
            MonitoringPreferences {
                enabled: true,
                interval_seconds: 61
            }
        )
        .is_err());
    }

    #[tokio::test]
    async fn disabled_monitoring_stays_idle_and_resumes_without_stale_samples() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let state = State::default();

        {
            let mut i = state.inner.lock().unwrap();

            i.preferences.enabled = false;

            i.preferences.interval_seconds = 1;
        }

        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();

        let s = state.clone();

        let worker = tokio::spawn(run(s, crate::server::State::default(), move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        }));

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        assert_eq!(count.load(Ordering::Relaxed), 0);

        {
            let mut i = state.inner.lock().unwrap();

            i.preferences.enabled = true;

            i.generation += 1;
        }

        state.notify.notify_one();

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while count.load(Ordering::Relaxed) == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap();

        {
            let mut i = state.inner.lock().unwrap();

            i.preferences.enabled = false;

            i.generation += 1;
        }

        state.notify.notify_one();

        let before = count.load(Ordering::Relaxed);

        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        assert_eq!(count.load(Ordering::Relaxed), before);

        state.inner.lock().unwrap().shutdown = true;

        state.notify.notify_one();

        tokio::time::timeout(std::time::Duration::from_secs(1), worker)
            .await
            .unwrap()
            .unwrap();
    }
}
