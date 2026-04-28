# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rust-ast-extractor` is a CLI tool that parses Rust source files and extracts structured data (functions, structs, enums, traits, impl blocks, type aliases, constants, macros, modules) into a JSON cache at `<project-root>/.ast-cache/`. Designed to be consumed by AI assistants for fast file/item lookup.

## Build & Run Commands

```bash
cargo build           # debug build
cargo build --release # optimized build
cargo run -- index src/   # index a directory
cargo run -- get src/main.rs  # get JSON summary
cargo run -- get src/main.rs::main  # get raw source of specific item
cargo test            # run all tests
cargo test <name>     # run a single test by name substring
cargo test --test integration  # run integration tests only
cargo clippy          # lint
cargo fmt             # format code
```

## CLI Usage

```
rust-ast-extractor index <path>              # index .rs file or directory recursively
rust-ast-extractor get <file>               # get JSON summary (auto-indexes if needed)
rust-ast-extractor get <file>::<item>       # get raw source of item
rust-ast-extractor get <file>::<kind>::<item>  # disambiguate by kind (fn/struct/impl/etc.)
```

## JSON Output Schema

```json
{
  "file": "src/lib.rs",
  "file_hash": "sha256:...",
  "indexed_at": "2026-04-26T...",
  "items": [{
    "kind": "fn",
    "name": "my_fn",
    "visibility": "pub",
    "signature": "pub fn my_fn(x: u32) -> String",
    "docs": "Doc comment text.",
    "attributes": ["#[inline]"],
    "item_hash": "sha256:...",
    "raw_source": "pub fn my_fn(x: u32) -> String { ... }"
  }]
}
```

`kind` is one of: `fn`, `struct`, `enum`, `trait`, `impl`, `type`, `const`, `macro`, `mod`

## AST Cache (rust-ast-extractor)

The project is indexed with [`rust-ast-extractor`](https://github.com/TcePrepK/rust-ast-extractor). The cache lives in
`.ast-cache/` (gitignored).
**Before reading a source file**, check the cache first — it's faster and gives you signatures, docs, and line numbers
without opening the file:

```bash
# Get structured summary of a file (items, signatures, docs)
rust-ast-extractor get src/app.rs

# Get raw source of one specific item
rust-ast-extractor get src/app.rs::App
rust-ast-extractor get src/handlers/feed_list.rs::handle_feed_list_input

# Re-index after editing source files
rust-ast-extractor index src/
```

**When to use it:**

- Before asking "what does X function do?" — `get src/file.rs::fn_name` gives you the source instantly
- When planning which files to touch — `get src/file.rs` shows all items with signatures and doc comments
- After making changes — re-index so the cache stays current

**Re-index rule:** Run `rust-ast-extractor index src/` at the start of any session where you plan to edit source files,
or after any significant changes. The tool skips unchanged files, so it's fast.

---

## Module Map

Run `rust-ast-extractor dir src/` for a live index of all source files and their responsibilities.
Each file's `//!` module doc is the authoritative description — it is never out of date.
