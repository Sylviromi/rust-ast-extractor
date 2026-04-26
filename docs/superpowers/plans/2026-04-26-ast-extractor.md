# rust-ast-extractor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a CLI tool that extracts structured data from Rust source files into a JSON cache, enabling fast AI-assisted lookup without re-reading raw source.

**Architecture:** Three independent layers — `extractor` (syn-based AST parser), `cache` (JSON read/write with per-item hashing), and `commands` (glue between the two). `project.rs` locates the project root; `main.rs` wires up the clap CLI.

**Tech Stack:** Rust 2024, `syn` (full), `proc-macro2` (span-locations), `quote`, `serde_json`, `sha2`, `clap` (derive), `walkdir`, `chrono`

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Dependencies |
| `src/main.rs` | clap CLI, dispatches to commands |
| `src/project.rs` | Walk parent dirs to find project root |
| `src/cache/schema.rs` | `ItemKind`, `ExtractedItem`, `FileCache` serde structs |
| `src/cache/mod.rs` | `compute_hash`, `cache_path_for_file`, `read_cache`, `write_cache`, `merge_items` |
| `src/extractor/visitor.rs` | `ItemVisitor` implementing `syn::visit::Visit` |
| `src/extractor/mod.rs` | `extract_file(path, source) -> Vec<ExtractedItem>` |
| `src/commands/index.rs` | `run_index(path)` — walk, extract, merge, write cache |
| `src/commands/get.rs` | `run_get(target)` — parse target, auto-index, print result |
| `tests/integration.rs` | End-to-end index + get tests |

---

## Task 1: Cargo.toml and project scaffold

**Files:**
- Modify: `Cargo.toml`
- Create: `src/project.rs`, `src/cache/mod.rs`, `src/cache/schema.rs`, `src/extractor/mod.rs`, `src/extractor/visitor.rs`, `src/commands/index.rs`, `src/commands/get.rs`

- [ ] **Step 1: Update Cargo.toml**

Replace the entire `[dependencies]` section:

```toml
[package]
name = "rust-ast-extractor"
version = "0.1.0"
edition = "2024"

[dependencies]
syn = { version = "2", features = ["full", "extra-traits"] }
proc-macro2 = { version = "1", features = ["span-locations"] }
quote = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
clap = { version = "4", features = ["derive"] }
walkdir = "2"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create empty module files**

Create `src/project.rs`:
```rust
use std::path::{Path, PathBuf};
```

Create `src/cache/schema.rs`:
```rust
```

Create `src/cache/mod.rs`:
```rust
pub mod schema;
```

Create `src/extractor/visitor.rs`:
```rust
```

Create `src/extractor/mod.rs`:
```rust
pub mod visitor;
```

Create `src/commands/index.rs`:
```rust
```

Create `src/commands/get.rs`:
```rust
```

- [ ] **Step 3: Update src/main.rs to declare modules**

```rust
mod cache;
mod commands;
mod extractor;
mod project;

fn main() {
    println!("Hello, world!");
}
```

Create `src/commands/mod.rs`:
```rust
pub mod get;
pub mod index;
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo check
```

Expected: no errors (warnings about unused imports are fine).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/
git commit -m "chore: add dependencies and module scaffold"
```

---

## Task 2: project.rs — locate project root

**Files:**
- Modify: `src/project.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/project.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn find_project_root(start: &Path) -> PathBuf {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_root_via_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src = tmp.path().join("src");
        fs::create_dir(&src).unwrap();

        let root = find_project_root(&src);
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn finds_root_via_git() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        let deep = tmp.path().join("a").join("b");
        fs::create_dir_all(&deep).unwrap();

        let root = find_project_root(&deep);
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn falls_back_to_start_when_no_marker() {
        let tmp = TempDir::new().unwrap();
        let root = find_project_root(tmp.path());
        assert_eq!(root, tmp.path());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test project::tests
```

Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement find_project_root**

Replace `todo!()` with:

