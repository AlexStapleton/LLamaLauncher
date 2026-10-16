use crate::mcp::Profile;
use std::sync::{Mutex, OnceLock};
use std::{
    fs,
    path::{Path, PathBuf},
};
type Faults = Mutex<std::collections::BTreeMap<PathBuf, String>>;
static FAULTS: OnceLock<Faults> = OnceLock::new();
static WRITES: Mutex<()> = Mutex::new(());
const RECORDS: [&str; 7] = [
    "working-settings.bin",
    "launch-profiles.bin",
    "mcp-profile.bin",
    "hugging-face.bin",
    "monitoring.bin",
    "downloads.bin",
    "artifacts.bin",
];
pub fn damaged(path: &Path, reason: &str) -> String {
    let message = format!(
        "{}: {reason}. Original preserved; use Storage recovery.",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    if let Ok(mut faults) = FAULTS.get_or_init(Default::default).lock() {
        faults.insert(path.into(), message.clone());
    }
    message
}
pub fn issues() -> Vec<serde_json::Value> {
    let paths = RECORDS.map(crate::locations::record);
    for (name, path) in RECORDS.into_iter().zip(&paths) {
        if path.exists() {
            if let Err(error) = read_private(path, record_limit(path))
                .and_then(|bytes| validate_recovery(name, &bytes))
            {
                damaged(path, &error);
            }
        }
    }
    FAULTS.get_or_init(Default::default).lock().map(|f|f.iter().filter(|(p,_)|paths.contains(p)).map(|(p,m)|serde_json::json!({"name":p.file_name().unwrap_or_default().to_string_lossy(),"message":m,"backup":p.with_extension("bin.bak").is_file()})).collect()).unwrap_or_default()
}
pub fn load_optional<T: serde::de::DeserializeOwned>(
    path: &Path,
    limit: usize,
) -> Result<Option<T>, String> {
    match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(damaged(path, "Cannot access saved record")),
        Ok(_) => {}
    }
    serde_json::from_slice(&read_private(path, limit)?)
        .map(Some)
        .map_err(|_| damaged(path, "Cannot decode saved record"))
}
fn legacy_profile_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        crate::locations::config().join("mcp-profile.bin")
    }
    #[cfg(all(not(windows), not(target_os = "linux")))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
                    .join(".config")
            });
        base.join("llama-launcher").join("mcp-profile.bin")
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_else(|| ".".into());
        PathBuf::from(base)
            .join("LlamaServerConfigurator")
            .join("mcp-profile.bin")
    }
}
pub fn executable_directory() -> PathBuf {
    #[cfg(windows)]
    if let Some(executable) = portable_executable() {
        if let Some(parent) = executable.parent() {
            return parent.into();
        }
    }
    #[cfg(unix)]
    if let Some(image) = std::env::var_os("APPIMAGE").map(PathBuf::from) {
        if image.is_absolute() {
            if let Some(parent) = image.parent() {
                return parent.into();
            }
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}
#[cfg(windows)]
pub fn portable_executable() -> Option<PathBuf> {
    std::env::var_os("LLAMA_LAUNCHER_PORTABLE_EXE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_file())
}
fn profile_path() -> PathBuf {
    crate::locations::record("mcp-profile.bin")
}

/// Hold this handle for the entire GUI lifetime. Atomic writes alone cannot
/// prevent two processes from replacing each other's read-modify-write state.
pub fn acquire_lock(root: &Path) -> Result<fs::File, String> {
    acquire_named_lock(root, "launcher.lock")
}
pub fn acquire_named_lock(root: &Path, name: &str) -> Result<fs::File, String> {
    fs::create_dir_all(root).map_err(|_| "The launcher data folder is not writable")?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join(name))
        .map_err(|_| "Cannot open the launcher data lock")?;
    file.try_lock().map_err(|_| "This launcher data folder is already open in another instance, or cannot be locked. Close the other instance before opening it again.")?;
    Ok(file)
}

/// Copy legacy saved records once. Imported executables/models keep their original paths.
pub fn initialize_portable() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if std::env::var_os("LLAMA_LAUNCHER_DATA_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .is_none()
    {
        return initialize_xdg();
    }
    let root = profile_directory();
    fs::create_dir_all(&root).map_err(|_|"The saved-data folder is not writable. Choose a writable location for the launcher or its data override.")?;
    let marker = root.join("storage-version.txt");
    if marker.exists() {
        return Ok(());
    }
    if std::env::var_os("LLAMA_LAUNCHER_DATA_DIR").is_none() {
        let legacy = legacy_profile_path()
            .parent()
            .ok_or("Invalid legacy storage location")?
            .to_path_buf();
        for name in [
            "launch-profiles.bin",
            "mcp-profile.bin",
            "hugging-face.bin",
            "monitoring.bin",
            "artifacts.bin",
            "downloads.bin",
        ] {
            let from = legacy.join(name);
            let to = root.join(name);
            if from.is_file() && !to.exists() {
                match read_private(&from, 4 * 1024 * 1024) {
                    Ok(bytes) => write_private_atomic(&to, &bytes)?,
                    Err(_) => {
                        let bytes =
                            fs::read(&from).map_err(|_| "Cannot preserve legacy saved record")?;
                        atomic_bytes(&to, &bytes)?;
                        damaged(
                            &to,
                            "Legacy record could not be decoded; its original was retained",
                        );
                    }
                }
            }
        }
    }
    fs::write(marker, b"portable-v1\n")
        .map_err(|_| "Cannot record portable storage initialization".into())
}

const RELATIVE_PREFIX: &str = "llama-launcher-relative:/";
#[cfg(target_os = "linux")]
fn initialize_xdg() -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;
    for directory in [
        crate::locations::config(),
        crate::locations::data(),
        crate::locations::state(),
        crate::locations::cache(),
    ] {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
            .map_err(|_| "Cannot create the application's XDG directories")?;
    }
    let marker = crate::locations::config().join("xdg-layout-v1");
    if marker.exists() {
        return Ok(());
    }
    // Copy only bounded metadata. Existing model/runtime files and originals
    // stay where they are; decoded absolute references continue to find them.
    for previous in [
        executable_directory().join("data"),
        legacy_profile_path()
            .parent()
            .ok_or("Invalid legacy location")?
            .into(),
    ] {
        let _previous_lock =
            if previous != profile_directory() && previous.join("launcher.lock").is_file() {
                let lock = fs::File::open(previous.join("launcher.lock"))
                    .map_err(|_| "Cannot read the previous launcher's lock")?;
                lock.try_lock()
                    .map_err(|_| "Close the previous launcher before importing its saved data")?;
                Some(lock)
            } else {
                None
            };
        for name in RECORDS {
            let from = previous.join(name);
            let to = crate::locations::record(name);
            if from == to || to.exists() || !from.is_file() {
                continue;
            }
            migrate_record(&from, &to, &previous, name)?;
            let backup = from.with_extension("bin.bak");
            let target_backup = to.with_extension("bin.bak");
            if backup.is_file() && !target_backup.exists() {
                migrate_record(&backup, &target_backup, &previous, name)?;
            }
        }
    }
    atomic_bytes(&marker, b"xdg-v1\n")
}

#[cfg(target_os = "linux")]
fn migrate_record(from: &Path, to: &Path, previous: &Path, name: &str) -> Result<(), String> {
    let limit = record_limit(Path::new(name));
    match read_private_at(from, limit, previous)
        .and_then(|bytes| validate_recovery(name, &bytes).map(|_| bytes))
    {
        Ok(mut bytes) => {
            if name == "artifacts.bin" {
                let mut artifacts: Vec<crate::registry::Artifact> = serde_json::from_slice(&bytes)
                    .map_err(|_| "Cannot import the previous artifact registry")?;
                for artifact in &mut artifacts {
                    if artifact.kind == "runtime"
                        && !Path::new(&artifact.path).starts_with(crate::registry::runtimes_dir())
                    {
                        // Retained old installations are now external imports:
                        // removal can forget their record, never their files.
                        artifact.managed = false;
                    }
                }
                bytes = serde_json::to_vec(&artifacts)
                    .map_err(|_| "Cannot encode the imported artifact registry")?;
            }
            write_private_atomic(to, &bytes)
        }
        Err(_) => {
            use std::io::Read;
            let mut original = Vec::new();
            fs::File::open(from)
                .and_then(|file| file.take(limit as u64 + 66049).read_to_end(&mut original))
                .map_err(|_| "Cannot preserve a previous saved record; original retained")?;
            if original.len() > limit + 66048 {
                return Err("Previous saved record exceeds its limit; original retained".into());
            }
            atomic_bytes(to, &original)?;
            damaged(
                to,
                "Previous record could not be decoded; original retained",
            );
            Ok(())
        }
    }
}

fn translate_paths(value: &mut serde_json::Value, root: &Path, encode: bool) {
    match value {
        serde_json::Value::String(text) => {
            if encode {
                let path = Path::new(text.trim_start_matches(r"\\?\"));
                if let Ok(relative) = path.strip_prefix(root) {
                    if !relative.as_os_str().is_empty()
                        && relative
                            .components()
                            .all(|c| matches!(c, std::path::Component::Normal(_)))
                    {
                        let relative = relative.to_string_lossy();
                        #[cfg(windows)]
                        let relative = relative.replace('\\', "/");
                        *text = format!("{RELATIVE_PREFIX}{relative}");
                    }
                }
            } else if let Some(relative) = text.strip_prefix(RELATIVE_PREFIX) {
                let path = Path::new(relative);
                if !relative.is_empty()
                    && path
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                {
                    *text = root.join(path).to_string_lossy().into();
                }
            }
        }
        serde_json::Value::Array(items) => items
            .iter_mut()
            .for_each(|v| translate_paths(v, root, encode)),
        serde_json::Value::Object(items) => items
            .values_mut()
            .for_each(|v| translate_paths(v, root, encode)),
        _ => {}
    }
}

fn portable_bytes(bytes: &[u8], encode: bool) -> Vec<u8> {
    portable_bytes_at(bytes, &crate::locations::data(), encode)
}
fn portable_bytes_at(bytes: &[u8], root: &Path, encode: bool) -> Vec<u8> {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return bytes.to_vec();
    };
    translate_paths(&mut value, root, encode);
    serde_json::to_vec(&value).unwrap_or_else(|_| bytes.to_vec())
}
#[cfg(windows)]
pub(crate) fn protect(data: &[u8], decrypt: bool) -> Result<Vec<u8>, String> {
    #[repr(C)]
    struct Blob {
        length: u32,
        data: *mut u8,
    }
    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            input: *const Blob,
            description: *const u16,
            entropy: *const Blob,
            reserved: *mut std::ffi::c_void,
            prompt: *mut std::ffi::c_void,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
        fn CryptUnprotectData(
            input: *const Blob,
            description: *mut *mut u16,
            entropy: *const Blob,
            reserved: *mut std::ffi::c_void,
            prompt: *mut std::ffi::c_void,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }
    let input = Blob {
        length: data
            .len()
            .try_into()
            .map_err(|_| "MCP profile too large.")?,
        data: data.as_ptr() as *mut u8,
    };
    let mut output = Blob {
        length: 0,
        data: std::ptr::null_mut(),
    };
    unsafe {
        let result = if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        };
        if result == 0 {
            return Err(
                "Windows could not protect or unlock the MCP profile for this account.".into(),
            );
        }
        let value = std::slice::from_raw_parts(output.data, output.length as usize).to_vec();
        LocalFree(output.data.cast());
        Ok(value)
    }
}
#[cfg(not(windows))]
pub(crate) fn protect(data: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}
pub fn save_profile(profile: &Profile) -> Result<(), String> {
    write_profile(&profile_path(), profile)
}
pub fn write_profile(path: &Path, profile: &Profile) -> Result<(), String> {
    let bytes = serde_json::to_vec(profile).map_err(|_| "Cannot serialize MCP profile.")?;
    if bytes.len() > 256 * 1024 {
        return Err("MCP profile exceeded 256 KiB.".into());
    }
    write_private_atomic(path, &bytes)
}
pub fn read_profile(path: &Path) -> Result<Profile, String> {
    load_optional(path, 256 * 1024)?.ok_or("No saved MCP profile".into())
}
pub fn load_profile() -> Option<Profile> {
    read_profile(&profile_path()).ok()
}
pub fn profile_directory() -> PathBuf {
    profile_path()
        .parent()
        .expect("Profile path has parent")
        .to_path_buf()
}

/// Private, atomic replacement: failed writes leave the previous library intact.
pub fn write_private_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let _write = WRITES.lock().map_err(|_| "Storage writer unavailable")?;
    if let Some(error) = FAULTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Storage recovery unavailable")?
        .get(path)
    {
        return Err(error.clone());
    }
    let limit = record_limit(path);
    if bytes.len() > limit {
        return Err("Saved record exceeds its size limit; previous record preserved".into());
    }
    if path.exists() {
        // Only a successfully decoded record may become the last-known backup.
        let previous = read_private(path, limit)?;
        if serde_json::from_slice::<serde_json::Value>(&previous).is_err() {
            return Err(damaged(path, "Cannot replace a damaged saved record"));
        }
        atomic_bytes(
            &path.with_extension("bin.bak"),
            &fs::read(path).map_err(|_| "Cannot back up saved record")?,
        )?;
    }
    let portable = portable_bytes(bytes, true);
    let plain = match serde_json::from_slice::<serde_json::Value>(&portable) {
        Ok(payload) => {
            serde_json::to_vec(&serde_json::json!({"_launcher_record_version":1,"payload":payload}))
                .map_err(|_| "Cannot encode saved record")?
        }
        Err(_) => portable,
    };
    atomic_bytes(path, &protect(&plain, false)?)
}
fn record_limit(path: &Path) -> usize {
    match path.file_name().unwrap_or_default().to_str().unwrap_or("") {
        "working-settings.bin" => 1024 * 1024,
        "mcp-profile.bin" => 256 * 1024,
        "hugging-face.bin" => 65536,
        "monitoring.bin" => 4096,
        _ => 4 * 1024 * 1024,
    }
}
/// Publish a new file/folder on the same filesystem without replacing an existing target.
pub fn publish_new(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let source: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 8) } == 0 {
            return Err("Cannot publish file; destination preserved".into());
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let source = std::ffi::CString::new(from.as_os_str().as_bytes())
            .map_err(|_| "Invalid source path")?;
        let target =
            std::ffi::CString::new(to.as_os_str().as_bytes()).map_err(|_| "Invalid target path")?;
        if unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        } != 0
        {
            return Err("Cannot publish file; destination preserved".into());
        }
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        if from.is_dir() {
            return Err("Atomic folder publication is supported on Windows and Linux".into());
        }
        fs::hard_link(from, to).map_err(|_| "Cannot publish file; destination preserved")?;
        fs::remove_file(from).map_err(|_| "Cannot remove published partial")?;
    }
    #[cfg(unix)]
    for directory in [from.parent(), to.parent()].into_iter().flatten() {
        fs::File::open(directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| "Cannot flush published directory")?;
    }
    Ok(())
}
pub fn write_public_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Public record exceeds 4 MiB".into());
    }
    let _write = WRITES.lock().map_err(|_| "Storage writer unavailable")?;
    atomic_bytes(path, bytes)
}
fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let directory = path.parent().ok_or("Missing private storage directory.")?;
    fs::create_dir_all(directory).map_err(|_| "Cannot create private storage directory.")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| "Cannot restrict private storage directory.")?;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Invalid clock.")?
        .as_nanos();
    let temporary = directory.join(format!("profiles-{}-{stamp}.tmp", std::process::id()));
    let _guard = crate::server::Snapshot(temporary.clone());
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| "Cannot create private profile file.")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot write private profiles.")?;
    drop(file);
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
            return Err(
                "Cannot replace private profiles. Previous profiles were preserved.".into(),
            );
        }
    }
    #[cfg(not(windows))]
    fs::rename(&temporary, path)
        .map_err(|_| "Cannot replace private profiles. Previous profiles were preserved.")?;
    #[cfg(unix)]
    fs::File::open(directory)
        .and_then(|f| f.sync_all())
        .map_err(|_| "Cannot flush saved record directory")?;
    Ok(())
}
pub fn read_private(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let result = read_private_inner(path, limit);
    if let Err(error) = &result {
        if path.exists() {
            return Err(damaged(path, error));
        }
    }
    result
}
fn read_private_inner(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    read_private_at(path, limit, &crate::locations::data())
}
fn read_private_at(path: &Path, limit: usize, root: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "Cannot read profile library.")?
        .take(limit as u64 + 66049)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read profile library.")?;
    if bytes.len() > limit + 66048 {
        return Err("Profile library exceeded its size limit.".into());
    }
    let mut plain = protect(&bytes, true)?;
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&plain) {
        if let Some(version) = value.get("_launcher_record_version") {
            if version.as_u64() != Some(1) {
                return Err("Saved record requires a newer launcher".into());
            }
            plain = serde_json::to_vec(value.get("payload").ok_or("Missing saved record payload")?)
                .map_err(|_| "Cannot decode saved record")?;
        }
    }
    let plain = portable_bytes_at(&plain, root, false);
    if plain.len() > limit {
        return Err("Profile library exceeded its size limit.".into());
    }
    Ok(plain)
}

