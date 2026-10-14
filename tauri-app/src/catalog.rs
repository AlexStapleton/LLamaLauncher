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
                let name = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
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