```rust
pub fn find_project_root(start: &Path) -> PathBuf {
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if current.join("Cargo.toml").exists() || current.join(".git").exists() {
            return current;
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return start.to_path_buf(),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test project::tests
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/project.rs
git commit -m "feat: implement project root detection"
```

---

## Task 3: cache/schema.rs — data types

**Files:**
- Modify: `src/cache/schema.rs`

- [ ] **Step 1: Write the failing test**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Const,
    Macro,
    Mod,
}

impl std::fmt::Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ItemKind::Fn => "fn",
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Trait => "trait",
            ItemKind::Impl => "impl",
            ItemKind::Type => "type",
            ItemKind::Const => "const",
            ItemKind::Macro => "macro",
            ItemKind::Mod => "mod",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedItem {
    pub kind: ItemKind,
    pub name: String,
    pub visibility: String,
    pub signature: String,
    pub docs: String,
    pub attributes: Vec<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub item_hash: String,
    pub raw_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    pub file: String,
    pub file_hash: String,
    pub indexed_at: String,
    pub items: Vec<ExtractedItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cache() -> FileCache {
        FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:abc".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            items: vec![ExtractedItem {
                kind: ItemKind::Fn,
                name: "my_fn".into(),
                visibility: "pub".into(),
                signature: "pub fn my_fn()".into(),
                docs: "Does a thing.".into(),
                attributes: vec!["#[inline]".into()],
                line_start: 1,
                line_end: 3,
                item_hash: "sha256:def".into(),
                raw_source: "pub fn my_fn() {}".into(),
            }],
        }
    }

    #[test]
    fn roundtrip_serialization() {
        let cache = sample_cache();
        let json = serde_json::to_string(&cache).unwrap();
        let decoded: FileCache = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.file, cache.file);
        assert_eq!(decoded.items[0].kind, ItemKind::Fn);
        assert_eq!(decoded.items[0].name, "my_fn");
    }

    #[test]
    fn item_kind_serializes_lowercase() {
        let json = serde_json::to_string(&ItemKind::Fn).unwrap();
        assert_eq!(json, r#""fn""#);
        let json = serde_json::to_string(&ItemKind::Impl).unwrap();
        assert_eq!(json, r#""impl""#);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass** (structs defined, so they should pass immediately)

```bash
cargo test cache::schema::tests
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/cache/schema.rs
git commit -m "feat: add cache schema types (FileCache, ExtractedItem, ItemKind)"
```

---

## Task 4: cache/mod.rs — hashing, read, write, merge

**Files:**
- Modify: `src/cache/mod.rs`

- [ ] **Step 1: Write the failing tests**

```rust
pub mod schema;

use schema::{ExtractedItem, FileCache, ItemKind};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn cache_path_for_file(project_root: &Path, source_file: &Path) -> PathBuf {
    todo!()
}

pub fn read_cache(cache_file: &Path) -> Option<FileCache> {
    todo!()
}

pub fn write_cache(cache_file: &Path, cache: &FileCache) -> std::io::Result<()> {
    todo!()
}

/// Merges freshly extracted items with the existing cache.
/// If an item's hash is unchanged, the existing entry is kept (preserving future ai_summary fields).
/// Items no longer in the source are dropped.
pub fn merge_items(
    new_items: Vec<ExtractedItem>,
    existing: Option<&FileCache>,
) -> Vec<ExtractedItem> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_item(name: &str, hash: &str) -> ExtractedItem {
        ExtractedItem {
            kind: ItemKind::Fn,
            name: name.into(),
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
        assert_eq!(cache, Path::new("/project/.ast-cache/files/src/lib.rs.json"));
    }

    #[test]
    fn write_and_read_cache_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let cache_file = tmp.path().join("test.json");
        let fc = FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:abc".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            items: vec![make_item("foo", "sha256:111")],
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
            items: vec![make_item("foo", "sha256:111")],
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
            items: vec![make_item("foo", "sha256:old_hash")],
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
            items: vec![make_item("foo", "sha256:111"), make_item("bar", "sha256:222")],
        };
        // "bar" is gone from source
        let new_items = vec![make_item("foo", "sha256:111")];
        let merged = merge_items(new_items, Some(&existing));
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "foo");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test cache::tests
```

Expected: FAIL on `cache_path_for_file`, `write_cache`, `read_cache`, `merge_items` (all `todo!()`).

- [ ] **Step 3: Implement the functions**

Replace the `todo!()` bodies:

```rust
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

pub fn merge_items(
    new_items: Vec<ExtractedItem>,
    existing: Option<&FileCache>,
) -> Vec<ExtractedItem> {
    new_items
        .into_iter()
        .map(|item| {
            if let Some(cached) = existing {
                if let Some(existing_item) = cached
                    .items
                    .iter()
                    .find(|e| e.name == item.name && e.kind == item.kind)
                {
                    if existing_item.item_hash == item.item_hash {
                        return existing_item.clone();
                    }
                }
            }
            item
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test cache::tests
```

Expected: 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/cache/mod.rs
git commit -m "feat: implement cache read/write/merge with per-item hashing"
```

---

## Task 5: extractor/visitor.rs — syn AST visitor

**Files:**
- Modify: `src/extractor/visitor.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/extractor/visitor.rs`:

```rust
use crate::cache::schema::{ExtractedItem, ItemKind};
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::visit::Visit;

pub struct ItemVisitor<'src> {
    pub items: Vec<ExtractedItem>,
    source: &'src str,
}

impl<'src> ItemVisitor<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            items: Vec::new(),
            source,
        }
    }
}

