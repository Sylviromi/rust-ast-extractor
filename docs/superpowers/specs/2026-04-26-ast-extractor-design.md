# rust-ast-extractor Design Spec

**Date:** 2026-04-26
**Status:** Approved

## Purpose

A CLI tool that parses Rust source files using `syn` and extracts structured data (functions, structs, enums, traits, impl blocks, type aliases, constants, macros, module declarations) into a JSON cache. The cache is designed to be consumed by AI assistants (e.g. Claude Code) in other projects — enabling fast file/item lookup without reading raw source on every query.

---

## CLI Interface

```
rust-ast-extractor index <path>           # index a file or directory (recursive)
rust-ast-extractor get <file>             # get JSON summary of a file
rust-ast-extractor get <file>::<item>     # get full raw source of a specific item
```

**Behaviour:**
- `index <path>`: if a directory, walks all `.rs` files recursively. Skips files/items whose hash is unchanged.
- `get <file>`: if the file is not yet indexed, silently runs `index` first, then returns the JSON.
- `get <file>::<item>`: resolves item by `name`. When names are ambiguous (e.g. multiple `impl` blocks), a `kind` prefix disambiguates: `impl::MyStruct`. If still ambiguous, returns all matches. Auto-indexes if needed. Returns the `raw_source` field verbatim.
- Output always goes to **stdout** (JSON for `get`, progress lines for `index`).
- Errors go to **stderr** with a non-zero exit code.

---

## Cache Location

The cache lives at `<project-root>/.ast-cache/files/<relative-path>.json`.

Project root is determined by walking parent directories until `Cargo.toml` or `.git` is found. Falls back to the current working directory if neither is found.

Example: `src/parser/mod.rs` → `.ast-cache/files/src/parser/mod.rs.json`

---

## JSON Schema

Each cached file produces one JSON document:

```json
{
  "file": "src/lib.rs",
  "file_hash": "sha256:abc123...",
  "indexed_at": "2026-04-26T12:00:00Z",
  "items": [
    {
      "kind": "fn",
      "name": "parse_tokens",
      "visibility": "pub",
      "signature": "pub fn parse_tokens(input: &str) -> Vec<Token>",
      "docs": "Parses a raw input string into a token list.",
      "attributes": ["#[inline]"],
      "line_start": 12,
      "line_end": 28,
      "item_hash": "sha256:def456...",
      "raw_source": "pub fn parse_tokens(input: &str) -> Vec<Token> {\n    ...\n}"
    }
  ]
}
```

**Field notes:**
- `kind`: one of `fn`, `struct`, `enum`, `trait`, `impl`, `type`, `const`, `macro`, `mod`
- `item_hash`: SHA-256 of the item's token stream — used for per-item change detection
- `raw_source`: verbatim source text of the item (enables `get ::item` as a pure cache lookup)
- `docs`: stripped text from `///` or `/** */` doc comments directly above the item
- `signature`: for `fn`, the full signature without the body; for `struct`/`enum`, the declaration line(s)
- `attributes`: list of `#[...]` attributes on the item

---

## Module Architecture

```
src/
├── main.rs              — clap CLI setup, dispatches to subcommands
├── commands/
│   ├── index.rs         — walks files, calls extractor, writes cache
│   └── get.rs           — reads cache (auto-indexing if needed), prints result
├── extractor/
│   ├── mod.rs           — drives syn parsing, returns Vec<ExtractedItem>
│   └── visitor.rs       — syn::Visit impl collecting all item kinds
├── cache/
│   ├── mod.rs           — read/write JSON, per-item hash comparison
│   └── schema.rs        — serde structs matching the JSON schema
└── project.rs           — walks parent dirs to locate project root
```

**Data flow:** `commands` → `extractor` → `cache`. The `extractor` and `cache` modules have no knowledge of each other; `commands` glues them. `project.rs` is a standalone utility.

---

## Cache Invalidation

On `index <file>`:

1. Compute SHA-256 of the entire file. Compare with stored `file_hash`.
2. If **file hash matches** → skip (file unchanged).
3. If **file hash differs** (or no cache exists) → parse with `syn`, extract all items.
4. For each extracted item, compute `item_hash` from its token stream.
5. Compare against the cached entry with the same `name` + `kind`:
   - **Hash matches** → keep existing cached entry (preserves future Ollama summaries).
   - **Hash differs or new item** → replace with freshly extracted data.
6. Remove cached entries for items that no longer exist in source.
7. Write updated JSON with the new `file_hash`.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `syn` (features = full) | Rust AST parsing |
| `serde`, `serde_json` | JSON serialisation |
| `sha2` | SHA-256 hashing |
| `clap` (derive) | CLI argument parsing |
| `walkdir` | Recursive directory traversal |
| `chrono` | ISO 8601 timestamps |

---

## Phase 2: Ollama Integration

After core extraction and caching are stable, add an optional `--enrich` flag to `index`:

```
rust-ast-extractor index src/ --enrich
```

**Behaviour:**
- For each item lacking a `docs` field (or on explicit request), calls a local Ollama endpoint to generate a natural-language summary.
- Stores the result in a new `ai_summary` field alongside the existing JSON (does not overwrite `docs`).
- Optionally (`--write-docs`) writes generated doc comments back to the source file.
- Ollama model and endpoint are configurable via a `.ast-cache/config.json` file.

This phase is explicitly out of scope for the initial implementation.