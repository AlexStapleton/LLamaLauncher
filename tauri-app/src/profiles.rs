use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};

const LIMIT: usize = 4 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct LaunchProfile {
    pub id: String,
    pub name: String,
    pub config: Config,
}
#[derive(Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub runtime: String,
    pub model: String,
    pub router: bool,
    pub overrides: u32,
}
#[derive(Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub profiles: Vec<LaunchProfile>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: Vec::new(),
        }
    }
}
#[derive(Default)]
pub struct Manager(pub Mutex<()>);

pub fn path() -> std::path::PathBuf {
    crate::storage::profile_directory().join("launch-profiles.bin")
}
pub fn read(path: &Path) -> Result<Library, String> {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Library::default()),
        Err(_) => return Err("Cannot access saved profiles.".into()),
        Ok(_) => {}
    }
    let library: Library = serde_json::from_slice(&crate::storage::read_private(path, LIMIT)?)
        .map_err(|_| crate::storage::damaged(path, "Saved profile library is damaged"))?;
    if library.version != 1 {
        return Err(crate::storage::damaged(
            path,
            "Profile library needs a newer LLamaLauncher version",
        ));
    }
    if library.profiles.len() > 128 {
        return Err(crate::storage::damaged(
            path,
            "Profile library exceeds 128 profiles",
        ));
    }
    if library
        .profiles
        .iter()
        .any(|profile| profile.config.schema_version > 3)
    {
        return Err(
            "A saved profile requires a newer LLamaLauncher version. The library was preserved."
                .into(),
        );
    }
    Ok(library)
}
pub fn write(path: &Path, library: &Library) -> Result<(), String> {
    let bytes = serde_json::to_vec(library).map_err(|_| "Cannot encode profiles.")?;
    if bytes.len() > LIMIT {
        return Err("Profile library exceeded 4 MiB. Reduce very large custom arguments.".into());
    }
    crate::storage::write_private_atomic(path, &bytes)
}
impl Library {
    pub fn summaries(&self) -> Vec<ProfileSummary> {
        self.profiles
            .iter()
            .map(|profile| ProfileSummary {
                id: profile.id.clone(),
                name: profile.name.clone(),
                runtime: profile.config.llama_cpp_path.clone(),
                model: if profile.config.router_mode {
                    profile.config.models_path.clone()
                } else {
                    profile.config.model_file.clone()
                },
                router: profile.config.router_mode,
                overrides: profile.config.argument_overrides.len() as u32,
            })
            .collect()
    }
    pub fn get(&self, id: &str) -> Result<LaunchProfile, String> {
        self.profiles
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
            .ok_or_else(|| "That profile no longer exists. Refresh the profile list.".into())
    }
    pub fn save(
        &mut self,
        id: Option<String>,
        name: String,
        config: Config,
    ) -> Result<LaunchProfile, String> {
        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(
                "Choose a profile name of 1–80 characters without control characters.".into(),
            );
        }
        if self.profiles.iter().any(|profile| {
            Some(&profile.id) != id.as_ref() && profile.name.to_lowercase() == name.to_lowercase()
        }) {
            return Err("A profile already has that name. Choose a different name.".into());
        }
        if let Some(id) = id {
            let entry = self
                .profiles
                .iter_mut()
                .find(|profile| profile.id == id)
                .ok_or("That profile no longer exists.")?;
            entry.name = name;
            entry.config = config;
            return Ok(entry.clone());
        }
        if self.profiles.len() >= 128 {
            return Err("You can save up to 128 profiles.".into());
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "Invalid clock.")?
            .as_nanos();
        let mut id = format!("{:x}-{stamp:x}", std::process::id());
        while self.profiles.iter().any(|profile| profile.id == id) {
            id.push('x');
        }
        let entry = LaunchProfile { id, name, config };
        self.profiles.push(entry.clone());
        Ok(entry)
    }
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.get(id)?;
        self.profiles.retain(|profile| profile.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_profiles_roundtrip_every_runtime_setting_and_secrets_privately() {
        let folder =
            std::env::temp_dir().join(format!("llama-launch-profiles-{}", std::process::id()));
        let path = folder.join("profiles.bin");
        let mut config = Config {
            llama_cpp_path: "/runtime/llama-server".into(),
            models_path: "/models".into(),
            model_file: "/models/test.gguf".into(),
            additional_args: vec!["--api-key".into(), "fixture-secret".into()],
            ..Config::default()
        };
        config.mcp_profile.bearer_token = "fixture-bearer".into();
        let mut library = Library::default();
        let profile = library.save(None, " Coding ".into(), config).unwrap();
        write(&path, &library).unwrap();
        let mut restored = read(&path).unwrap();
        let loaded = restored.get(&profile.id).unwrap();
        assert_eq!(loaded.name, "Coding");
        assert_eq!(loaded.config.additional_args[1], "fixture-secret");
        assert_eq!(loaded.config.mcp_profile.bearer_token, "fixture-bearer");
        #[cfg(windows)]
        assert!(!std::fs::read(&path)
            .unwrap()
            .windows(14)
            .any(|part| part == b"fixture-secret"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(restored
            .save(None, "coding".into(), Config::default())
            .is_err());
        let duplicate = restored
            .save(None, "Coding copy".into(), loaded.config.clone())
            .unwrap();
        let renamed = restored
            .save(Some(profile.id.clone()), "Chat".into(), loaded.config)
            .unwrap();
        assert_eq!(renamed.id, profile.id);
        write(&path, &restored).unwrap();
        restored.delete(&duplicate.id).unwrap();
        write(&path, &restored).unwrap();
        assert_eq!(read(&path).unwrap().profiles.len(), 1);
        std::fs::remove_file(path.with_extension("bin.bak")).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn corrupt_library_is_not_silently_reset() {
        let folder =
            std::env::temp_dir().join(format!("llama-bad-profiles-{}", std::process::id()));
        let path = folder.join("profiles.bin");
        crate::storage::write_private_atomic(&path, b"not json").unwrap();
        assert!(read(&path).is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn oversized_updates_preserve_previous_library_and_future_configs_are_rejected() {
        let folder =
            std::env::temp_dir().join(format!("llama-profile-limits-{}", std::process::id()));
        let path = folder.join("profiles.bin");
        let mut library = Library::default();
        let original = library
            .save(None, "Safe".into(), Config::default())
            .unwrap();
        write(&path, &library).unwrap();
        library
            .save(
                Some(original.id.clone()),
                "Too big".into(),
                Config {
                    additional_args: vec!["x".repeat(LIMIT)],
                    ..Config::default()
                },
            )
            .unwrap();
        assert!(write(&path, &library).is_err());
        assert_eq!(read(&path).unwrap().get(&original.id).unwrap().name, "Safe");
        library.profiles[0].config = Config {
            schema_version: 99,
            ..Config::default()
        };
        write(&path, &library).unwrap();
        assert!(read(&path).is_err());
        std::fs::remove_file(path.with_extension("bin.bak")).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
