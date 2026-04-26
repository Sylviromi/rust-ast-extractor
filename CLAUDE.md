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

## Architecture

```
src/
├── main.rs              — clap CLI (index + get subcommands)
├── project.rs           — find_project_root(): walks parents for Cargo.toml/.git
├── cache/
│   ├── schema.rs        — FileCache, ExtractedItem, ItemKind (serde types)
│   └── mod.rs           — compute_hash, cache_path_for_file, read/write/merge_items
├── extractor/
│   ├── mod.rs           — extract_file(): parse file with syn, return Vec<ExtractedItem>
│   └── visitor.rs       — ItemVisitor (syn::Visit) collecting all item kinds
└── commands/
    ├── index.rs         — run_index(): walk files, compute hash, merge cache
    └── get.rs           — run_get(): auto-index, return JSON or raw_source
```

**Data flow:** `commands` → `extractor` → `cache`. Extractor and cache modules are independent; commands glues them.

**Cache location:** `<project-root>/.ast-cache/files/<relative-path>.json`

**Cache invalidation:** Per-file hash check (skip if unchanged). Per-item hash merge (preserve unchanged items, update changed, drop removed).

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
    "line_start": 10,
    "line_end": 15,
    "item_hash": "sha256:...",
    "raw_source": "pub fn my_fn(x: u32) -> String { ... }"
  }]
}
```

`kind` is one of: `fn`, `struct`, `enum`, `trait`, `impl`, `type`, `const`, `macro`, `mod`
