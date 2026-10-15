use serde::Serialize;
use std::{fs, path::Path};
#[derive(Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[cfg(test)]
pub fn scan_models(directory: &Path) -> Result<Vec<ModelInfo>, String> {
    scan_models_cancel(directory, None, |_, _| {})
}
pub static SCAN_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn scan_models_cancel(
    directory: &Path,
    generation: Option<u64>,
    progress: impl Fn(usize, usize),
) -> Result<Vec<ModelInfo>, String> {
    let root = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    if !root.is_dir() {
        return Err("Select a models folder.".into());
    }
    let mut pending = vec![(root.clone(), 0usize)];
    let mut models = Vec::new();
    let mut visited = 0usize;
    let mut managed_groups = std::collections::HashMap::<std::path::PathBuf, Vec<String>>::new();
    while let Some((folder, depth)) = pending.pop() {
        for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
            if generation
                .is_some_and(|g| g != SCAN_GENERATION.load(std::sync::atomic::Ordering::Relaxed))
            {
                return Err("Model scan cancelled by a newer folder selection.".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            visited += 1;
            if visited.is_multiple_of(1000) {
                progress(visited, models.len());
            }
            if visited > 100000 {
                return Err(
                    "This folder has over 100,000 entries. Select a narrower model folder.".into(),
                );
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if !kind.is_dir()
                && !(kind.is_file()
                    && path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("gguf")))
            {
                continue;
            }
            let canonical = match fs::canonicalize(&path) {
                Ok(path) => path,
                Err(_) => continue,
            };
            if !canonical.starts_with(&root) {
                continue;
            }
            if kind.is_dir() && depth < 16 {
                pending.push((canonical, depth + 1));
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("gguf"))
            {
                if let Ok(relative) = canonical.strip_prefix(&root) {
                    let parts: Vec<_> = relative.components().collect();
                    if let Some(start) = parts
                        .iter()
                        .position(|p| p.as_os_str() == ".launcher-downloads")
                    {
                        if parts.len() > start + 4 {
                            let mut group = root.clone();
                            for component in &parts[..start + 4] {
                                group.push(component);
                            }
                            let names = managed_groups.entry(group.clone()).or_insert_with(|| {
                                let marker = group.join("complete.json");
                                if !fs::metadata(&marker).is_ok_and(|m| m.len() <= 4 * 1024 * 1024)
                                {
                                    return Vec::new();
                                }
                                fs::read(marker)
                                    .ok()
                                    .and_then(|b| {
                                        serde_json::from_slice::<crate::downloads::DownloadPlan>(&b)
                                            .ok()
                                    })
                                    .map(|p| p.files.into_iter().map(|f| f.name).collect())
                                    .unwrap_or_default()
                            });
                            let relative = canonical
                                .strip_prefix(&group)
                                .map(|p| p.to_string_lossy().replace('\\', "/"))
                                .unwrap_or_default();
                            if !names.contains(&relative) {
                                continue;
                            }
                        }
                    }
                }
                let name = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                if path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().contains("mmproj"))
                {
                    continue;
                }
                if let Some((_, suffix)) = name.rsplit_once("-of-") {
                    if suffix.ends_with(".gguf") && !name.contains("-00001-of-") {
                        continue;
                    }
                }
                models.push(ModelInfo {
                    name,
                    path: canonical
                        .to_string_lossy()
                        .trim_start_matches(r"\\?\")
                        .to_owned(),
                    size: entry.metadata().map_err(|e| e.to_string())?.len(),
                });
            }
        }
    }
    models.sort_by_key(|m| m.name.to_lowercase());
    Ok(models)
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn case_sensitive_models_remain_distinct() {
        let root = std::env::temp_dir().join(format!("llama-case-models-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Model.gguf"), b"one").unwrap();
        fs::write(root.join("model.gguf"), b"two").unwrap();
        let models = scan_models(&root).unwrap();
        assert_eq!(models.len(), 2);
        assert_ne!(models[0].path, models[1].path);
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod managed_tests {
    use super::*;
    #[test]
    fn only_completed_manifest_members_are_discovered() {
        let root = std::env::temp_dir().join(format!("managed-scan-{}", std::process::id()));
        let group = root
            .join(".launcher-downloads/owner/repo")
            .join("a".repeat(40));
        fs::create_dir_all(&group).unwrap();
        fs::write(group.join("complete-model.gguf"), b"one").unwrap();
        fs::write(group.join("incomplete-model.gguf"), b"two").unwrap();
        assert!(scan_models(&root).unwrap().is_empty());
        let plan = crate::downloads::DownloadPlan {
            id: "fixture".into(),
            repo: "owner/repo".into(),
            revision: "a".repeat(40),
            root: root.to_string_lossy().into(),
            files: vec![crate::downloads::DownloadFile {
                name: "complete-model.gguf".into(),
                size: 3,
                sha256: String::new(),
            }],
        };
        fs::write(
            group.join("complete.json"),
            serde_json::to_vec(&plan).unwrap(),
        )
        .unwrap();
        let models = scan_models(&root).unwrap();
        assert_eq!(models.len(), 1);
        assert!(models[0].name.contains("complete-model"));
        fs::remove_dir_all(root).unwrap();
    }
}
