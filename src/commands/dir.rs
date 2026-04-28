use crate::{cache, commands::index::run_index, project::find_project_root};
use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize)]
struct DirEntry {
    file: String,
    module_doc: String,
}

pub fn run_dir(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("path is not a directory: {}", path.display());
    }

    let project_root = find_project_root(path);

    let mut entries: Vec<DirEntry> = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .map(|e| -> anyhow::Result<DirEntry> {
            let file_path = e.path();
            let cache_file = cache::cache_path_for_file(&project_root, file_path);
            if !cache_file.exists() {
                run_index(file_path)?;
            }
            let fc = cache::read_cache(&cache_file).ok_or_else(|| {
                anyhow::anyhow!("could not read cache for {}", file_path.display())
            })?;
            Ok(DirEntry {
                file: fc.file,
                module_doc: fc.module_doc,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    entries.sort_by(|a, b| a.file.cmp(&b.file));

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src = tmp.path().join("src");
        fs::create_dir(&src).unwrap();
        fs::write(
            src.join("lib.rs"),
            "//! Library module.\n\npub fn foo() {}",
        )
        .unwrap();
        fs::write(src.join("main.rs"), "pub fn main() {}").unwrap();
        tmp
    }

    #[test]
    fn dir_returns_sorted_entries() {
        let tmp = setup_dir();
        let src = tmp.path().join("src");

        // Capture output via direct logic (run_dir prints to stdout)
        let project_root = find_project_root(&src);
        let mut entries: Vec<DirEntry> = WalkDir::new(&src)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .map(|e| -> anyhow::Result<DirEntry> {
                let file_path = e.path();
                let cache_file = cache::cache_path_for_file(&project_root, file_path);
                if !cache_file.exists() {
                    run_index(file_path)?;
                }
                let fc = cache::read_cache(&cache_file).ok_or_else(|| {
                    anyhow::anyhow!("could not read cache for {}", file_path.display())
                })?;
                Ok(DirEntry {
                    file: fc.file,
                    module_doc: fc.module_doc,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .unwrap();

        entries.sort_by(|a, b| a.file.cmp(&b.file));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file, "src/lib.rs");
        assert_eq!(entries[0].module_doc, "Library module.");
        assert_eq!(entries[1].file, "src/main.rs");
        assert_eq!(entries[1].module_doc, "");
    }

    #[test]
    fn dir_errors_on_nonexistent_path() {
        let result = run_dir(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn dir_errors_on_file_not_dir() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("foo.rs");
        fs::write(&file, "").unwrap();
        let result = run_dir(&file);
        assert!(result.is_err());
    }
}