fn validate_recovery(name: &str, bytes: &[u8]) -> Result<(), String> {
    let decode = || "Saved record has invalid or unsupported contents".to_string();
    match name {
        "working-settings.bin" => {
            let config: crate::config::Config =
                serde_json::from_slice(bytes).map_err(|_| decode())?;
            if config.schema_version > 3 {
                return Err(decode());
            }
        }
        "launch-profiles.bin" => {
            let library: crate::profiles::Library =
                serde_json::from_slice(bytes).map_err(|_| decode())?;
            if library.version != 1
                || library.profiles.len() > 128
                || library.profiles.iter().any(|p| p.config.schema_version > 3)
            {
                return Err(decode());
            }
        }
        "mcp-profile.bin" => {
            let _: crate::mcp::Profile = serde_json::from_slice(bytes).map_err(|_| decode())?;
        }
        "hugging-face.bin" => {
            let _: crate::hub::Account = serde_json::from_slice(bytes).map_err(|_| decode())?;
        }
        "monitoring.bin" => {
            let p: crate::monitor::MonitoringPreferences =
                serde_json::from_slice(bytes).map_err(|_| decode())?;
            if !(1..=60).contains(&p.interval_seconds) {
                return Err(decode());
            }
        }
        "downloads.bin" => {
            let plans: Vec<crate::downloads::DownloadPlan> =
                serde_json::from_slice(bytes).map_err(|_| decode())?;
            if plans.len() > 128
                || plans
                    .iter()
                    .any(|p| crate::downloads::validate_plan(p).is_err())
            {
                return Err(decode());
            }
        }
        "artifacts.bin" => {
            let rows: Vec<crate::registry::Artifact> =
                serde_json::from_slice(bytes).map_err(|_| decode())?;
            if rows.len() > 1024 {
                return Err(decode());
            }
        }
        _ => return Err("Unknown record".into()),
    }
    Ok(())
}
/// Explicit recovery archives the original protected bytes before changing state.
pub fn recover(name: &str, action: &str) -> Result<(), String> {
    recover_record(&crate::locations::record(name), name, action)
}
fn recover_record(path: &Path, name: &str, action: &str) -> Result<(), String> {
    if ![
        "working-settings.bin",
        "launch-profiles.bin",
        "mcp-profile.bin",
        "hugging-face.bin",
        "monitoring.bin",
        "downloads.bin",
        "artifacts.bin",
    ]
    .contains(&name)
    {
        return Err("Unknown saved record".into());
    }
    let path = path.to_owned();
    let backup = path.with_extension("bin.bak");
    if action == "retry" {
        let bytes = read_private_inner(&path, record_limit(&path))?;
        validate_recovery(name, &bytes)?;
    } else {
        let restored = if action == "restore" {
            let bytes = read_private_inner(&backup, record_limit(&path))?;
            validate_recovery(name, &bytes)?;
            Some(bytes)
        } else if action == "reset" {
            None
        } else {
            return Err("Unknown recovery action".into());
        };
        if path.exists() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "Invalid clock")?
                .as_nanos();
            fs::rename(
                &path,
                path.with_file_name(format!("{name}.preserved-{stamp}")),
            )
            .map_err(|_| "Cannot preserve original saved record")?;
        }
        FAULTS
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| "Storage recovery unavailable")?
            .remove(&path);
        if let Some(bytes) = restored {
            if let Err(error) = write_private_atomic(&path, &bytes) {
                return Err(damaged(&path, &format!("Restore failed: {error}")));
            }
        }
    }
    FAULTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Storage recovery unavailable")?
        .remove(&path);
    Ok(())
}

