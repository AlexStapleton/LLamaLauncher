#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde::Serialize;
use std::{
    fs,
    net::{IpAddr, TcpListener},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
#[cfg(test)]
use std::{process::Child, sync::Mutex};
use tauri::{Manager, State};
mod downloads;
mod features;
mod framing;
mod gguf;
mod hub;
mod jobs;
mod logging;
mod mcp;
mod monitor;
mod process_tree;
mod profiles;
mod registry;
mod server;
mod session;
mod storage;

mod catalog;
#[cfg(test)]
use catalog::scan_models;
use catalog::{scan_models_cancel, ModelInfo, SCAN_GENERATION};
mod config;
mod credentials;
#[cfg(test)]
use config::ArgumentOverride;
use config::Config;

type AppState = server::State;
#[derive(Default)]
pub struct CloseGuard {
    allow: std::sync::atomic::AtomicBool,
    ready: std::sync::atomic::AtomicBool,
}

pub(crate) fn configure_runtime_environment(command: &mut Command) {
    // AppImage's GUI libraries must not shadow a separately installed runtime.
    if std::env::var_os("APPIMAGE").is_some() {
        if let Some(directory) = Path::new(command.get_program())
            .parent()
            .map(Path::to_path_buf)
        {
            command.env("LD_LIBRARY_PATH", directory);
        } else {
            command.env_remove("LD_LIBRARY_PATH");
        }
        command.env_remove("LD_PRELOAD");
    }
}

fn checked_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("Please select the executable, models folder, and model file.".into());
    }
    let path = Path::new(value);
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Paths cannot contain parent-directory traversal (..).".into());
    }
    fs::canonicalize(path).map_err(|e| format!("Cannot access {}: {}", value, e))
}
fn validate(config: &Config) -> Result<(PathBuf, Option<PathBuf>, Vec<IpAddr>), String> {
    registry::resolve(&config.runtime_id, "runtime", &config.llama_cpp_path)?;
    if !config.router_mode && !config.model_id.is_empty() {
        let path = Path::new(&config.model_file);
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            Path::new(&config.models_path).join(path)
        };
        registry::resolve(&config.model_id, "model", &path.to_string_lossy())?;
    }
    if config.mcp_profile.enabled {
        validate_mcp_profile(&config.mcp_profile)?;
        if !config.router_mode {
            return Err("Enable router mode to use the configured MCP connection.".into());
        }
        if config.argument_overrides.iter().any(|a| {
            matches!(
                a.flag.as_str(),
                "--mcp-servers-json" | "--mcp-servers-config"
            )
        }) || config
            .additional_args
            .iter()
            .any(|a| a.starts_with("--mcp-servers-"))
        {
            return Err(
                "Configure MCP in the MCP tab or with custom flags, rather than both.".into(),
            );
        }
    }
    if config.router_mode {
        if !config.models_path.is_empty() && !checked_path(&config.models_path)?.is_dir() {
            return Err("Router models path must be a directory.".into());
        }
        if config.argument_overrides.iter().any(|a| {
            matches!(
                a.flag.as_str(),
                "--hf-repo" | "-hf" | "-hfr" | "--model-url" | "-mu" | "--docker-repo" | "-dr"
            ) || a.flag.ends_with("-default")
                || a.flag.ends_with("-spec")
        }) {
            return Err("Router mode must not specify a fixed or remote model. Disable the model-source override.".into());
        }
    }
    let binary = checked_path(&config.llama_cpp_path)?;
    if !binary.is_file() {
        return Err("The llama-server path must be an executable file.".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&binary)
            .map_err(|e| e.to_string())?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err("The llama-server file needs executable permission (chmod +x).".into());
        }
    }
    if cfg!(windows)
        && !binary
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    {
        return Err("Select a Windows llama-server.exe executable.".into());
    }
    let remote = uses_alternate_source(config);
    let model = if remote {
        None
    } else {
        let directory = checked_path(&config.models_path)?;
        if !directory.is_dir() {
            return Err("Select a models directory.".into());
        }
        let selected = Path::new(&config.model_file);
        let model = checked_path(
            &if selected.is_absolute() {
                selected.to_path_buf()
            } else {
                directory.join(selected)
            }
            .to_string_lossy(),
        )?;
        if !model.is_file() || !model.starts_with(&directory) {
            return Err("Select a model file inside the models directory.".into());
        }
        if !model
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gguf"))
        {
            return Err("llama-server requires a GGUF model.".into());
        }
        Some(model)
    };
    let hosts = if config.host.ends_with(".sock") {
        let socket = Path::new(&config.host);
        checked_path(
            socket
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .as_ref(),
        )?;
        Vec::new()
    } else {
        config.host.split(',').map(|host| host.trim().parse::<IpAddr>().map_err(|_| "Listen address must be an IP address, comma-separated IP addresses, or a socket path ending in .sock.".to_string())).collect::<Result<Vec<_>, _>>()?
    };
    if config.port < 1024 {
        return Err("Port must be between 1024 and 65535.".into());
    }
    validate_arguments(config)?;
    Ok((binary, model, hosts))
}

