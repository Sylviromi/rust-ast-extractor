use crate::{cache, commands::index::run_index, project::find_project_root};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct ItemSummary<'a> {
    kind: &'a crate::cache::schema::ItemKind,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<&'a str>,
    visibility: &'a str,
    signature: &'a str,
    docs: &'a str,
    attributes: &'a [String],
    line_start: u32,
    line_end: u32,
}

#[derive(Serialize)]
struct FileSummary<'a> {
    file: &'a str,
    module_doc: &'a str,
    items: Vec<ItemSummary<'a>>,
}

fn build_file_summary_json(fc: &crate::cache::schema::FileCache) -> anyhow::Result<String> {
    let summary = FileSummary {
        file: &fc.file,
        module_doc: &fc.module_doc,
        items: fc
            .items
            .iter()
            .map(|item| ItemSummary {
                kind: &item.kind,
                name: &item.name,
                parent: item.parent.as_deref(),
                visibility: &item.visibility,
                signature: &item.signature,
                docs: &item.docs,
                attributes: &item.attributes,
                line_start: item.line_start,
                line_end: item.line_end,
            })
            .collect(),
    };
    Ok(serde_json::to_string_pretty(&summary)?)
}

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
        println!("{}", build_file_summary_json(&fc)?);
        return Ok(());
    }

    let name = name_filter.unwrap();

    // If the middle segment is not a valid item kind, treat it as a parent type name.
    const VALID_KINDS: &[&str] = &[
        "fn", "struct", "enum", "trait", "impl", "type", "const", "macro", "mod",
    ];
    let (kind_filter, parent_filter) = match kind_filter {
        Some(mid) if VALID_KINDS.contains(&mid) => (Some(mid), None),
        Some(mid) => (None, Some(mid)),
        None => (None, None),
    };

    let matches: Vec<_> = fc
        .items
        .iter()
        .filter(|item| {
            item.name == name
                && kind_filter
                    .map(|k| item.kind.to_string() == k)
                    .unwrap_or(true)
                && parent_filter
                    .map(|p| item.parent.as_deref() == Some(p))
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

    #[test]
    fn get_file_summary_excludes_internal_fields() {
        let src = "pub fn hello() {}";
        let (_tmp, file) = setup_indexed_project(src);

        let project_root = find_project_root(&file);
        let cache_path = crate::cache::cache_path_for_file(&project_root, &file);
        let raw_json = fs::read_to_string(&cache_path).unwrap();

        // The on-disk cache should still have item_hash and raw_source
        let cache_value: serde_json::Value = serde_json::from_str(&raw_json).unwrap();
        assert!(
            cache_value["items"][0].get("item_hash").is_some(),
            "cache should retain item_hash"
        );
        assert!(
            cache_value["items"][0].get("raw_source").is_some(),
            "cache should retain raw_source"
        );

        // The get output (stdout) should NOT include them.
        // We test this by constructing the summary the same way run_get would.
        let fc: crate::cache::schema::FileCache = serde_json::from_str(&raw_json).unwrap();
        let summary_json = build_file_summary_json(&fc).unwrap();
        let summary: serde_json::Value = serde_json::from_str(&summary_json).unwrap();
        assert!(
            summary.get("file_hash").is_none(),
            "file_hash must be absent"
        );
        assert!(
            summary.get("indexed_at").is_none(),
            "indexed_at must be absent"
        );
        assert!(
            summary["items"][0].get("item_hash").is_none(),
            "item_hash must be absent"
        );
        assert!(
            summary["items"][0].get("raw_source").is_none(),
            "raw_source must be absent"
        );
        assert!(
            summary["items"][0].get("line_start").is_some(),
            "line_start must be present"
        );
        assert!(summary["items"][0]["kind"].as_str().unwrap() == "fn");
        assert!(summary["items"][0]["name"].as_str().unwrap() == "hello");
    }
}