#[cfg(test)]
mod portable_tests {
    use super::*;
    #[test]
    fn explicit_restore_uses_valid_backup_and_preserves_damaged_original() {
        let root = std::env::temp_dir().join(format!("restore-fixture-{}", std::process::id()));
        let path = root.join("monitoring.bin");
        write_private_atomic(&path, br#"{"enabled":false,"interval_seconds":2}"#).unwrap();
        write_private_atomic(&path, br#"{"enabled":true,"interval_seconds":3}"#).unwrap();
        atomic_bytes(&path, &protect(b"truncated", false).unwrap()).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(load_optional::<crate::monitor::MonitoringPreferences>(&path, 4096).is_err());
        assert!(recover_record(&path, "monitoring.bin", "retry").is_err());
        recover_record(&path, "monitoring.bin", "restore").unwrap();
        let restored: crate::monitor::MonitoringPreferences =
            load_optional(&path, 4096).unwrap().unwrap();
        assert!(!restored.enabled);
        assert_eq!(restored.interval_seconds, 2);
        let preserved = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().contains(".preserved-"))
            .unwrap();
        assert_eq!(fs::read(preserved.path()).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn new_publication_cannot_overwrite_an_existing_file() {
        let root = std::env::temp_dir().join(format!("publish-fixture-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let from = root.join("part");
        let to = root.join("model");
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"keep").unwrap();
        assert!(publish_new(&from, &to).is_err());
        assert_eq!(fs::read(&to).unwrap(), b"keep");
        assert!(from.exists());
        fs::remove_file(&to).unwrap();
        publish_new(&from, &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn denied_backup_publication_preserves_the_previous_record() {
        let root = std::env::temp_dir().join(format!("backup-failure-{}", std::process::id()));
        let path = root.join("monitoring.bin");
        write_private_atomic(&path, br#"{"enabled":false,"interval_seconds":2}"#).unwrap();
        let original = fs::read(&path).unwrap();
        fs::create_dir(path.with_extension("bin.bak")).unwrap();
        assert!(write_private_atomic(&path, br#"{"enabled":true,"interval_seconds":3}"#).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn damaged_records_cannot_replace_their_protected_backup_or_original() {
        let root = std::env::temp_dir().join(format!("storage-fault-{}", std::process::id()));
        let path = root.join("monitoring.bin");
        write_private_atomic(&path, br#"{"enabled":false,"interval_seconds":2}"#).unwrap();
        write_private_atomic(&path, br#"{"enabled":true,"interval_seconds":3}"#).unwrap();
        let backup = path.with_extension("bin.bak");
        let saved = fs::read(&backup).unwrap();
        atomic_bytes(&path, &protect(b"not-json", false).unwrap()).unwrap();
        assert!(load_optional::<crate::monitor::MonitoringPreferences>(&path, 4096).is_err());
        let original = fs::read(&path).unwrap();
        assert!(write_private_atomic(&path, br#"{"enabled":true,"interval_seconds":9}"#).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(&backup).unwrap(), saved);
        let restored = read_private_inner(&backup, 4096).unwrap();
        validate_recovery("monitoring.bin", &restored).unwrap();
        assert!(String::from_utf8(restored).unwrap().contains("false"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn future_record_versions_and_oversized_updates_preserve_existing_data() {
        let root = std::env::temp_dir().join(format!("storage-version-{}", std::process::id()));
        let path = root.join("monitoring.bin");
        write_private_atomic(&path, br#"{"enabled":true,"interval_seconds":2}"#).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(write_private_atomic(&path, &vec![b'x'; 4097]).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        atomic_bytes(
            &path,
            &protect(br#"{"_launcher_record_version":99,"payload":{}}"#, false).unwrap(),
        )
        .unwrap();
        let original = fs::read(&path).unwrap();
        assert!(read_private(&path, 4096).is_err());
        assert!(write_private_atomic(&path, b"{}").is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn data_lock_excludes_other_processes_and_releases_on_close() {
        let root = std::env::temp_dir().join(format!("launcher-lock-{}", std::process::id()));
        let lock = acquire_lock(&root).unwrap();
        let probe = |blocked: bool| {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "storage::portable_tests::lock_fixture",
                    "--ignored",
                ])
                .env("LLAMA_TEST_LOCK_PATH", &root)
                .env("LLAMA_TEST_LOCK_BLOCKED", blocked.to_string())
                .status()
                .unwrap();
            assert!(status.success());
        };
        probe(true);
        drop(lock);
        probe(false);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "Cross-process lock fixture"]
    fn lock_fixture() {
        let root = PathBuf::from(std::env::var_os("LLAMA_TEST_LOCK_PATH").unwrap());
        let blocked = std::env::var("LLAMA_TEST_LOCK_BLOCKED").unwrap() == "true";
        assert_eq!(acquire_lock(&root).is_err(), blocked);
    }
    #[test]
    fn moving_the_folder_rebases_managed_paths_and_preserves_external_paths() {
        let old = std::env::temp_dir().join("launcher-old/data");
        let new = std::env::temp_dir().join("launcher-moved/data");
        let mut value = serde_json::json!({"runtime":old.join("runtimes/build/llama-server").to_string_lossy(),"model":old.join("models/test.gguf").to_string_lossy(),"external":"/another-drive/model.gguf","token":"fixture-private-token"});
        translate_paths(&mut value, &old, true);
        assert!(value["runtime"]
            .as_str()
            .unwrap()
            .starts_with(RELATIVE_PREFIX));
        translate_paths(&mut value, &new, false);
        assert_eq!(
            value["runtime"],
            new.join("runtimes/build/llama-server")
                .to_string_lossy()
                .as_ref()
        );
        assert_eq!(
            value["model"],
            new.join("models/test.gguf").to_string_lossy().as_ref()
        );
        assert_eq!(value["external"], "/another-drive/model.gguf");
        assert_eq!(value["token"], "fixture-private-token");
        let mut escape = serde_json::json!(format!("{RELATIVE_PREFIX}../escape"));
        translate_paths(&mut escape, &new, false);
        assert!(escape.as_str().unwrap().starts_with(RELATIVE_PREFIX));
    }
}

#[cfg(all(test, target_os = "linux"))]
mod xdg_tests {
    use super::*;
    #[test]
    #[ignore = "Isolated XDG fixture launched by migration test"]
    fn layout_fixture() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = PathBuf::from(std::env::var_os("LLAMA_XDG_FIXTURE").unwrap());
        let old = fixture.join("application/data");
        assert_eq!(profile_directory(), fixture.join("config/llama-launcher"));
        let _lock = acquire_lock(&profile_directory()).unwrap();
        initialize_portable().unwrap();
        let config: crate::config::Config = load_optional(
            &crate::locations::record("working-settings.bin"),
            1024 * 1024,
        )
        .unwrap()
        .unwrap();
        assert_eq!(config.models_path, old.join("models").to_string_lossy());
        assert_eq!(
            config.model_file,
            old.join("models/example.gguf").to_string_lossy()
        );
        assert!(Path::new(&config.model_file).is_file());
        assert_eq!(
            crate::locations::models(),
            fixture.join("data/llama-launcher/models")
        );
        assert_eq!(
            crate::registry::runtimes_dir(),
            fixture.join("data/llama-launcher/runtimes")
        );
        assert_eq!(
            crate::locations::record("downloads.bin"),
            fixture.join("state/llama-launcher/downloads.bin")
        );
        assert_eq!(
            crate::locations::probes(),
            fixture.join("cache/llama-launcher/probes")
        );
        let registry = crate::registry::list().unwrap();
        assert!(
            !registry[0].managed,
            "Retained external runtimes must not be deleted as new managed installs"
        );
        assert_eq!(
            registry[0].path,
            old.join("runtimes/build/llama-server").to_string_lossy()
        );
        let queue: Vec<crate::downloads::DownloadPlan> =
            load_optional(&crate::locations::record("downloads.bin"), 4 * 1024 * 1024)
                .unwrap()
                .unwrap();
        assert!(queue.is_empty());
        let monitoring: crate::monitor::MonitoringPreferences =
            load_optional(&crate::locations::record("monitoring.bin"), 4096)
                .unwrap()
                .unwrap();
        assert_eq!(
            monitoring.interval_seconds, 7,
            "Keep an existing XDG preference"
        );
        assert!(issues()
            .iter()
            .any(|issue| issue["name"] == "mcp-profile.bin"));
        assert!(load_profile().is_none());
        recover("mcp-profile.bin", "restore").unwrap();
        assert_eq!(
            load_profile().unwrap().cwd,
            old.join("models").to_string_lossy()
        );
        assert!(fs::read_dir(profile_directory())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains(".preserved-")));
        assert_eq!(
            fs::metadata(crate::locations::record("working-settings.bin"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(profile_directory())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        // The marker prevents re-importing a previous source over newer settings.
        fs::write(old.join("monitoring.bin"), b"broken old preference").unwrap();
        initialize_portable().unwrap();
        assert_eq!(
            fs::read(crate::locations::record("monitoring.bin")).unwrap(),
            fs::read(fixture.join("monitor-copy")).unwrap()
        );
    }
    #[test]
    fn xdg_migration_preserves_paths_originals_backups_and_existing_preferences() {
        let fixture = std::env::temp_dir().join(format!(
            "llama-xdg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let old = fixture.join("application/data");
        fs::create_dir_all(old.join("models")).unwrap();
        fs::create_dir_all(old.join("runtimes/build")).unwrap();
        fs::create_dir_all(fixture.join("config/llama-launcher")).unwrap();
        fs::write(
            fixture.join("application/launcher.AppImage"),
            b"synthetic path marker, never executed",
        )
        .unwrap();
        fs::write(
            fixture.join("config/user-dirs.dirs"),
            format!(
                "XDG_DOWNLOAD_DIR=\"{}\"\n",
                fixture.join("Localized downloads").display()
            ),
        )
        .unwrap();
        let preferences = br#"{"enabled":false,"interval_seconds":7}"#;
        fs::write(
            fixture.join("config/llama-launcher/monitoring.bin"),
            preferences,
        )
        .unwrap();
        fs::write(fixture.join("monitor-copy"), preferences).unwrap();
        fs::write(old.join("launcher.lock"), b"").unwrap();
        fs::write(
            old.join("models/example.gguf"),
            b"synthetic model path fixture, not weights",
        )
        .unwrap();
        let config = crate::config::Config {
            models_path: format!("{RELATIVE_PREFIX}models"),
            model_file: format!("{RELATIVE_PREFIX}models/example.gguf"),
            ..Default::default()
        };
        let saved =
            serde_json::to_vec(&serde_json::json!({"_launcher_record_version":1,"payload":config}))
                .unwrap();
        fs::write(old.join("working-settings.bin"), &saved).unwrap();
        fs::write(old.join("downloads.bin"), b"[]").unwrap();
        fs::write(
            old.join("monitoring.bin"),
            br#"{"enabled":true,"interval_seconds":3}"#,
        )
        .unwrap();
        let registry = serde_json::json!([{"id":"runtime-fixture","kind":"runtime","name":"fixture","path":format!("{RELATIVE_PREFIX}runtimes/build/llama-server"),"version":"fixture","managed":true,"details":{}}]);
        fs::write(
            old.join("artifacts.bin"),
            serde_json::to_vec(&registry).unwrap(),
        )
        .unwrap();
        fs::write(old.join("mcp-profile.bin"), b"damaged fixture").unwrap();
        let profile = Profile {
            cwd: format!("{RELATIVE_PREFIX}models"),
            ..Default::default()
        };
        fs::write(
            old.join("mcp-profile.bin.bak"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "storage::xdg_tests::layout_fixture", "--ignored"])
            .env_remove("LLAMA_LAUNCHER_DATA_DIR")
            .env("LLAMA_XDG_FIXTURE", &fixture)
            .env("APPIMAGE", fixture.join("application/launcher.AppImage"))
            .env("XDG_CONFIG_HOME", fixture.join("config"))
            .env("XDG_DATA_HOME", fixture.join("data"))
            .env("XDG_STATE_HOME", fixture.join("state"))
            .env("XDG_CACHE_HOME", fixture.join("cache"))
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(fs::read(old.join("working-settings.bin")).unwrap(), saved);
        assert_eq!(
            fs::read(old.join("mcp-profile.bin")).unwrap(),
            b"damaged fixture"
        );
        assert!(old.join("models/example.gguf").is_file());
        fs::remove_dir_all(fixture).unwrap();
    }
}