// — helpers —

fn extract_docs(attrs: &[syn::Attribute]) -> String {
    todo!()
}

fn extract_non_doc_attrs(attrs: &[syn::Attribute]) -> Vec<String> {
    todo!()
}

fn visibility_str(vis: &syn::Visibility) -> String {
    vis.to_token_stream().to_string()
}

fn extract_lines(source: &str, start_line: usize, end_line: usize) -> String {
    source
        .lines()
        .enumerate()
        .filter(|(i, _)| *i + 1 >= start_line && *i + 1 <= end_line)
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn item_hash(tokens: proc_macro2::TokenStream) -> String {
    crate::cache::compute_hash(&tokens.to_string())
}

// — visitor impl —

impl<'src, 'ast> Visit<'ast> for ItemVisitor<'src> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        todo!()
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        todo!()
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        todo!()
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        todo!()
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        todo!()
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        todo!()
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        todo!()
    }

    fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
        todo!()
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::visit::Visit;

    fn collect(src: &str) -> Vec<ExtractedItem> {
        let file = syn::parse_str::<syn::File>(src).expect("parse failed");
        let mut visitor = ItemVisitor::new(src);
        visitor.visit_file(&file);
        visitor.items
    }

    #[test]
    fn extracts_public_fn() {
        let src = r#"/// Does a thing.
pub fn hello(x: u32) -> String {
    x.to_string()
}"#;
        let items = collect(src);
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.kind, ItemKind::Fn);
        assert_eq!(item.name, "hello");
        assert_eq!(item.visibility, "pub");
        assert!(item.signature.contains("pub fn hello"), "got: {}", item.signature);
        assert_eq!(item.docs, "Does a thing.");
        assert!(item.raw_source.contains("pub fn hello"));
        assert!(!item.item_hash.is_empty());
    }

    #[test]
    fn extracts_struct() {
        let src = "pub struct Foo { pub x: u32 }";
        let items = collect(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Struct);
        assert_eq!(items[0].name, "Foo");
    }

    #[test]
    fn extracts_enum() {
        let src = "pub enum Color { Red, Green, Blue }";
        let items = collect(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Enum);
        assert_eq!(items[0].name, "Color");
    }

    #[test]
    fn extracts_trait() {
        let src = "pub trait Animal { fn speak(&self); }";
        let items = collect(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Trait);
        assert_eq!(items[0].name, "Animal");
    }

    #[test]
    fn extracts_impl_and_methods() {
        let src = r#"
struct Dog;
impl Dog {
    pub fn bark(&self) {}
}
"#;
        let items = collect(src);
        let kinds: Vec<_> = items.iter().map(|i| (&i.kind, i.name.as_str())).collect();
        assert!(kinds.contains(&(&ItemKind::Struct, "Dog")));
        assert!(kinds.contains(&(&ItemKind::Impl, "Dog")));
        assert!(kinds.contains(&(&ItemKind::Fn, "bark")));
    }

    #[test]
    fn extracts_type_alias() {
        let src = "pub type Result<T> = std::result::Result<T, String>;";
        let items = collect(src);
        assert_eq!(items[0].kind, ItemKind::Type);
        assert_eq!(items[0].name, "Result");
    }

    #[test]
    fn extracts_const() {
        let src = "pub const MAX: u32 = 100;";
        let items = collect(src);
        assert_eq!(items[0].kind, ItemKind::Const);
        assert_eq!(items[0].name, "MAX");
    }

    #[test]
    fn extracts_mod() {
        let src = "pub mod utils {}";
        let items = collect(src);
        assert!(items.iter().any(|i| i.kind == ItemKind::Mod && i.name == "utils"));
    }

    #[test]
    fn separates_doc_from_other_attrs() {
        let src = r#"
#[inline]
#[allow(dead_code)]
/// My function.
pub fn foo() {}
"#;
        let items = collect(src);
        assert_eq!(items[0].docs, "My function.");
        assert_eq!(items[0].attributes.len(), 2);
        assert!(items[0].attributes.iter().any(|a| a.contains("inline")));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test extractor::visitor::tests
```

Expected: FAIL with "not yet implemented".

- [ ] **Step 3: Implement helper functions**

```rust
fn extract_docs(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    return Some(s.value().trim().to_string());
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_non_doc_attrs(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc"))
        .map(|a| a.to_token_stream().to_string())
        .collect()
}
```

- [ ] **Step 4: Implement visit_item_fn**

```rust
fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
    let span = i.span();
    let start = span.start().line;
    let end = span.end().line;
    let raw = extract_lines(self.source, start, end);
    let vis = visibility_str(&i.vis);
    let sig = format!("{} {}", vis, i.sig.to_token_stream()).trim().to_string();

    self.items.push(ExtractedItem {
        kind: ItemKind::Fn,
        name: i.sig.ident.to_string(),
        visibility: vis,
        signature: sig,
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: raw,
    });
    // do not recurse into fn body — top-level fns only at this level
}
```

- [ ] **Step 5: Implement visit_item_struct, visit_item_enum, visit_item_trait**

```rust
fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let vis = visibility_str(&i.vis);
    self.items.push(ExtractedItem {
        kind: ItemKind::Struct,
        name: i.ident.to_string(),
        visibility: vis.clone(),
        signature: format!("{} struct {}", vis, i.ident).trim().to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
}

fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let vis = visibility_str(&i.vis);
    self.items.push(ExtractedItem {
        kind: ItemKind::Enum,
        name: i.ident.to_string(),
        visibility: vis.clone(),
        signature: format!("{} enum {}", vis, i.ident).trim().to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
}

fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let vis = visibility_str(&i.vis);
    self.items.push(ExtractedItem {
        kind: ItemKind::Trait,
        name: i.ident.to_string(),
        visibility: vis.clone(),
        signature: format!("{} trait {}", vis, i.ident).trim().to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
    // do not recurse — trait methods are not extracted as separate items
}
```

- [ ] **Step 6: Implement visit_item_impl**

```rust
fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);

    let name = if let Some((_, trait_path, _)) = &i.trait_ {
        format!(
            "{} for {}",
            trait_path.to_token_stream(),
            i.self_ty.to_token_stream()
        )
    } else {
        i.self_ty.to_token_stream().to_string()
    };

    let sig = format!("impl {name}");

    self.items.push(ExtractedItem {
        kind: ItemKind::Impl,
        name: name.clone(),
        visibility: String::new(),
        signature: sig,
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });

    // recurse into impl to collect methods as Fn items
    for item in &i.items {
        if let syn::ImplItem::Fn(method) = item {
            let mspan = method.span();
            let (ms, me) = (mspan.start().line, mspan.end().line);
            let vis = visibility_str(&method.vis);
            let msig = format!("{} {}", vis, method.sig.to_token_stream()).trim().to_string();
            self.items.push(ExtractedItem {
                kind: ItemKind::Fn,
                name: method.sig.ident.to_string(),
                visibility: vis,
                signature: msig,
                docs: extract_docs(&method.attrs),
                attributes: extract_non_doc_attrs(&method.attrs),
                line_start: ms as u32,
                line_end: me as u32,
                item_hash: item_hash(method.to_token_stream()),
                raw_source: extract_lines(self.source, ms, me),
            });
        }
    }
}
```

- [ ] **Step 7: Implement remaining visitors**

```rust
fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let vis = visibility_str(&i.vis);
    self.items.push(ExtractedItem {
        kind: ItemKind::Type,
        name: i.ident.to_string(),
        visibility: vis.clone(),
        signature: format!("{} type {}", vis, i.ident).trim().to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
}

fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let vis = visibility_str(&i.vis);
    self.items.push(ExtractedItem {
        kind: ItemKind::Const,
        name: i.ident.to_string(),
        visibility: vis.clone(),
        signature: format!("{} const {}: {}", vis, i.ident, i.ty.to_token_stream())
            .trim()
            .to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
}

fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let name = i
        .ident
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| i.mac.path.to_token_stream().to_string());
    self.items.push(ExtractedItem {
        kind: ItemKind::Macro,
        name,
        visibility: String::new(),
        signature: i.mac.path.to_token_stream().to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
}

fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
    let span = i.span();
    let (start, end) = (span.start().line, span.end().line);
    let vis = visibility_str(&i.vis);
    self.items.push(ExtractedItem {
        kind: ItemKind::Mod,
        name: i.ident.to_string(),
        visibility: vis.clone(),
        signature: format!("{} mod {}", vis, i.ident).trim().to_string(),
        docs: extract_docs(&i.attrs),
        attributes: extract_non_doc_attrs(&i.attrs),
        line_start: start as u32,
        line_end: end as u32,
        item_hash: item_hash(i.to_token_stream()),
        raw_source: extract_lines(self.source, start, end),
    });
    // recurse into inline mod bodies to find nested items
    syn::visit::visit_item_mod(self, i);
}
```

- [ ] **Step 8: Add required use statements at the top of the file**

The top of `src/extractor/visitor.rs` must have:

```rust
use crate::cache::schema::{ExtractedItem, ItemKind};
use proc_macro2::Span;
use quote::ToTokens;
use syn::visit::Visit;
```

- [ ] **Step 9: Run tests to verify they pass**

```bash
cargo test extractor::visitor::tests
```

Expected: 9 tests pass.

- [ ] **Step 10: Commit**

```bash
git add src/extractor/visitor.rs
git commit -m "feat: implement syn visitor for all Rust item kinds"
```

---

## Task 6: extractor/mod.rs — parse a file

**Files:**
- Modify: `src/extractor/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
pub mod visitor;

use crate::cache::schema::ExtractedItem;
use std::path::Path;

pub fn extract_file(path: &Path, source: &str) -> Result<Vec<ExtractedItem>, syn::Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::schema::ItemKind;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn extracts_items_from_file_content() {
        let src = r#"
/// A greeting.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}")
}

pub struct Config {
    pub debug: bool,
}
"#;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("lib.rs");
        fs::write(&path, src).unwrap();

        let items = extract_file(&path, src).unwrap();
        assert!(items.iter().any(|i| i.kind == ItemKind::Fn && i.name == "greet"));
        assert!(items.iter().any(|i| i.kind == ItemKind::Struct && i.name == "Config"));
    }

    #[test]
    fn returns_error_on_invalid_rust() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.rs");
        let bad = "fn broken( {";
        fs::write(&path, bad).unwrap();
        assert!(extract_file(&path, bad).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test extractor::tests
```

Expected: FAIL with "not yet implemented".

- [ ] **Step 3: Implement extract_file**

```rust
pub fn extract_file(path: &Path, source: &str) -> Result<Vec<ExtractedItem>, syn::Error> {
    let file = syn::parse_str::<syn::File>(source)?;
    let mut visitor = visitor::ItemVisitor::new(source);
    syn::visit::Visit::visit_file(&mut visitor, &file);
    Ok(visitor.items)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test extractor::tests
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/extractor/mod.rs
git commit -m "feat: implement extract_file using syn visitor"
```

---

## Task 7: commands/index.rs — index a file or directory

**Files:**
- Modify: `src/commands/index.rs`

- [ ] **Step 1: Write the failing tests**

```rust
use crate::{
    cache::{self, schema::FileCache},
    extractor,
    project::find_project_root,
};
use chrono;
use std::path::Path;
use walkdir::WalkDir;

pub fn run_index(path: &Path) -> anyhow::Result<()> {
    todo!()
}

fn index_single_file(source_file: &Path, project_root: &Path) -> anyhow::Result<()> {
    todo!()
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

        assert!(tmp.path().join(".ast-cache/files/src/main.rs.json").exists());
        assert!(tmp.path().join(".ast-cache/files/src/lib.rs.json").exists());
    }
}
```

Also add `anyhow` to `Cargo.toml` dependencies:

```toml
anyhow = "1"
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test commands::index::tests
```

Expected: FAIL (compile error on `anyhow` not found, or `todo!()` panics).

- [ ] **Step 3: Implement index_single_file**

```rust
fn index_single_file(source_file: &Path, project_root: &Path) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(source_file)?;
    let file_hash = cache::compute_hash(&source);
    let cache_file = cache::cache_path_for_file(project_root, source_file);

    // Check if file hash matches — skip if unchanged
    if let Some(existing) = cache::read_cache(&cache_file) {
        if existing.file_hash == file_hash {
            return Ok(());
        }
        // File changed: extract and merge
        let new_items = extractor::extract_file(source_file, &source)
            .map_err(|e| anyhow::anyhow!("parse error in {}: {}", source_file.display(), e))?;
        let merged = cache::merge_items(new_items, Some(&existing));
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
    } else {
        // No cache yet: extract fresh
        let new_items = extractor::extract_file(source_file, &source)
            .map_err(|e| anyhow::anyhow!("parse error in {}: {}", source_file.display(), e))?;
        let fc = FileCache {
            file: source_file
                .strip_prefix(project_root)
                .unwrap_or(source_file)
                .to_string_lossy()
                .to_string(),
            file_hash,
            indexed_at: chrono::Utc::now().to_rfc3339(),
            items: new_items,
        };
        cache::write_cache(&cache_file, &fc)?;
    }

    Ok(())
}
```

- [ ] **Step 4: Implement run_index**

```rust
pub fn run_index(path: &Path) -> anyhow::Result<()> {
    let project_root = find_project_root(path);

    if path.is_file() {
        let rel = path
            .strip_prefix(&project_root)
            .unwrap_or(path)
            .display()
            .to_string();
        eprintln!("indexing {rel}");
        index_single_file(path, &project_root)?;
    } else {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let rel = entry
                .path()
                .strip_prefix(&project_root)
                .unwrap_or(entry.path())
                .display()
                .to_string();
            eprintln!("indexing {rel}");
            index_single_file(entry.path(), &project_root)?;
        }
    }

    Ok(())
}
```

Add `use chrono;` to the top of the file (it's already in deps).

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test commands::index::tests
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/commands/index.rs
git commit -m "feat: implement index command with file hashing and cache write"
```

---

## Task 8: commands/get.rs — get file summary or specific item

**Files:**
- Modify: `src/commands/get.rs`

- [ ] **Step 1: Write the failing tests**

```rust
use crate::{cache, commands::index::run_index, project::find_project_root};
use std::path::Path;

/// `target` is either `"path/to/file.rs"` or `"path/to/file.rs::item_name"`
/// or `"path/to/file.rs::kind::item_name"` for disambiguation.
pub fn run_get(target: &str) -> anyhow::Result<()> {
    todo!()
}

fn parse_target(target: &str) -> (&str, Option<&str>, Option<&str>) {
    // Returns (file_path, optional_kind, optional_name)
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::schema::{FileCache, ItemKind};
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

    use std::fs;

    #[test]
    fn get_file_returns_json_cache() {
        let src = "pub fn hello() {}\npub struct World;";
        let (_tmp, file) = setup_indexed_project(src);

        // Capture stdout by reading cache directly (since run_get prints to stdout)
        let project_root = find_project_root(&file);
        let cache_path = cache::cache_path_for_file(&project_root, &file);
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
        let cache_path = cache::cache_path_for_file(&project_root, &file);
        assert!(cache_path.exists(), "auto-index should have created cache");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test commands::get::tests
```

Expected: FAIL with "not yet implemented".

- [ ] **Step 3: Implement parse_target**

```rust
fn parse_target(target: &str) -> (&str, Option<&str>, Option<&str>) {
    match target.split_once("::") {
        None => (target, None, None),
        Some((file, rest)) => match rest.split_once("::") {
            Some((kind, name)) => (file, Some(kind), Some(name)),
            None => (file, None, Some(rest)),
        },
    }
}
```

- [ ] **Step 4: Implement run_get**

```rust
pub fn run_get(target: &str) -> anyhow::Result<()> {
    let (file_str, kind_filter, name_filter) = parse_target(target);
    let file_path = std::path::PathBuf::from(file_str);
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
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test commands::get::tests
```

Expected: 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/commands/get.rs
git commit -m "feat: implement get command with auto-index and item lookup"
```

---

## Task 9: main.rs — clap CLI wiring

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement main.rs**

```rust
mod cache;
mod commands;
mod extractor;
mod project;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rust-ast-extractor",
    about = "Extract structured data from Rust source files into a JSON cache"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a Rust file or directory (recursive). Skips unchanged files.
    Index {
        /// Path to a .rs file or directory
        path: PathBuf,
    },
    /// Get JSON summary of a file, or raw source of a specific item.
    ///
    /// Examples:
    ///   get src/lib.rs
    ///   get src/lib.rs::my_function
    ///   get src/lib.rs::fn::my_function
    Get {
        /// File path, optionally with ::item or ::kind::item suffix
        target: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Index { path } => commands::index::run_index(&path),
        Commands::Get { target } => commands::get::run_get(&target),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Build and run a manual smoke test**

```bash
cargo build
```

Expected: compiles with no errors.

```bash
echo 'pub fn hello(x: u32) -> String { x.to_string() }' > /tmp/test_extract.rs
./target/debug/rust-ast-extractor index /tmp/test_extract.rs
```

Expected: prints `indexing test_extract.rs` (or similar path).

```bash
./target/debug/rust-ast-extractor get /tmp/test_extract.rs
```

Expected: JSON with one item, `"name": "hello"`, `"kind": "fn"`.

```bash
./target/debug/rust-ast-extractor get /tmp/test_extract.rs::hello
```

Expected: prints `pub fn hello(x: u32) -> String { x.to_string() }`.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up clap CLI with index and get subcommands"
```

---

## Task 10: Integration tests and CLAUDE.md update

**Files:**
- Create: `tests/integration.rs`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Write integration tests**

Create `tests/integration.rs`:

```rust
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("rust-ast-extractor")
}

fn setup_project(src: &str) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "[package]\nname=\"test\"\nversion=\"0.1.0\"\nedition=\"2024\"").unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let file = src_dir.join("lib.rs");
    fs::write(&file, src).unwrap();
    (tmp, file)
}