fn validate_arguments(config: &Config) -> Result<(), String> {
    if config.schema_version > 3 {
        return Err("Configuration requires a newer LLamaLauncher version.".into());
    }
    for option in &config.argument_overrides {
        if !option.flag.starts_with('-')
            || !option
                .flag
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            || option.flag.chars().any(|c| c.is_whitespace() || c == '\0')
        {
            return Err("Invalid argument flag.".into());
        }
        if matches!(option.flag.as_str(), "--model" | "-m" | "--host" | "--port") {
            return Err("Choose the model in Models; set the listen address and port in Launch instead of overriding them.".into());
        }
        if option.values.iter().any(|v| v.contains('\0')) {
            return Err("Argument values cannot contain null characters.".into());
        }
        validate_numeric(&option.flag, option.values.first().map(String::as_str))?;
    }
    if config.additional_args.iter().any(|v| v.contains('\0')) {
        return Err("Additional arguments cannot contain null characters.".into());
    }
    if config.additional_args.iter().any(|v| {
        matches!(
            v.split('=').next().unwrap_or(""),
            "--model" | "-m" | "--host" | "--port"
        )
    }) {
        return Err(
            "Choose the model in Models; set the listen address and port in Launch.".into(),
        );
    }
    let mut args = config.additional_args.iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(f, v)| (f, Some(v)));
        if numeric_rule(flag).is_some() {
            validate_numeric(flag, inline.or_else(|| args.next().map(String::as_str)))?;
        }
        if config.router_mode && remote_model_flag(flag) {
            return Err(
                "Router mode must not specify a fixed remote model in custom arguments".into(),
            );
        }
    }
    Ok(())
}
fn remote_model_flag(flag: &str) -> bool {
    matches!(
        flag,
        "--hf-repo" | "-hf" | "-hfr" | "--model-url" | "-mu" | "--docker-repo" | "-dr"
    ) || flag.ends_with("-default")
        || flag.ends_with("-spec")
}
fn numeric_rule(flag: &str) -> Option<(f64, f64, bool)> {
    match flag {
        "--ctx-size" | "-c" => Some((0.0, 1048576.0, true)),
        "--batch-size" | "-b" | "--ubatch-size" | "-ub" => Some((1.0, 1048576.0, true)),
        "--threads" | "-t" | "--threads-batch" | "-tb" | "--threads-http" => {
            Some((-1.0, 4096.0, true))
        }
        "--parallel" | "-np" => Some((-1.0, 4096.0, true)),
        "--n-predict" | "--predict" | "-n" => Some((-2.0, 2147483647.0, true)),
        "--top-k" => Some((-1.0, 2147483647.0, true)),
        "--temp" | "--temperature" => Some((0.0, 100.0, false)),
        "--top-p" | "--min-p" | "--typical" | "--typical-p" => Some((0.0, 1.0, false)),
        _ => None,
    }
}
fn validate_capabilities(config: &Config, help: &str) -> Result<(), String> {
    let mut aliases = std::collections::HashMap::<String, String>::new();
    for line in help
        .lines()
        .filter(|line| line.trim_start().starts_with('-'))
    {
        let names: Vec<_> = line
            .split_whitespace()
            .map(|v| v.trim_matches(','))
            .filter(|v| {
                v.starts_with('-')
                    && v.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            })
            .collect();
        let canonical = names.iter().find(|n| n.starts_with("--")).copied();
        if let Some(canonical) = canonical {
            for name in names {
                aliases.insert(name.into(), canonical.into());
            }
        }
    }
    if aliases.is_empty() {
        return Err("Selected executable did not provide a usable option catalog".into());
    }
    let mut requested = Vec::new();
    for item in &config.argument_overrides {
        requested.push((item.flag.as_str(), item.values.first().map(String::as_str)));
    }
    let mut extra = config.additional_args.iter().peekable();
    while let Some(arg) = extra.next() {
        let (flag, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(f, v)| (f, Some(v)));
        if flag.starts_with('-') && flag.parse::<f64>().is_err() {
            requested.push((flag, inline.or_else(|| extra.peek().map(|v| v.as_str()))));
        }
    }
    for (flag, value) in requested {
        let canonical=aliases.get(flag).ok_or_else(||format!("{flag} is not supported by the selected llama-server; refresh its options or choose a compatible runtime"))?;
        validate_numeric(canonical, value)?;
    }
    if config.router_mode && !config.models_path.is_empty() && !aliases.contains_key("--models-dir")
    {
        return Err("Selected runtime does not support router option --models-dir".into());
    }
    Ok(())
}
fn validate_numeric(flag: &str, value: Option<&str>) -> Result<(), String> {
    if let Some((min, max, integer)) = numeric_rule(flag) {
        let number = value
            .ok_or_else(|| format!("{flag} requires a value"))?
            .parse::<f64>()
            .map_err(|_| format!("{flag} requires a number"))?;
        if !number.is_finite() || number < min || number > max || (integer && number.fract() != 0.0)
        {
            return Err(format!(
                "{flag} requires {} between {min} and {max}",
                if integer { "an integer" } else { "a number" }
            ));
        }
    }
    Ok(())
}

fn uses_alternate_source(config: &Config) -> bool {
    if config.router_mode {
        return true;
    }
    let sources = [
        "--hf-repo",
        "-hf",
        "-hfr",
        "--model-url",
        "-mu",
        "--docker-repo",
        "-dr",
        "--models-dir",
        "--models-preset",
    ];
    let utilities = [
        "--help",
        "-h",
        "--usage",
        "--version",
        "--cache-list",
        "-cl",
        "--completion-bash",
        "--list-devices",
    ];
    config.argument_overrides.iter().any(|a| {
        (sources.contains(&a.flag.as_str()) && !a.values.is_empty())
            || utilities.contains(&a.flag.as_str())
            || a.flag.ends_with("-default")
            || a.flag.ends_with("-spec")
    }) || config.additional_args.iter().any(|a| {
        sources.contains(&a.split('=').next().unwrap_or("").to_owned().as_str())
            || utilities.contains(&a.as_str())
    })
}

fn arguments(config: &Config, model: Option<&Path>) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(model) = model.filter(|_| !config.router_mode) {
        args.extend(["--model".into(), model.to_string_lossy().into_owned()]);
    }
    args.extend([
        "--host".into(),
        config.host.clone(),
        "--port".into(),
        config.port.to_string(),
    ]);
    if config.router_mode
        && !config.models_path.is_empty()
        && !config
            .argument_overrides
            .iter()
            .any(|a| a.flag == "--models-dir")
        && !config
            .additional_args
            .iter()
            .any(|a| a.starts_with("--models-dir"))
    {
        args.extend(["--models-dir".into(), config.models_path.clone()]);
    }
    let mut ordered: Vec<_> = config.argument_overrides.iter().collect();
    ordered.sort_by_key(|option| {
        if matches!(option.flag.as_str(), "--jinja" | "--no-jinja") {
            0
        } else {
            1
        }
    });
    for option in ordered {
        args.push(option.flag.clone());
        args.extend(option.values.clone());
    }
    args.extend(config.additional_args.clone());
    args
}

