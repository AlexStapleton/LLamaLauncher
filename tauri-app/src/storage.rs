use crate::mcp::Profile;
#[cfg(unix)]
use std::io::Write;
use std::{
    fs,
    path::{Path, PathBuf},
};
fn profile_path() -> PathBuf {
    #[cfg(not(windows))]
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "Cannot create MCP profile directory.")?;
    }
    let bytes = serde_json::to_vec(profile).map_err(|_| "Cannot serialize MCP profile.")?;
    if bytes.len() > 256 * 1024 {
        return Err("MCP profile exceeded 256 KiB.".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(parent) = path.parent() {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .map_err(|_| "Cannot restrict MCP profile directory.")?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| "Cannot save private MCP profile.")?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot restrict MCP profile permissions.")?;
        file.write_all(&bytes)
            .map_err(|_| "Cannot write MCP profile.".into())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, protect(&bytes, false)?)
            .map_err(|_| "Cannot save protected MCP profile.".into())
    }
}
pub fn read_profile(path: &Path) -> Result<Profile, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "Cannot read protected MCP profile.")?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read protected MCP profile.")?;
    if bytes.len() > 1024 * 1024 {
        return Err("Protected MCP profile exceeded 1 MiB.".into());
    }
    serde_json::from_slice(&protect(&bytes, true)?).map_err(|_| "Cannot decode MCP profile.".into())
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
    file.write_all(&protect(bytes, false)?)
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
    Ok(())
}
pub fn read_private(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "Cannot read profile library.")?
        .take(limit as u64 + 65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read profile library.")?;
    if bytes.len() > limit + 65536 {
        return Err("Profile library exceeded its size limit.".into());
    }
    let plain = protect(&bytes, true)?;
    if plain.len() > limit {
        return Err("Profile library exceeded its size limit.".into());
    }
    Ok(plain)
}