#[test]
fn index_and_get_file() {
    let src = r#"
/// Greets someone.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}")
}

pub struct Config {
    pub debug: bool,
}
"#;
    let (tmp, file) = setup_project(src);
    let bin = binary();

    let index = Command::new(&bin)
        .args(["index", file.to_str().unwrap()])
        .output()
        .expect("failed to run binary");
    assert!(index.status.success(), "index failed: {:?}", String::from_utf8_lossy(&index.stderr));

    let get = Command::new(&bin)
        .args(["get", file.to_str().unwrap()])
        .output()
        .expect("failed to run binary");
    assert!(get.status.success());

    let json: serde_json::Value = serde_json::from_slice(&get.stdout).expect("invalid JSON output");
    let items = json["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["name"] == "greet" && i["kind"] == "fn"));
    assert!(items.iter().any(|i| i["name"] == "Config" && i["kind"] == "struct"));

    let greet_item = items.iter().find(|i| i["name"] == "greet").unwrap();
    assert_eq!(greet_item["docs"], "Greets someone.");
    assert!(greet_item["signature"].as_str().unwrap().contains("pub fn greet"));
}

#[test]
fn get_specific_item_returns_raw_source() {
    let src = "pub fn target_fn(x: u32) -> u32 { x * 2 }\npub fn other() {}";
    let (tmp, file) = setup_project(src);
    let bin = binary();

    let target = format!("{}::target_fn", file.to_str().unwrap());
    let get = Command::new(&bin)
        .args(["get", &target])
        .output()
        .expect("failed to run binary");
    assert!(get.status.success(), "stderr: {}", String::from_utf8_lossy(&get.stderr));

    let output = String::from_utf8_lossy(&get.stdout);
    assert!(output.contains("target_fn"), "got: {output}");
    assert!(!output.contains("other"), "should not contain other fn: {output}");
}

