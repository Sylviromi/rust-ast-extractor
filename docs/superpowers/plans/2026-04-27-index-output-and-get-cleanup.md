# Index Output & Get Command Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clean up CLI output so `index` reports only updated files plus a summary, and `get <file>` returns lean agent-friendly JSON without hashes, timestamps, line numbers, or raw source.

**Architecture:** Three independent changes: (1) remove unused `line_start`/`line_end` fields from `ExtractedItem` and all set sites, (2) change `index_single_file` to return a `bool` and update `run_index` to print selectively plus a summary, (3) add lean `FileSummary`/`ItemSummary` output structs to `get.rs` used when no item filter is given.

**Tech Stack:** Rust, serde/serde_json, walkdir, syn, anyhow

---

### Task 1: Remove `line_start`/`line_end` from schema and all set sites

**Files:**
- Modify: `src/cache/schema.rs`
- Modify: `src/cache/mod.rs`
- Modify: `src/extractor/visitor.rs`

- [ ] **Step 1: Remove fields from `ExtractedItem` in `src/cache/schema.rs`**

In `src/cache/schema.rs`, remove lines 42–43 from `ExtractedItem`:

```rust
// DELETE these two lines:
pub line_start: u32,
pub line_end: u32,
```

Also update the `sample_cache()` test fixture (around line 72–73) — remove `line_start: 1,` and `line_end: 3,` from the `ExtractedItem` literal.

- [ ] **Step 2: Remove from `make_item` helper in `src/cache/mod.rs`**

In `src/cache/mod.rs`, the `make_item` helper (around line 66) currently sets `line_start: 1,` and `line_end: 1,`. Remove both lines:

```rust
fn make_item(name: &str, hash: &str) -> ExtractedItem {
    ExtractedItem {
        kind: ItemKind::Fn,
        name: name.into(),
        visibility: "pub".into(),
        signature: format!("pub fn {name}()"),
        docs: String::new(),
        attributes: vec![],
        item_hash: hash.into(),
        raw_source: format!("pub fn {name}() {{}}"),
    }
}
```

- [ ] **Step 3: Remove all `line_start`/`line_end` assignments in `src/extractor/visitor.rs`**

Each `ExtractedItem` construction in `visitor.rs` sets `line_start` and `line_end`. Remove those two lines from every struct literal. There are 9 occurrences (one per visitor method plus the impl method loop).

For example, `visit_item_fn` (around line 80–91) becomes:

```rust
self.items.push(ExtractedItem {
    kind: ItemKind::Fn,
    name: i.sig.ident.to_string(),
    visibility: vis,
    signature: sig,
    docs: extract_docs(&i.attrs),
    attributes: extract_non_doc_attrs(&i.attrs),
    item_hash: item_hash(i.to_token_stream()),
    raw_source: raw,
});
```

Apply the same removal to `visit_item_struct`, `visit_item_enum`, `visit_item_trait`, `visit_item_impl` (both the impl block itself and the method loop inside it), `visit_item_type`, `visit_item_const`, `visit_item_macro`, `visit_item_mod`.

Note: the local `(start, end)` or `start`/`end` variables are still needed for `extract_lines` calls (which compute `raw_source`). Do NOT remove those. Only remove the two lines that assign to `line_start`/`line_end` in the struct literal.

- [ ] **Step 4: Run tests and verify they all pass**

```bash
cargo test
```

Expected: all tests pass. If the compiler complains about unknown fields, you missed a struct literal — search for `line_start` to find any stragglers:

```bash
grep -r line_start src/
```

Expected output: no matches.

- [ ] **Step 5: Commit**

```bash
git add src/cache/schema.rs src/cache/mod.rs src/extractor/visitor.rs
git commit -m "refactor: remove unused line_start/line_end from ExtractedItem"
```

---

### Task 2: Index command — show only updated files plus summary

**Files:**
- Modify: `src/commands/index.rs`

- [ ] **Step 1: Update `index_single_file` return type to `bool`**

Change the signature of `index_single_file` (line 44) from `-> anyhow::Result<()>` to `-> anyhow::Result<bool>`.

Change the early-return skip branch to `return Ok(false);`:

```rust
if let Some(ref existing) = cache::read_cache(&cache_file)
    && existing.file_hash == file_hash
{
    return Ok(false);
}
```

Change the final line from `Ok(())` to `Ok(true)`:

```rust
cache::write_cache(&cache_file, &fc)?;
Ok(true)
```

- [ ] **Step 2: Write a failing test for the return value**

Add this test inside the `#[cfg(test)]` block in `src/commands/index.rs`:

```rust
#[test]
fn index_single_file_returns_true_on_new_file() {
    let (tmp, file) = setup_project("pub fn foo() {}");
    let project_root = crate::project::find_project_root(&file);
    let result = index_single_file(&file, &project_root).unwrap();
    assert!(result, "new file should return true");
}

#[test]
fn index_single_file_returns_false_on_unchanged_file() {
    let (tmp, file) = setup_project("pub fn foo() {}");
    let project_root = crate::project::find_project_root(&file);
    // first index
    index_single_file(&file, &project_root).unwrap();
    // second index — same hash
    let result = index_single_file(&file, &project_root).unwrap();
    assert!(!result, "unchanged file should return false");
}
```

