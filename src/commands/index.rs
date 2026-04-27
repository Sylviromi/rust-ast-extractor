use crate::{
    cache::{self, schema::FileCache},
    extractor,
    project::find_project_root,
};
use std::path::Path;
use walkdir::WalkDir;

pub fn run_index(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }

    let project_root = find_project_root(path);

    if path.is_file() {
        let rel = path
            .strip_prefix(&project_root)
            .unwrap_or(path)
            .display()
            .to_string();
        let updated = index_single_file(path, &project_root)?;
        if updated {
            eprintln!("updated {rel}");
        } else {
            eprintln!("{rel} up to date");
        }
    } else {
        let mut updated_count: usize = 0;
        let mut unchanged_count: usize = 0;
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        {
            let updated = index_single_file(entry.path(), &project_root)?;
            if updated {
                let rel = entry
                    .path()
                    .strip_prefix(&project_root)
                    .unwrap_or(entry.path())
                    .display()
                    .to_string();
                eprintln!("updated {rel}");
                updated_count += 1;
            } else {
                unchanged_count += 1;
            }
        }
        if updated_count == 0 {
            eprintln!("All {} files up to date.", unchanged_count);
        } else {
            eprintln!("{updated_count} files updated, {unchanged_count} unchanged.");
        }
    }

    Ok(())
}

fn index_single_file(source_file: &Path, project_root: &Path) -> anyhow::Result<bool> {
    let source = std::fs::read_to_string(source_file)?;
    let file_hash = cache::compute_hash(&source);
    let cache_file = cache::cache_path_for_file(project_root, source_file);

    let existing = cache::read_cache(&cache_file);

    // Skip if file is unchanged
    if let Some(ref e) = existing
        && e.file_hash == file_hash
    {
        return Ok(false);
    }
    let new_items = extractor::extract_file(source_file, &source)
        .map_err(|e| anyhow::anyhow!("parse error in {}: {}", source_file.display(), e))?;
    let merged = cache::merge_items(new_items, existing.as_ref());
    let fc = FileCache {
        file: source_file
            .strip_prefix(project_root)
            .unwrap_or(source_file)
            .to_string_lossy()
            .to_string(),
        file_hash,
        indexed_at: chrono::Utc::now().to_rfc3339(),
        items: merged,
    };
    cache::write_cache(&cache_file, &fc)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project(src: &str) -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("lib.rs");
        fs::write(&file, src).unwrap();
        (tmp, file)
    }

    #[test]
    fn index_creates_cache_file() {
        let (tmp, file) = setup_project("pub fn foo() {}");
        run_index(&file).unwrap();

        let cache_path = tmp.path().join(".ast-cache/files/src/lib.rs.json");
        assert!(cache_path.exists(), "cache file not created");

        let content = fs::read_to_string(&cache_path).unwrap();
        let fc: FileCache = serde_json::from_str(&content).unwrap();
        assert_eq!(fc.items.len(), 1);
        assert_eq!(fc.items[0].name, "foo");
    }

    #[test]
    fn reindex_unchanged_file_skips_reparse() {
        let (tmp, file) = setup_project("pub fn foo() {}");
        run_index(&file).unwrap();

        let cache_path = tmp.path().join(".ast-cache/files/src/lib.rs.json");
        let mtime1 = fs::metadata(&cache_path).unwrap().modified().unwrap();

        // Small delay to ensure mtime would differ if file were rewritten
        std::thread::sleep(std::time::Duration::from_millis(10));
        run_index(&file).unwrap();

        let mtime2 = fs::metadata(&cache_path).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "cache was rewritten despite no change");
    }

    #[test]
    fn index_directory_indexes_all_rs_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src = tmp.path().join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("main.rs"), "pub fn main() {}").unwrap();
        fs::write(src.join("lib.rs"), "pub struct Foo;").unwrap();

        run_index(tmp.path()).unwrap();

        assert!(
            tmp.path()
                .join(".ast-cache/files/src/main.rs.json")
                .exists()
        );
        assert!(tmp.path().join(".ast-cache/files/src/lib.rs.json").exists());
    }

    #[test]
    fn index_single_file_returns_true_on_new_file() {
        let (_tmp, file) = setup_project("pub fn foo() {}");
        let project_root = crate::project::find_project_root(&file);
        let result = index_single_file(&file, &project_root).unwrap();
        assert!(result, "new file should return true");
    }

    #[test]
    fn index_single_file_returns_false_on_unchanged_file() {
        let (_tmp, file) = setup_project("pub fn foo() {}");
        let project_root = crate::project::find_project_root(&file);
        // first index
        index_single_file(&file, &project_root).unwrap();
        // second index — same hash
        let result = index_single_file(&file, &project_root).unwrap();
        assert!(!result, "unchanged file should return false");
    }
}