#[tauri::command]
async fn list_models(
    app: tauri::AppHandle,
    directory: String,
    force: Option<bool>,
) -> Result<Vec<ModelInfo>, String> {
    let generation = SCAN_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    tauri::async_runtime::spawn_blocking(move || {
        type ScanCache = std::sync::Mutex<Option<(PathBuf, std::time::Instant, Vec<ModelInfo>)>>;
        static CACHE: std::sync::OnceLock<ScanCache> = std::sync::OnceLock::new();
        let cache = CACHE.get_or_init(Default::default);
        let path = fs::canonicalize(checked_path(&directory)?).map_err(|e| e.to_string())?;
        if !force.unwrap_or(false) {
            if let Some((previous, time, models)) = &*cache.lock().map_err(|e| e.to_string())? {
                if previous == &path && time.elapsed().as_secs() < 5 {
                    return Ok(models.clone());
                }
            }
        }
        let models = scan_models_cancel(&path, Some(generation), |visited, models| {
            let _ = app.emit_all(
                "model-scan",
                serde_json::json!({"directory":directory,"visited":visited,"models":models}),
            );
        })?;
        *cache.lock().map_err(|e| e.to_string())? =
            Some((path, std::time::Instant::now(), models.clone()));
        Ok(models)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn cancel_models() {
    SCAN_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
#[tauri::command]
async fn server_help(
    _app: tauri::AppHandle,
    binary: String,
    force: Option<bool>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || runtime_help(&binary, force.unwrap_or(false)))
        .await
        .map_err(|e| e.to_string())?
}
fn runtime_help(binary: &str, force: bool) -> Result<String, String> {
    let binary = checked_path(binary)?;
    type HelpCache =
        std::sync::Mutex<std::collections::HashMap<(PathBuf, u64, std::time::SystemTime), String>>;
    static HELP_CACHE: std::sync::OnceLock<HelpCache> = std::sync::OnceLock::new();
    let metadata = fs::metadata(&binary).map_err(|e| e.to_string())?;
    let key = (
        fs::canonicalize(&binary).map_err(|e| e.to_string())?,
        metadata.len(),
        metadata.modified().map_err(|e| e.to_string())?,
    );
    let help_cache = HELP_CACHE.get_or_init(Default::default);
    if !force {
        if let Some(cached) = help_cache.lock().map_err(|e| e.to_string())?.get(&key) {
            return Ok(cached.clone());
        }
    }
    if !binary.is_file()
        || (cfg!(windows)
            && !binary
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe")))
    {
        return Err("Choose llama-server.exe first.".into());
    }
    let cache = storage::profile_directory().join("cache");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let output_path = cache.join(format!("help-{}.txt", stamp));
    let _output_guard = server::Snapshot(output_path.clone());
    let output = fs::File::create(&output_path).map_err(|e| e.to_string())?;
    let mut command = Command::new(&binary);
    command
        .arg("--help")
        .current_dir(binary.parent().ok_or("Missing executable folder.")?)
        .stdout(output.try_clone().map_err(|e| e.to_string())?)
        .stderr(output)
        .stdin(Stdio::null());
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
    let mut child = server::TemporaryProcess::new(command.spawn().map_err(|e| e.to_string())?)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let result = loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            if fs::metadata(&output_path).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
                break Err("Executable help exceeded 1 MiB.".into());
            }
            break fs::read_to_string(&output_path).map_err(|e| e.to_string());
        }
        if std::time::Instant::now() >= deadline
            || fs::metadata(&output_path).is_ok_and(|m| m.len() > 1024 * 1024)
        {
            let _ = child.kill();
            let _ = child.wait();
            break Err("Reading executable options timed out after 15 seconds.".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let _ = fs::remove_file(output_path);
    if let Ok(text) = &result {
        let mut cache = help_cache.lock().map_err(|e| e.to_string())?;
        if cache.len() >= 8 {
            cache.clear();
        }
        cache.insert(key, text.clone());
    }
    result
}

#[tauri::command]
async fn command_preview(config: Config) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || preview_inner(config))
        .await
        .map_err(|e| e.to_string())?
}
fn preview_inner(config: Config) -> Result<Vec<String>, String> {
    validate_arguments(&config)?;
    let model = if config.model_file.is_empty() || uses_alternate_source(&config) {
        None
    } else {
        let path = PathBuf::from(&config.model_file);
        Some(if path.is_absolute() {
            path
        } else {
            Path::new(&config.models_path).join(path)
        })
    };
    let mut preview = config.clone();
    for item in &mut preview.argument_overrides {
        if secret_flag(&item.flag) {
            item.values.fill("[redacted]".into());
        }
    }
    redact_arguments(&mut preview.additional_args);
    let mut args = arguments(&preview, model.as_deref());
    if config.mcp_profile.enabled {
        args.extend([
            "--mcp-servers-json".into(),
            "[MCP configuration; credentials protected]".into(),
        ]);
    }
    args.insert(0, config.llama_cpp_path.clone());
    let sensitive = credentials::values(&config);
    for arg in &mut args {
        *arg = credentials::redact(std::mem::take(arg), &sensitive);
    }
    Ok(args)
}

fn secret_flag(flag: &str) -> bool {
    credentials::secret_flag(flag)
}

fn redact_arguments(args: &mut [String]) {
    let mut secret = false;
    let mut required = false;
    for arg in args {
        if required {
            *arg = "[redacted]".into();
            required = false;
            continue;
        }
        if let Some((flag, _)) = arg.split_once('=') {
            if secret_flag(flag) {
                *arg = format!("{flag}=[redacted]");
                secret = false;
                continue;
            }
        }
        if arg.starts_with('-') {
            secret = secret_flag(arg);
            required = secret;
        } else if secret {
            *arg = "[redacted]".into();
        }
    }
}

#[tauri::command]
async fn server_logs(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || read_server_logs(app))
        .await
        .map_err(|e| e.to_string())?
}
fn read_server_logs(app: tauri::AppHandle) -> Result<String, String> {
    let config = app
        .state::<AppState>()
        .0
        .lock()
        .ok()
        .and_then(|m| m.instance.as_ref().map(|i| i.config.clone()));
    let secrets = config.as_ref().map(credentials::values).unwrap_or_default();
    let directory = storage::profile_directory().join("logs");
    let mut result = format!("Log folder: {}\n", directory.display());
    for name in ["server-stderr.log", "server-stdout.log"] {
        result.push_str(&format!("\n--- {name} ---\n"));
        if let Ok(text) = logging::tail(&directory.join(name), 32768) {
            result.push_str(&credentials::redact(text, &secrets));
        }
    }
    Ok(result)
}
#[tauri::command]
async fn get_defaults(app: tauri::AppHandle) -> Result<Config, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut config = Config::default();
        let models = storage::profile_directory().join("models");
        fs::create_dir_all(&models).map_err(|_| "Cannot create the portable models folder")?;
        config.models_path = models.to_string_lossy().into();
        let mut candidates = Vec::new();
        if let Ok(executable) = std::env::current_exe() {
            if let Some(parent) = executable.parent() {
                candidates.push(parent.join("runtime"));
            }
        }
        if let Some(resources) = app.path_resolver().resource_dir() {
            candidates.push(resources.join("artifacts/portable/runtime"));
        }
        for directory in candidates {
            let binary = directory.join(if cfg!(windows) {
                "llama-server.exe"
            } else {
                "llama-server"
            });
            if binary.is_file() {
                config.llama_cpp_path = binary.to_string_lossy().trim_start_matches(r"\\?\").into();
                if let Ok(artifact) = registry::import(&binary, false, None) {
                    config.runtime_id = artifact.id;
                }
                break;
            }
        }
        if let Some(profile) = mcp::load_profile() {
            config.router_mode = profile.enabled;
            config.mcp_profile = profile;
        }
        Ok(config)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Serialize)]