- [ ] **Step 3: Run the new tests — expect compile error**

```bash
cargo test index_single_file_returns
```

Expected: compile error because `index_single_file` still returns `()`. This confirms the tests are wired up correctly.

- [ ] **Step 4: Apply the `bool` return change to `index_single_file`**

Replace the full body of `index_single_file` in `src/commands/index.rs`:

```rust
fn index_single_file(source_file: &Path, project_root: &Path) -> anyhow::Result<bool> {
    let source = std::fs::read_to_string(source_file)?;
    let file_hash = cache::compute_hash(&source);
    let cache_file = cache::cache_path_for_file(project_root, source_file);

    if let Some(ref existing) = cache::read_cache(&cache_file)
        && existing.file_hash == file_hash
    {
        return Ok(false);
    }

    let existing = cache::read_cache(&cache_file);
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
```

- [ ] **Step 5: Update `run_index` to use the return value**

Replace the full body of `run_index` in `src/commands/index.rs`:

```rust
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
                eprintln!("  updated {rel}");
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
```

- [ ] **Step 6: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/commands/index.rs
git commit -m "feat: index command reports only updated files with summary"
```

---

### Task 3: Lean `get` output for file summary mode

**Files:**
- Modify: `src/commands/get.rs`

- [ ] **Step 1: Write a failing test for lean output**

Add this test inside the `#[cfg(test)]` block in `src/commands/get.rs`:

```rust
#[test]
fn get_file_summary_excludes_internal_fields() {
    let src = "pub fn hello() {}";
    let (_tmp, file) = setup_indexed_project(src);

    let project_root = find_project_root(&file);
    let cache_path = crate::cache::cache_path_for_file(&project_root, &file);
    let raw_json = fs::read_to_string(&cache_path).unwrap();

    // The on-disk cache should still have item_hash and raw_source
    let cache_value: serde_json::Value = serde_json::from_str(&raw_json).unwrap();
    assert!(cache_value["items"][0].get("item_hash").is_some(), "cache should retain item_hash");
    assert!(cache_value["items"][0].get("raw_source").is_some(), "cache should retain raw_source");

    // The get output (stdout) should NOT include them.
    // We test this by constructing the summary the same way run_get would.
    let fc: crate::cache::schema::FileCache = serde_json::from_str(&raw_json).unwrap();
    let summary_json = build_file_summary_json(&fc);
    let summary: serde_json::Value = serde_json::from_str(&summary_json).unwrap();
    assert!(summary.get("file_hash").is_none(), "file_hash must be absent");
    assert!(summary.get("indexed_at").is_none(), "indexed_at must be absent");
    assert!(summary["items"][0].get("item_hash").is_none(), "item_hash must be absent");
    assert!(summary["items"][0].get("raw_source").is_none(), "raw_source must be absent");
    assert!(summary["items"][0].get("line_start").is_none(), "line_start must be absent");
    assert!(summary["items"][0]["kind"].as_str().unwrap() == "fn");
    assert!(summary["items"][0]["name"].as_str().unwrap() == "hello");
}
```

This test calls `build_file_summary_json`, which does not exist yet — this is intentional.

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test get_file_summary_excludes_internal_fields
```

Expected: compile error — `build_file_summary_json` not found.

- [ ] **Step 3: Add the lean output structs and `build_file_summary_json` to `get.rs`**

Add `use serde::Serialize;` at the top of `src/commands/get.rs` (alongside existing `use` statements).

Then add these structs and helper function before `run_get`:

```rust
use serde::Serialize;

#[derive(Serialize)]
struct ItemSummary<'a> {
    kind: &'a crate::cache::schema::ItemKind,
    name: &'a str,
    visibility: &'a str,
    signature: &'a str,
    docs: &'a str,
    attributes: &'a [String],
}

#[derive(Serialize)]
struct FileSummary<'a> {
    file: &'a str,
    items: Vec<ItemSummary<'a>>,
}

fn build_file_summary_json(fc: &crate::cache::schema::FileCache) -> String {
    let summary = FileSummary {
        file: &fc.file,
        items: fc
            .items
            .iter()
            .map(|item| ItemSummary {
                kind: &item.kind,
                name: &item.name,
                visibility: &item.visibility,
                signature: &item.signature,
                docs: &item.docs,
                attributes: &item.attributes,
            })
            .collect(),
    };
    serde_json::to_string_pretty(&summary).expect("serialization failed")
}
```

- [ ] **Step 4: Use `build_file_summary_json` in the file-only output path**

In `run_get`, replace the block that handles `name_filter.is_none()` (currently around lines 20–24):

```rust
if name_filter.is_none() {
    println!("{}", build_file_summary_json(&fc));
    return Ok(());
}
```

- [ ] **Step 5: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Verify the `use serde::Serialize` import is not duplicated**

`serde` is already a dependency in `Cargo.toml` with the `derive` feature. No `Cargo.toml` changes needed.

- [ ] **Step 7: Commit**

```bash
git add src/commands/get.rs
git commit -m "feat: get file summary strips internal fields for agent consumption"
```
