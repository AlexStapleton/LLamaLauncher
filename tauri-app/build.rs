fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/frontend");
    println!("cargo:rerun-if-changed=dist/index.html");
    println!("cargo:rerun-if-changed=package-lock.json");
    if std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some() {
        let built = std::fs::metadata("dist/index.html")
            .and_then(|m| m.modified())
            .map_err(|_| {
                "Frontend assets are missing. Run npm ci and npm run build before Cargo."
            })?;
        let mut pending = vec![std::path::PathBuf::from("src/frontend")];
        let mut files = vec![
            "package.json".into(),
            "package-lock.json".into(),
            "vite.config.js".into(),
            "tsconfig.json".into(),
        ];
        while let Some(folder) = pending.pop() {
            for entry in std::fs::read_dir(folder)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    pending.push(entry.path());
                } else {
                    files.push(entry.path());
                }
            }
        }
        for file in files {
            if std::fs::metadata(&file)?.modified()? > built {
                return Err(format!(
                    "Frontend assets are stale after {} changed. Run npm run build first.",
                    file.display()
                )
                .into());
            }
        }
    }
    tauri_build::build();
    Ok(())
}