#[test]
fn get_auto_indexes_unindexed_file() {
    let src = "pub const ANSWER: u32 = 42;";
    let (tmp, file) = setup_project(src);
    let bin = binary();

    // No index run — get should auto-index
    let get = Command::new(&bin)
        .args(["get", file.to_str().unwrap()])
        .output()
        .expect("failed to run binary");
    assert!(get.status.success(), "stderr: {}", String::from_utf8_lossy(&get.stderr));

    let json: serde_json::Value = serde_json::from_slice(&get.stdout).unwrap();
    assert!(json["items"].as_array().unwrap().iter().any(|i| i["name"] == "ANSWER"));
}

#[test]
fn reindex_unchanged_file_does_not_rewrite_cache() {
    let src = "pub fn stable() {}";
    let (tmp, file) = setup_project(src);
    let bin = binary();

    Command::new(&bin)
        .args(["index", file.to_str().unwrap()])
        .output()
        .unwrap();

    let cache_path = tmp
        .path()
        .join(".ast-cache/files/src/lib.rs.json");
    let mtime1 = fs::metadata(&cache_path).unwrap().modified().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(50));

    Command::new(&bin)
        .args(["index", file.to_str().unwrap()])
        .output()
        .unwrap();
    let mtime2 = fs::metadata(&cache_path).unwrap().modified().unwrap();

    assert_eq!(mtime1, mtime2, "cache was rewritten despite no change");
}
```

- [ ] **Step 2: Run integration tests**

```bash
cargo test --test integration
```

Expected: 4 tests pass.

- [ ] **Step 3: Update CLAUDE.md with run command**

Add to `CLAUDE.md` under Build & Run Commands:

```bash
cargo test --test integration   # run integration tests only
```

And update the Architecture section to reflect the actual implemented structure.

- [ ] **Step 4: Commit**

```bash
git add tests/integration.rs CLAUDE.md
git commit -m "test: add integration tests for index/get CLI commands"
```

---

## Completion Checklist

- [ ] All unit tests pass: `cargo test`
- [ ] All integration tests pass: `cargo test --test integration`
- [ ] `cargo clippy` produces no errors
- [ ] Manual smoke test: `rust-ast-extractor index src/` on itself works
- [ ] Manual smoke test: `rust-ast-extractor get src/main.rs` returns valid JSON
- [ ] Manual smoke test: `rust-ast-extractor get src/main.rs::main` returns raw source of `main`
