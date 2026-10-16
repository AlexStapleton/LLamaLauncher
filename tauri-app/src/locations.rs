//! Platform storage policy. The explicit override keeps native fixtures isolated.
use std::path::PathBuf;

fn override_root() -> Option<PathBuf> {
    std::env::var_os("LLAMA_LAUNCHER_DATA_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

#[cfg(target_os = "linux")]
fn xdg_base(value: Option<std::ffi::OsString>, home: &std::path::Path, fallback: &str) -> PathBuf {
    value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(fallback))
        .join("llama-launcher")
}

fn directory(kind: &str) -> PathBuf {
    if let Some(root) = override_root() {
        return root;
    }
    #[cfg(target_os = "linux")]
    {
        let (variable, fallback) = match kind {
            "config" => ("XDG_CONFIG_HOME", ".config"),
            "state" => ("XDG_STATE_HOME", ".local/state"),
            "cache" => ("XDG_CACHE_HOME", ".cache"),
            _ => ("XDG_DATA_HOME", ".local/share"),
        };
        xdg_base(std::env::var_os(variable), &glib::home_dir(), fallback)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = kind;
        crate::storage::executable_directory().join("data")
    }
}

pub fn config() -> PathBuf {
    directory("config")
}
pub fn data() -> PathBuf {
    directory("data")
}
pub fn state() -> PathBuf {
    directory("state")
}
pub fn cache() -> PathBuf {
    let root = directory("cache");
    if cfg!(target_os = "linux") && override_root().is_none() {
        root
    } else {
        root.join("cache")
    }
}
pub fn probes() -> PathBuf {
    if cfg!(target_os = "linux") && override_root().is_none() {
        cache().join("probes")
    } else {
        data().join("probes")
    }
}
pub fn models() -> PathBuf {
    data().join("models")
}
pub fn record(name: &str) -> PathBuf {
    match name {
        "artifacts.bin" => data(),
        "downloads.bin" => state(),
        _ => config(),
    }
    .join(name)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn xdg_paths_ignore_empty_and_relative_values() {
        let home = std::path::Path::new("/home/fixture");
        for value in [None, Some("".into()), Some("relative/path".into())] {
            assert_eq!(
                xdg_base(value, home, ".config"),
                home.join(".config/llama-launcher")
            );
        }
        assert_eq!(
            xdg_base(Some("/custom settings".into()), home, ".config"),
            PathBuf::from("/custom settings/llama-launcher")
        );
    }
}
