pub mod schema;

use schema::{ExtractedItem, FileCache};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn cache_path_for_file(project_root: &Path, source_file: &Path) -> PathBuf {
    let relative = source_file
        .strip_prefix(project_root)
        .unwrap_or(source_file);
    project_root
        .join(".ast-cache")
        .join("files")
        .join(format!("{}.json", relative.display()))
}

pub fn read_cache(cache_file: &Path) -> Option<FileCache> {
    let content = std::fs::read_to_string(cache_file).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_cache(cache_file: &Path, cache: &FileCache) -> std::io::Result<()> {
    if let Some(parent) = cache_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cache).expect("serialization failed");
    std::fs::write(cache_file, json)
}

/// Merges freshly extracted items with the existing cache.
/// If an item's hash is unchanged, the existing entry is kept (preserving future ai_summary fields).
/// Items no longer in the source are dropped.
pub fn merge_items(
    new_items: Vec<ExtractedItem>,
    existing: Option<&FileCache>,
) -> Vec<ExtractedItem> {
    new_items
        .into_iter()
        .map(|item| {
            if let Some(cached) = existing
                && let Some(existing_item) = cached
                    .items
                    .iter()
                    .find(|e| e.name == item.name && e.kind == item.kind && e.parent == item.parent)
                && existing_item.item_hash == item.item_hash
            {
                return existing_item.clone();
            }
            item
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::ItemKind;
    use tempfile::TempDir;

    fn make_item(name: &str, hash: &str) -> ExtractedItem {
        ExtractedItem {
            kind: ItemKind::Fn,
            name: name.into(),
            parent: None,
            visibility: "pub".into(),
            signature: format!("pub fn {name}()"),
            docs: String::new(),
            attributes: vec![],
            line_start: 1,
            line_end: 1,
            item_hash: hash.into(),
            raw_source: format!("pub fn {name}() {{}}"),
        }
    }

    #[test]
    fn compute_hash_is_deterministic() {
        let h1 = compute_hash("hello");
        let h2 = compute_hash("hello");
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn compute_hash_differs_for_different_content() {
        assert_ne!(compute_hash("hello"), compute_hash("world"));
    }

    #[test]
    fn cache_path_mirrors_source_path() {
        let root = Path::new("/project");
        let source = Path::new("/project/src/lib.rs");
        let cache = cache_path_for_file(root, source);
        assert_eq!(
            cache,
            Path::new("/project/.ast-cache/files/src/lib.rs.json")
        );
    }

    #[test]
    fn write_and_read_cache_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let cache_file = tmp.path().join("test.json");
        let fc = FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:abc".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            module_doc: String::new(),
            items: vec![make_item("foo", "sha256:111")],
            line_count: 0,
        };
        write_cache(&cache_file, &fc).unwrap();
        let loaded = read_cache(&cache_file).unwrap();
        assert_eq!(loaded.file, "src/lib.rs");
        assert_eq!(loaded.items[0].name, "foo");
    }

    #[test]
    fn read_cache_returns_none_for_missing_file() {
        let result = read_cache(Path::new("/nonexistent/file.json"));
        assert!(result.is_none());
    }

    #[test]
    fn merge_keeps_unchanged_items() {
        let existing = FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:old".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            module_doc: String::new(),
            items: vec![make_item("foo", "sha256:111")],
            line_count: 0,
        };
        let new_items = vec![make_item("foo", "sha256:111")];
        let merged = merge_items(new_items, Some(&existing));
        assert_eq!(merged[0].item_hash, "sha256:111");
    }

    #[test]
    fn merge_replaces_changed_items() {
        let existing = FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:old".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            module_doc: String::new(),
            items: vec![make_item("foo", "sha256:old_hash")],
            line_count: 0,
        };
        let mut new_item = make_item("foo", "sha256:new_hash");
        new_item.signature = "pub fn foo(x: u32)".into();
        let merged = merge_items(vec![new_item], Some(&existing));
        assert_eq!(merged[0].item_hash, "sha256:new_hash");
        assert_eq!(merged[0].signature, "pub fn foo(x: u32)");
    }

    #[test]
    fn merge_drops_removed_items() {
        let existing = FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:old".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            module_doc: String::new(),
            items: vec![
                make_item("foo", "sha256:111"),
                make_item("bar", "sha256:222"),
            ],
            line_count: 0,
        };
        // "bar" is gone from source
        let new_items = vec![make_item("foo", "sha256:111")];
        let merged = merge_items(new_items, Some(&existing));
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "foo");
    }
}
