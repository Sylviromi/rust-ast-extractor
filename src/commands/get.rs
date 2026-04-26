use crate::{cache, commands::index::run_index, project::find_project_root};
use std::path::PathBuf;

/// `target` is either `"path/to/file.rs"` or `"path/to/file.rs::item_name"`
/// or `"path/to/file.rs::kind::item_name"` for disambiguation.
pub fn run_get(target: &str) -> anyhow::Result<()> {
    let (file_str, kind_filter, name_filter) = parse_target(target);
    let file_path = PathBuf::from(file_str);
    let project_root = find_project_root(&file_path);
    let cache_file = cache::cache_path_for_file(&project_root, &file_path);

    // Auto-index if not cached
    if !cache_file.exists() {
        run_index(&file_path)?;
    }

    let fc = cache::read_cache(&cache_file)
        .ok_or_else(|| anyhow::anyhow!("could not read cache for {file_str}"))?;

    if name_filter.is_none() {
        // Return full file JSON
        println!("{}", serde_json::to_string_pretty(&fc)?);
        return Ok(());
    }

    let name = name_filter.unwrap();
    let matches: Vec<_> = fc
        .items
        .iter()
        .filter(|item| {
            item.name == name
                && kind_filter
                    .map(|k| item.kind.to_string() == k)
                    .unwrap_or(true)
        })
        .collect();

    if matches.is_empty() {
        anyhow::bail!("no item named '{name}' found in {file_str}");
    }

    if matches.len() == 1 {
        print!("{}", matches[0].raw_source);
    } else {
        // Multiple matches — print all as JSON array
        println!("{}", serde_json::to_string_pretty(&matches)?);
    }

    Ok(())
}

fn parse_target(target: &str) -> (&str, Option<&str>, Option<&str>) {
    match target.split_once("::") {
        None => (target, None, None),
        Some((file, rest)) => match rest.split_once("::") {
            Some((kind, name)) => (file, Some(kind), Some(name)),
            None => (file, None, Some(rest)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::schema::FileCache;
    use crate::commands::index::run_index;
    use crate::project::find_project_root;
    use std::fs;
    use tempfile::TempDir;

    fn setup_indexed_project(src: &str) -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("lib.rs");
        fs::write(&file, src).unwrap();
        run_index(&file).unwrap();
        (tmp, file)
    }

    #[test]
    fn get_file_returns_json_cache() {
        let src = "pub fn hello() {}\npub struct World;";
        let (_tmp, file) = setup_indexed_project(src);

        // Read cache directly to verify contents (run_get prints to stdout which we can't capture easily in unit tests)
        let project_root = find_project_root(&file);
        let cache_path = crate::cache::cache_path_for_file(&project_root, &file);
        let fc: FileCache =
            serde_json::from_str(&fs::read_to_string(&cache_path).unwrap()).unwrap();

        assert_eq!(fc.items.len(), 2);
        assert!(fc.items.iter().any(|i| i.name == "hello"));
        assert!(fc.items.iter().any(|i| i.name == "World"));
    }

    #[test]
    fn parse_target_file_only() {
        let (file, kind, name) = parse_target("src/lib.rs");
        assert_eq!(file, "src/lib.rs");
        assert!(kind.is_none());
        assert!(name.is_none());
    }

    #[test]
    fn parse_target_file_with_item() {
        let (file, kind, name) = parse_target("src/lib.rs::my_fn");
        assert_eq!(file, "src/lib.rs");
        assert!(kind.is_none());
        assert_eq!(name, Some("my_fn"));
    }

    #[test]
    fn parse_target_file_with_kind_and_item() {
        let (file, kind, name) = parse_target("src/lib.rs::fn::my_fn");
        assert_eq!(file, "src/lib.rs");
        assert_eq!(kind, Some("fn"));
        assert_eq!(name, Some("my_fn"));
    }

    #[test]
    fn get_auto_indexes_unindexed_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("lib.rs");
        fs::write(&file, "pub fn auto() {}").unwrap();

        // No run_index call — get should handle it
        let target = file.to_string_lossy().to_string();
        run_get(&target).unwrap(); // should not panic

        let project_root = find_project_root(&file);
        let cache_path = crate::cache::cache_path_for_file(&project_root, &file);
        assert!(cache_path.exists(), "auto-index should have created cache");
    }
}