struct PlatformInfo {
    windows: bool,
}
#[tauri::command]
fn platform_info() -> PlatformInfo {
    PlatformInfo {
        windows: cfg!(windows),
    }
}
fn validate_saved_profile(config: &Config) -> Result<(), String> {
    if config.port < 1024 {
        return Err("Profile port must be between 1024 and 65535.".into());
    }
    validate_arguments(config)?;
    if config.mcp_profile.enabled {
        validate_mcp_profile(&config.mcp_profile)?;
    }
    Ok(())
}
#[tauri::command]
async fn profiles_list(app: tauri::AppHandle) -> Result<Vec<profiles::ProfileSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<profiles::Manager>();
        let _lock = manager.0.lock().map_err(|e| e.to_string())?;
        Ok(profiles::read(&profiles::path())?.summaries())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn profiles_load(
    app: tauri::AppHandle,
    id: String,
) -> Result<profiles::LaunchProfile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<profiles::Manager>();
        let _lock = manager.0.lock().map_err(|e| e.to_string())?;
        profiles::read(&profiles::path())?.get(&id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn profiles_save(
    app: tauri::AppHandle,
    id: Option<String>,
    name: String,
    config: Config,
) -> Result<profiles::LaunchProfile, String> {
    validate_saved_profile(&config)?;
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<profiles::Manager>();
        let _lock = manager.0.lock().map_err(|e| e.to_string())?;
        let mut library = profiles::read(&profiles::path())?;
        let entry = library.save(id, name, config)?;
        profiles::write(&profiles::path(), &library)?;
        Ok(entry)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn profiles_copy(
    app: tauri::AppHandle,
    id: String,
    name: String,
    rename: bool,
) -> Result<profiles::LaunchProfile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<profiles::Manager>();
        let _lock = manager.0.lock().map_err(|e| e.to_string())?;
        let mut library = profiles::read(&profiles::path())?;
        let original = library.get(&id)?;
        let entry = library.save(if rename { Some(id) } else { None }, name, original.config)?;
        profiles::write(&profiles::path(), &library)?;
        Ok(entry)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn profiles_delete(app: tauri::AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<profiles::Manager>();
        let _lock = manager.0.lock().map_err(|e| e.to_string())?;
        let mut library = profiles::read(&profiles::path())?;
        library.delete(&id)?;
        profiles::write(&profiles::path(), &library)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn validate_mcp_profile(profile: &mcp::Profile) -> Result<(), String> {
    if profile.name.is_empty()
        || !profile
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("MCP name must contain only letters, numbers, underscores or hyphens.".into());
    }
    if profile.transport == "http" {
        let url = reqwest::Url::parse(&profile.url)
            .map_err(|_| "Enter a complete MCP HTTP URL.".to_string())?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(
                "MCP URL must be HTTP(S), without embedded credentials or fragments.".into(),
            );
        }
        if profile.bearer_token.contains(['\r', '\n']) {
            return Err("Bearer token cannot contain line breaks.".into());
        }
    } else if profile.transport == "stdio" {
        if profile.command.trim().is_empty()
            || profile.command.contains('\0')
            || profile.args.iter().any(|a| a.contains('\0'))
        {
            return Err("Enter a local MCP executable and valid arguments.".into());
        }
        if cfg!(windows) && (profile.command.ends_with(".cmd") || profile.command.ends_with(".bat"))
        {
            return Err(
                "Use an executable such as node.exe or python.exe, with the script in Arguments."
                    .into(),
            );
        }
        if !profile.cwd.is_empty() && !checked_path(&profile.cwd)?.is_dir() {
            return Err("MCP working directory must exist.".into());
        }
    } else {
        return Err("Choose HTTP or local stdio transport.".into());
    }
    Ok(())
}

#[tauri::command]
async fn mcp_connect(
    profile: mcp::Profile,
    state: State<'_, mcp::Manager>,
) -> Result<mcp::Report, String> {
    validate_mcp_profile(&profile)?;
    let connection = state.connection.clone();
    let (generation, old) = {
        let mut state = connection.lock().map_err(|e| e.to_string())?;
        state.generation = state.generation.wrapping_add(1);
        state.report = mcp::Report::default();
        (state.generation, state.client.take())
    };
    tauri::async_runtime::spawn_blocking(move || {
        drop(old);
        let mut profile = profile;
        profile.enabled = true;
        let mut client = mcp::Client::new(profile.clone())?;
        let report = client.initialize()?;
        if profile.transport == "http" {
            profile.url = report.endpoint.clone();
        }
        let mut state = connection.lock().map_err(|e| e.to_string())?;
        if state.generation != generation {
            drop(state);
            return Err(
                "Connection request superseded by disconnect or a newer connection.".into(),
            );
        }
        mcp::save_profile(&profile)?;
        state.client = Some(client);
        state.report = report.clone();
        Ok::<_, String>(report)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn mcp_test(profile: mcp::Profile) -> Result<mcp::Report, String> {
    validate_mcp_profile(&profile)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut client = mcp::Client::new(profile)?;
        let mut report = client.initialize()?;
        report.connected = false;
        report.message =
            "Test passed: initialization and tool discovery succeeded. No tools were executed."
                .into();
        Ok::<_, String>(report)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn mcp_disconnect(state: State<'_, mcp::Manager>) -> Result<(), String> {
    let connection = state.connection.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let old = {
            let mut connection = connection.lock().map_err(|e| e.to_string())?;
            connection.generation = connection.generation.wrapping_add(1);
            connection.report = mcp::Report::default();
            if let Some(mut profile) = mcp::load_profile() {
                profile.enabled = false;
                mcp::save_profile(&profile)?;
            }
            connection.client.take()
        };
        drop(old);
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn mcp_status(
    app: tauri::AppHandle,
    state: State<'_, mcp::Manager>,
) -> Result<mcp::Heartbeat, String> {
    let connection = state.connection.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (client, generation) = {
            let mut state = connection.lock().map_err(|e| e.to_string())?;
            (state.client.take(), state.generation)
        };
        if let Some(mut client) = client {
            let ping = client
                .ping()
                .and_then(|_| client.refresh_tools_if_changed());
            let mut state = connection.lock().map_err(|e| e.to_string())?;
            if state.generation == generation {
                match ping {
                    Ok(tools) => {
                        if let Some(tools) = tools {
                            state.report.tools = tools;
                            let _ = app.emit_all("mcp-tools", &state.report.tools);
                        }
                        state.client = Some(client);
                    }
                    Err(error) => {
                        state.report.connected = false;
                        state.report.message = error;
                        // The transport is dropped after the state lock is released.
                        drop(state);
                        drop(client);
                    }
                }
            }
        }
        let state = connection.lock().map_err(|e| e.to_string())?;
        Ok::<_, String>(mcp::Heartbeat {
            connected: state.report.connected,
            message: state.report.message.clone(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn browse_path(kind: String, directory: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = tauri::api::dialog::blocking::FileDialogBuilder::new();
        if let Some(directory) = directory.filter(|d| Path::new(d).is_dir()) {
            dialog = dialog.set_directory(directory);
        }
        let result = match kind.as_str() {
            "binary" => {
                dialog = dialog.set_title("Select llama-server executable");
                if cfg!(windows) {
                    dialog = dialog.add_filter("Executable", &["exe"]);
                }
                dialog.pick_file()
            }
            "models" => dialog.set_title("Select models folder").pick_folder(),
            "model" => dialog
                .set_title("Select GGUF model")
                .add_filter("GGUF model", &["gguf"])
                .pick_file(),
            "file" => dialog.set_title("Select file").pick_file(),
            _ => return Err("Unknown file selection.".into()),
        };
        Ok(result.map(|p| p.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn save_config(config: Config) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let config = export_config(config);
        let path = tauri::api::dialog::blocking::FileDialogBuilder::new()
            .set_title("Save configuration")
            .add_filter("JSON", &["json"])
            .set_file_name("llama-server-config.json")
            .save_file();
        match path {
            Some(path) => {
                let content = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
                fs::write(&path, content).map_err(|e| e.to_string())?;
                Ok(Some(path.to_string_lossy().into_owned()))
            }
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
fn export_config(config: Config) -> Config {
    credentials::share(config)
}
#[tauri::command]
async fn load_config() -> Result<Option<Config>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = tauri::api::dialog::blocking::FileDialogBuilder::new()
            .set_title("Load configuration")
            .add_filter("JSON", &["json"])
            .pick_file();
        match path {
            Some(path) => {
                use std::io::Read;
                let mut content = Vec::new();
                fs::File::open(path)
                    .map_err(|e| e.to_string())?
                    .take(1024 * 1024 + 1)
                    .read_to_end(&mut content)
                    .map_err(|e| e.to_string())?;
                if content.len() > 1024 * 1024 {
                    return Err("Configuration exceeded 1 MiB.".into());
                }
                decode_config(&content).map(Some)
            }
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
fn decode_config(content: &[u8]) -> Result<Config, String> {
    let mut value: serde_json::Value =
        serde_json::from_slice(content).map_err(|e| format!("Invalid configuration: {e}"))?;
    let object = value
        .as_object_mut()
        .ok_or("Configuration must be a JSON object.")?;
    if object
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        > 3
    {
        return Err("Configuration requires a newer LLamaLauncher version.".into());
    }
    if !object.contains_key("argument_overrides") {
        let fields = [
            ("n_ctx", "--ctx-size"),
            ("n_batch", "--batch-size"),
            ("max_tokens", "--n-predict"),
            ("temperature", "--temp"),
            ("top_p", "--top-p"),
            ("top_k", "--top-k"),
            ("presence_penalty", "--presence-penalty"),
            ("frequency_penalty", "--frequency-penalty"),
            ("repeat_penalty", "--repeat-penalty"),
        ];
        let mut overrides = Vec::new();
        for (field, flag) in fields {
            if let Some(number) = object.get(field).filter(|v| v.is_number()) {
                overrides.push(serde_json::json!({"flag":flag,"values":[number.to_string()]}));
            }
        }
        if let Some(layers) = object.get("n_gpu_layers").and_then(|v| v.as_i64()) {
            let value = if object.get("gpu_offload").and_then(|v| v.as_bool()) == Some(false) {
                "0".into()
            } else if layers == -1 {
                "all".into()
            } else {
                layers.to_string()
            };
            overrides.push(serde_json::json!({"flag":"--gpu-layers","values":[value]}));
        }
        if object.get("verbose").and_then(|v| v.as_bool()) == Some(true) {
            overrides.push(serde_json::json!({"flag":"--verbose","values":[]}));
        }
        object.insert("argument_overrides".into(), serde_json::json!(overrides));
    }
    object.insert("schema_version".into(), serde_json::json!(3));
    serde_json::from_value(value).map_err(|e| format!("Invalid configuration: {e}"))
}
#[tauri::command]
async fn start_server(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    config: Config,
) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || start_server_inner(app, &state, config))
        .await
        .map_err(|e| e.to_string())?
}
fn start_server_inner(
    _app: tauri::AppHandle,
    state: &AppState,
    config: Config,
) -> Result<String, String> {
    let (binary, model, hosts) = validate(&config)?;
    validate_capabilities(&config, &runtime_help(&config.llama_cpp_path, false)?)?;
    let isolated = state
        .0
        .lock()
        .map_err(|_| "Server state unavailable")?
        .isolated;
    let (api_token, api_prefix) = session::authenticate(&config, isolated)?;
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    if manager.benchmark {
        return Err("Wait for the benchmark to finish or cancel it first.".into());
    }
    if let Some(instance) = manager.instance.as_mut() {
        if instance
            .child
            .try_wait()
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err("A server is already running. Stop it first.".into());
        }
    }
    manager.instance = None;
    for host in hosts {
        drop(
            TcpListener::bind((host, config.port))
                .map_err(|e| format!("Cannot use {}:{}: {}", host, config.port, e))?,
        );
    }
    let log_dir = storage::profile_directory().join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let stdout = logging::Writer::open(&log_dir.join("server-stdout.log"), logging::STREAM_LIMIT)
        .map_err(|e| e.to_string())?;
    let stderr = logging::Writer::open(&log_dir.join("server-stderr.log"), logging::STREAM_LIMIT)
        .map_err(|e| e.to_string())?;
    let mut command = Command::new(&binary);
    configure_runtime_environment(&mut command);
    for key in [
        "LLAMA_ARG_HOST",
        "LLAMA_ARG_PORT",
        "LLAMA_ARG_MODEL",
        "LLAMA_ARG_HF_REPO",
        "LLAMA_ARG_MODEL_URL",
        "LLAMA_ARG_DOCKER_REPO",
        "LLAMA_ARG_MCP_SERVERS_JSON",
        "LLAMA_ARG_MCP_SERVERS_CONFIG",
    ] {
        command.env_remove(key);
    }
    if manager.isolated {
        for (key, _) in std::env::vars_os() {
            let name = key.to_string_lossy();
            if name.starts_with("LLAMA_") || name.starts_with("MCP_") || name == "HF_TOKEN" {
                command.env_remove(key);
            }
        }
    }
    command.args(arguments(&config, model.as_deref()));
    let snapshot = if config.mcp_profile.enabled {
        let cache = storage::profile_directory().join("cache");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let snapshot = cache.join(format!("mcp-bridge-{}.bin", stamp));
        let snapshot_guard = server::Snapshot(snapshot.clone());
        mcp::write_profile(&snapshot, &config.mcp_profile)?;
        command.args([
            "--mcp-servers-json",
            &mcp::server_config(&config.mcp_profile, &snapshot)?,
        ]);
        Some(snapshot_guard)
    } else {
        None
    };
    command
        .current_dir(
            binary
                .parent()
                .ok_or("Cannot locate executable directory.")?,
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start llama-server: {}", e))?;
    let pid = child.id();
    let mut tree = match process_tree::ProcessTree::attach(&child) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let relays = match logging::Relays::start(
        child.stdout.take().ok_or("Missing server output pipe")?,
        child.stderr.take().ok_or("Missing server error pipe")?,
        stdout,
        stderr,
        credentials::values(&config),
    ) {
        Ok(relays) => relays,
        Err(error) => {
            tree.terminate(&mut child);
            let _ = child.wait();
            return Err(format!("Cannot drain server logs: {error}"));
        }
    };
    let health_host = match config.host.split(',').next().unwrap_or("127.0.0.1").trim() {
        "0.0.0.0" => "127.0.0.1",
        "::" => "[::1]",
        host if host.contains(':') => host,
        host => host,
    };
    let health_host = if health_host.contains(':') && !health_host.starts_with('[') {
        format!("[{health_host}]")
    } else {
        health_host.to_string()
    };
    manager.instance = Some(server::Instance {
        config: config.clone(),
        child,
        tree,
        _snapshot: snapshot,
        _logs: Some(relays),
        api_token,
        api_prefix,
        endpoint: if config.host.ends_with(".sock") {
            format!("unix:{}", config.host)
        } else {
            let scheme = if (!manager.isolated
                && std::env::var_os("LLAMA_ARG_SSL_KEY_FILE").is_some())
                || config
                    .argument_overrides
                    .iter()
                    .any(|o| o.flag == "--ssl-key-file")
                || config
                    .additional_args
                    .iter()
                    .any(|a| a.starts_with("--ssl-key-file"))
            {
                "https"
            } else {
                "http"
            };
            format!("{scheme}://{health_host}:{}", config.port)
        },
    });
    manager.generation = manager.generation.wrapping_add(1);
    manager.phase = "loading".into();
    Ok(format!(
        "Server process {} started. Logs: {}",
        pid,
        log_dir.display()
    ))
}
#[tauri::command]
async fn stop_server(state: State<'_, AppState>) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || server::stop(&state))
        .await
        .map_err(|e| e.to_string())?
}
#[cfg(test)]
fn stop_process(process: &Mutex<Option<Child>>) -> Result<(), String> {
    let mut process = process.lock().map_err(|e| e.to_string())?;
    if let Some(child) = process.as_mut() {
        if child.try_wait().map_err(|e| e.to_string())?.is_none() {
            child.kill().map_err(|e| e.to_string())?;
            child.wait().map_err(|e| e.to_string())?;
        }
    }
    *process = None;
    Ok(())
}
#[tauri::command]
async fn server_status(state: State<'_, AppState>) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || server::status(&state))
        .await
        .map_err(|e| e.to_string())?
}
#[cfg(test)]
fn process_status(process: &Mutex<Option<Child>>) -> Result<String, String> {
    let mut process = process.lock().map_err(|e| e.to_string())?;
    match process.as_mut() {
        Some(child) => match child.try_wait().map_err(|e| e.to_string())? {
            None => Ok("running".into()),
            Some(status) => {
                *process = None;
                Ok(format!("stopped ({})", status))
            }
        },
        None => Ok("stopped".into()),
    }
}
fn main() {
    // Portable Windows releases use a private fixed WebView2 installation.
    #[cfg(windows)]
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            let webview = directory.join("WebView2");
            if webview.join("msedgewebview2.exe").is_file() {
                std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", webview);
            }
        }
    }
    let args: Vec<String> = std::env::args().collect();
    if matches!(
        args.get(1).map(String::as_str),
        Some(
            "--hardware-report"
                | "--inspect-model"
                | "--hub-search"
                | "--hub-details"
                | "--runtime-releases"
        )
    ) {
        let result: Result<serde_json::Value, String> = tauri::async_runtime::block_on(async {
            match args[1].as_str() {
                "--hardware-report" => {
                    let mut h = monitor::hardware();
                    h.gpus = monitor::gpus().await;
                    if h.gpus.is_empty() {
                        h.gpus = monitor::adapters().await;
                    }
                    serde_json::to_value(h).map_err(|e| e.to_string())
                }
                "--inspect-model" => serde_json::to_value(gguf::inspect(
                    Path::new(args.get(2).ok_or("Supply a GGUF path")?),
                    4096,
                    1,
                )?)
                .map_err(|e| e.to_string()),
                "--hub-search" => {
                    hub::search(
                        &hub::State::default(),
                        args.get(2).map(String::as_str).unwrap_or(""),
                        "",
                    )
                    .await
                }
                "--hub-details" => {
                    hub::details(
                        &hub::State::default(),
                        args.get(2).ok_or("Supply a repository ID")?,
                    )
                    .await
                }
                _ => registry::releases().await,
            }
        });
        match result {
            Ok(v) => println!("{v}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("--benchmark-scan") {
        let started = std::time::Instant::now();
        let result = (|| {
            let directory = checked_path(args.get(2).ok_or("Supply a model folder.")?)?;
            scan_models_cancel(&directory, None, |_, _| {})
        })();
        match result {
            Ok(models) => println!(
                "{}",
                serde_json::json!({"models":models.len(),"scanMs":started.elapsed().as_secs_f64()*1000.0})
            ),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if matches!(
        args.get(1).map(String::as_str),
        Some("--probe-mcp" | "--probe-saved-mcp")
    ) {
        let result = (|| {
            let profile: mcp::Profile = if args[1] == "--probe-saved-mcp" {
                mcp::load_profile().ok_or("No saved MCP profile.")?
            } else {
                let json =
                    std::env::var("LLAMA_MCP_PROFILE").map_err(|_| "Missing profile input.")?;
                serde_json::from_str(&json).map_err(|_| "Invalid profile input.")?
            };
            validate_mcp_profile(&profile)?;
            let mut client = mcp::Client::new(profile)?;
            client.initialize()
        })();
        match result {
            Ok(report) => println!(
                "{}",
                serde_json::json!({"connected":report.connected,"server":report.server,"tools":report.tools.len(),"endpoint":report.endpoint,"protocol":report.protocol})
            ),
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("--mcp-http-bridge") {
        let result = args
            .get(2)
            .ok_or("Missing protected MCP profile path.")
            .and_then(|path| mcp::bridge(Path::new(path)).map_err(|_| "MCP HTTP bridge failed."));
        if result.is_err() {
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("--configure-mcp") {
        let result = (|| {
            let _storage_lock = storage::acquire_lock(&storage::profile_directory())?;
            let json = std::env::var("LLAMA_MCP_PROFILE").map_err(|_| "Missing profile input.")?;
            let profile: mcp::Profile =
                serde_json::from_str(&json).map_err(|_| "Invalid profile input.")?;
            validate_mcp_profile(&profile)?;
            mcp::save_profile(&profile)
        })(); // Credentials arrive through the environment, never command-line arguments.
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    let _storage_lock = match storage::acquire_lock(&storage::profile_directory()) {
        Ok(lock) => lock,
        Err(error) => {
            tauri::api::dialog::blocking::message(
                None::<&tauri::Window>,
                "Cannot open launcher data",
                error,
            );
            return;
        }
    };
    if let Err(error) = storage::initialize_portable() {
        tauri::api::dialog::blocking::message(
            None::<&tauri::Window>,
            "Cannot initialize portable storage",
            error,
        );
        return;
    }
    let mut context = tauri::generate_context!();
    let windows = context.config().tauri.windows.clone();
    context.config_mut().tauri.windows.clear();
    tauri::Builder::default()
        .manage(CloseGuard::default())
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                if event
                    .window()
                    .state::<CloseGuard>()
                    .ready
                    .load(std::sync::atomic::Ordering::SeqCst)
                    && !event
                        .window()
                        .state::<CloseGuard>()
                        .allow
                        .load(std::sync::atomic::Ordering::SeqCst)
                {
                    api.prevent_close();
                    let _ = event.window().emit("launcher-close-request", ());
                }
            }
        })
        .manage(AppState::default())
        .manage(mcp::Manager::default())
        .manage(profiles::Manager::default())
        .manage(jobs::State::default())
        .manage(monitor::State::default())
        .manage(hub::State::load())
        .manage(downloads::State::load())
        .setup(move |app| {
            for window in windows {
                tauri::WindowBuilder::from_config(app, window)
                    .data_directory(storage::profile_directory().join("webview"))
                    .build()?;
            }
            session::watch_router(app.handle(), app.state::<AppState>().inner().clone());
            monitor::start(
                app.handle(),
                app.state::<monitor::State>().inner().clone(),
                app.state::<AppState>().inner().clone(),
            );
            let recovery_app = app.handle();
            let recovery_jobs = app.state::<jobs::State>().inner().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok((id, _)) = recovery_jobs.begin("runtime-recovery") {
                    let result = tauri::async_runtime::spawn_blocking(registry::recover_installs)
                        .await
                        .map_err(|_| "Runtime recovery worker failed".to_string())
                        .and_then(|r| r)
                        .map(|_| serde_json::json!({"recovered":true}));
                    recovery_jobs.finish(&recovery_app, &id, result);
                }
            });
            let handle = app.handle();
            let state = app.state::<AppState>().inner().clone();
            std::thread::spawn(move || {
                let Ok(client) = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_millis(800))
                    .build()
                else {
                    return;
                };
                let mut previous = String::new();
                loop {
                    if state.0.lock().map_or(true, |manager| manager.shutdown) {
                        break;
                    }
                    server::check_readiness(&state, &client);
                    if let Ok(status) = server::status(&state) {
                        if status != previous {
                            let _ = handle.emit_all("server-state", &status);
                            previous = status;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_defaults,
            features::working_load,
            features::working_save,
            features::storage_location,
            features::storage_issues,
            features::storage_recover,
            features::finish_close,
            features::close_ready,
            platform_info,
            profiles_list,
            profiles_load,
            profiles_save,
            profiles_copy,
            profiles_delete,
            browse_path,
            save_config,
            load_config,
            start_server,
            stop_server,
            server_status,
            list_models,
            cancel_models,
            server_help,
            command_preview,
            server_logs,
            mcp_connect,
            mcp_test,
            mcp_disconnect,
            mcp_status,
            features::monitoring_get,
            features::monitoring_set,
            features::monitoring_history,
            features::hardware_snapshot,
            features::model_inspect,
            features::model_inspect_config,
            features::launch_preflight,
            features::open_llama_documentation,
            features::server_models,
            features::router_model,
            features::open_server_chat,
            features::jobs_list,
            features::job_cancel,
            features::inference_test,
            features::hub_account,
            features::hub_connect,
            features::hub_test,
            features::hub_disconnect,
            features::hub_search,
            features::hub_details,
            features::hub_read_card,
            features::hub_sign_in,
            features::hub_open_sign_in,
            features::open_hub_page,
            features::hub_open_tokens,
            features::download_start,
            features::downloads_list,
            features::download_resume,
            features::artifacts_list,
            features::runtime_import,
            features::runtime_releases,
            features::runtime_install,
            features::runtime_channel_plan,
            features::runtime_install_channel,
            features::runtime_remove,
            features::benchmark_start,
            features::results_export,
            features::results_list,
            features::download_remove
        ])
        .build(context)
        .expect("Unable to initialize the configurator")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<jobs::State>().shutdown();
                let monitor = app.state::<monitor::State>();
                if let Ok(mut state) = monitor.inner.lock() {
                    state.shutdown = true;
                }
                monitor.notify.notify_one();
                let benchmark = app
                    .state::<AppState>()
                    .0
                    .lock()
                    .ok()
                    .and_then(|mut m| m.benchmark_server.take());
                if let Some(benchmark) = benchmark {
                    let _ = server::stop(&benchmark);
                }
                if let Ok(mut manager) = app.state::<AppState>().0.lock() {
                    manager.shutdown = true;
                }
                let _ = server::stop(app.state::<AppState>().inner());
            }
        });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    #[test]
    fn glib_backport_string_iterator_is_sound_in_optimized_builds() {
        use glib::variant::ToVariant;
        let value = vec!["first", "middle", "last"].to_variant();
        let mut iter = value.array_iter_str().unwrap();
        assert_eq!(iter.next(), Some("first"));
        assert_eq!(iter.next_back(), Some("last"));
        assert_eq!(iter.next(), Some("middle"));
        assert_eq!(iter.next(), None);
        assert_eq!(value.array_iter_str().unwrap().nth(1), Some("middle"));
        assert_eq!(value.array_iter_str().unwrap().nth_back(1), Some("middle"));
        assert_eq!(value.array_iter_str().unwrap().last(), Some("last"));
    }
    #[test]
    fn previews_redact_inline_aliases_and_every_structured_secret_value() {
        let config = Config {
            argument_overrides: vec![ArgumentOverride {
                flag: "--api-key".into(),
                values: vec!["fixture-first".into(), "fixture-second".into()],
            }],
            additional_args: vec!["-hft=fixture-inline".into(), "--threads".into(), "8".into()],
            ..Config::default()
        };
        let preview = preview_inner(config).unwrap().join(" ");
        assert!(!preview.contains("fixture-"));
        assert!(preview.contains("-hft=[redacted]"));
        assert!(preview.contains("--threads 8"));
        let mut args = vec![
            "--api-key".into(),
            "first".into(),
            "second".into(),
            "--threads".into(),
            "8".into(),
        ];
        redact_arguments(&mut args);
        assert_eq!(
            args,
            ["--api-key", "[redacted]", "[redacted]", "--threads", "8"]
        );
    }
    #[test]
    fn rejects_missing_paths_and_traversal() {
        assert!(validate(&Config::default()).is_err());
        assert!(checked_path("C:/models/../secret.gguf").is_err());
    }
    #[test]
    fn selected_runtime_aliases_and_custom_values_receive_equal_validation() {
        let help =
            "-t, --threads N    CPU threads\n-c, --ctx-size N    Context\n--metrics    Metrics";
        let mut config = Config {
            additional_args: vec!["-t=1.5".into()],
            ..Config::default()
        };
        assert!(validate_capabilities(&config, help).is_err());
        config.additional_args = vec!["-t".into(), "4".into(), "--metrics".into()];
        assert!(validate_capabilities(&config, help).is_ok());
        config.additional_args = vec!["--future-option".into()];
        assert!(validate_capabilities(&config, help).is_err());
        config.additional_args = vec!["--ctx-size=NaN".into()];
        assert!(validate_arguments(&config).is_err());
        config.router_mode = true;
        config.additional_args = vec!["-hf=owner/model".into()];
        assert!(validate_arguments(&config).is_err());
    }
    #[test]
    fn dotted_model_preset_flags_are_preserved() {
        let config = Config {
            additional_args: vec!["--fim-qwen-1.5b-default".into()],
            ..Config::default()
        };
        assert!(validate_arguments(&config).is_ok());
        assert!(validate_capabilities(&config, "--fim-qwen-1.5b-default    Qwen preset").is_ok());
    }
    #[test]
    fn json_exports_remove_credentials_without_changing_private_profiles() {
        let mut config = Config::default();
        config.mcp_profile.bearer_token = "fixture-token".into();
        config
            .mcp_profile
            .env
            .insert("SECRET".into(), "fixture-secret".into());
        config.argument_overrides.push(ArgumentOverride {
            flag: "--api-key".into(),
            values: vec!["fixture-key".into()],
        });
        config.argument_overrides.push(ArgumentOverride {
            flag: "--threads".into(),
            values: vec!["8".into()],
        });
        config.additional_args = vec!["--hf-token=fixture-hf".into()];
        let exported = export_config(config.clone());
        let text = serde_json::to_string(&exported).unwrap();
        for secret in [
            "fixture-token",
            "fixture-secret",
            "fixture-key",
            "fixture-hf",
        ] {
            assert!(!text.contains(secret));
        }
        assert_eq!(exported.argument_overrides[0].flag, "--threads");
        assert_eq!(config.mcp_profile.bearer_token, "fixture-token");
    }
    #[test]
    fn config_roundtrip_keeps_defaults() {
        let config: Config = serde_json::from_str("{\"port\":9000}").unwrap();
        assert_eq!(config.port, 9000);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.schema_version, 3);
        let decoded: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert!(decoded.argument_overrides.is_empty());
    }
    #[test]
    fn config_migration_preserves_explicit_values_without_duplicate_defaults() {
        let config =
            decode_config(br#"{"n_ctx":8192,"temperature":0.6,"n_gpu_layers":-1,"port":9000}"#)
                .unwrap();
        assert_eq!(config.schema_version, 3);
        assert_eq!(config.argument_overrides.len(), 3);
        assert_eq!(config.argument_overrides[0].values, vec!["8192"]);
        assert_eq!(config.argument_overrides[2].values, vec!["all"]);
        assert!(decode_config(br#"{"schema_version":99}"#).is_err());
        let current =
            decode_config(br#"{"schema_version":2,"temperature":0.5,"argument_overrides":[]}"#)
                .unwrap();
        assert!(current.argument_overrides.is_empty());
    }
    #[test]
    fn router_launch_omits_fixed_model_and_uses_models_directory() {
        let config = Config {
            router_mode: true,
            models_path: "models".into(),
            ..Config::default()
        };
        let args = arguments(&config, Some(Path::new("chosen.gguf")));
        assert!(!args.contains(&"--model".to_string()));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--models-dir", "models"]));
    }
    #[test]
    fn mcp_validation_rejects_invalid_auth_headers_and_embedded_url_credentials() {
        let profile = mcp::Profile {
            url: "https://user:secret@example.com/mcp".into(),
            ..mcp::Profile::default()
        };
        assert!(validate_mcp_profile(&profile).is_err());
        let profile = mcp::Profile {
            bearer_token: "token\r\nHeader: value".into(),
            ..mcp::Profile::default()
        };
        assert!(validate_mcp_profile(&profile).is_err());
    }
    #[test]
    fn model_scan_finds_nested_models_and_only_first_split_shard() {
        let root = std::env::temp_dir().join(format!("llama-model-scan-{}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("alpha.gguf"), [1, 2, 3]).unwrap();
        fs::write(root.join("nested/beta.GGUF"), [4]).unwrap();
        fs::write(root.join("split-00001-of-00002.gguf"), []).unwrap();
        fs::write(root.join("split-00002-of-00002.gguf"), []).unwrap();
        fs::write(root.join("not-a-model.txt"), []).unwrap();
        let models = scan_models(&root).unwrap();
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].name, "alpha.gguf");
        assert_eq!(models[0].size, 3);
        assert!(models.iter().any(|m| m.name.contains("beta.GGUF")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn all_options_are_passed_as_arguments_without_shell_parsing() {
        let config = Config {
            argument_overrides: vec![
                ArgumentOverride {
                    flag: "--lora".into(),
                    values: vec![r"D:\Models & adapters\my adapter.gguf".into()],
                },
                ArgumentOverride {
                    flag: "--no-kv-offload".into(),
                    values: vec![],
                },
            ],
            additional_args: vec![
                "--control-vector-layer-range".into(),
                "0".into(),
                "32".into(),
            ],
            ..Config::default()
        };
        validate_arguments(&config).unwrap();
        let args = arguments(&config, Some(Path::new("model.gguf")));
        assert!(args.contains(&r"D:\Models & adapters\my adapter.gguf".to_string()));
        assert_eq!(
            &args[args.len() - 3..],
            ["--control-vector-layer-range", "0", "32"]
        );
        assert!(!args.contains(&"--temp".to_string()));
    }
    #[test]
    fn overrides_cannot_bypass_host_port_model_validation() {
        let mut config = Config {
            additional_args: vec!["--port=80".into()],
            ..Config::default()
        };
        assert!(validate_arguments(&config).is_err());
        config.additional_args.clear();
        config.argument_overrides = vec![ArgumentOverride {
            flag: "--ctx-size".into(),
            values: vec!["NaN".into()],
        }];
        assert!(validate_arguments(&config).is_err());
    }
    #[test]
    fn stop_reaps_the_owned_process_and_updates_status() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "tests::process_lifecycle_fixture", "--ignored"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let process = Mutex::new(Some(command.spawn().unwrap()));
        assert_eq!(process_status(&process).unwrap(), "running");
        stop_process(&process).unwrap();
        assert_eq!(process_status(&process).unwrap(), "stopped");
        stop_process(&process).unwrap();
    }
    #[test]
    #[ignore = "Fixture run only as a subprocess by stop_reaps_the_owned_process_and_updates_status"]
    fn process_lifecycle_fixture() {
        std::thread::sleep(std::time::Duration::from_secs(120));
    }
}
