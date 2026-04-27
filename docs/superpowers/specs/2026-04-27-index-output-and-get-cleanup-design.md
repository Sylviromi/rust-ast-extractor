# Design: Index Output & Get Command Cleanup

**Date:** 2026-04-27

## Overview

Two focused improvements to CLI output and schema cleanliness:

1. `index` command should only report files that were actually updated, plus a summary.
2. `get <file>` (file summary mode) should return lean, agent-friendly JSON — no hashes, timestamps, line numbers, or raw source.

---

## TODO 1: Index Output

### Problem

`run_index` currently prints `"indexing {rel}"` for every `.rs` file it visits, including files that are skipped because their hash is unchanged. This produces noisy output when indexing a large project.

### Design

**`index_single_file`** changes return type from `anyhow::Result<()>` to `anyhow::Result<bool>`:
- Returns `true` if the file was re-indexed (hash changed or no cache).
- Returns `false` if the file was skipped (hash unchanged).

**`run_index` directory path:**
- Accumulates `updated` and `unchanged` counts.
- Prints `"  updated {rel}"` inline for each updated file.
- Prints a trailing summary: `"3 files updated, 9 unchanged."` (or `"All X files up to date."` if nothing changed).

**`run_index` single-file path:**
- Prints `"updated {rel}"` if re-indexed, `"{rel} up to date"` if skipped.

---

## TODO 2: Get Command Output Cleanup

### Problem

`get <file>` serializes the full `FileCache` struct including internal fields (`file_hash`, `indexed_at`, `item_hash`, `raw_source`, `line_start`, `line_end`) that are meaningless noise for an AI agent consuming this data. `raw_source` in the file summary is redundant because `get <file>::<item>` already returns raw source for specific items.

### Design

**Remove `line_start`/`line_end` entirely from the schema:**
- Delete the fields from `ExtractedItem` in `cache/schema.rs`.
- Remove all assignments in `extractor/visitor.rs` (all call sites just set them; they are never read).
- Remove from the test fixture in `cache/schema.rs`.
- Remove placeholder values from `cache/mod.rs` test helper.

**Lean output structs in `get.rs`:**

```rust
#[derive(Serialize)]
struct ItemSummary<'a> {
    kind: &'a ItemKind,
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
```

When `name_filter` is `None`, map `FileCache` → `FileSummary` and serialize that instead of the raw `FileCache`. The on-disk cache format is unchanged.

The `get <file>::<item>` path is unchanged — it already returns `raw_source` as plain text.

---

## Files Changed

| File | Change |
|------|--------|
| `src/commands/index.rs` | `index_single_file` returns `bool`; `run_index` prints updated files + summary |
| `src/cache/schema.rs` | Remove `line_start`, `line_end` from `ExtractedItem` |
| `src/cache/mod.rs` | Remove `line_start`/`line_end` from test helper struct literal |
| `src/extractor/visitor.rs` | Remove all `line_start`/`line_end` assignments |
| `src/commands/get.rs` | Add `FileSummary`/`ItemSummary`; use them in file-only output path |

No new files needed.
