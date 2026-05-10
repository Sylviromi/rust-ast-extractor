# synopsis

A CLI tool that parses Rust source files and extracts structured AST data — functions, structs, enums, traits, impl blocks, type aliases, constants, macros, and modules — into a JSON cache at `<project-root>/.ast-cache/`. Designed for fast file/item lookup by AI assistants and developer tooling.

## Installation

```bash
cargo install synopsis
```

## Usage

```bash
# Index a file or directory recursively
synopsis index src/

# Get a JSON summary of a file (auto-indexes if needed)
synopsis get src/main.rs

# Get the raw source of a specific item
synopsis get src/main.rs::my_fn

# Disambiguate by kind (fn/struct/impl/enum/trait/type/const/macro/mod)
synopsis get src/main.rs::fn::my_fn

# Get a method scoped to a specific impl block
synopsis get src/main.rs::MyStruct::my_method

# List all .rs files in a directory with their module-level doc comments
synopsis dir src/
```

## JSON Output Schema

`get <file>` returns a file summary:

```json
{
  "file": "src/lib.rs",
  "module_doc": "Top-level module doc.",
  "items": [{
    "kind": "fn",
    "name": "my_fn",
    "parent": "MyStruct",
    "visibility": "pub",
    "signature": "pub fn my_fn(x: u32) -> String",
    "docs": "Doc comment text.",
    "attributes": ["#[inline]"],
    "line_start": 42,
    "line_end": 45
  }]
}
```

`parent` is only present for methods extracted from an `impl` block.

`kind` is one of: `fn`, `struct`, `enum`, `trait`, `impl`, `type`, `const`, `macro`, `mod`

`dir <path>` returns a sorted list of files with their module docs:

```json
[
  { "file": "src/lib.rs", "module_doc": "Top-level module." },
  { "file": "src/utils.rs", "module_doc": "" }
]
```

## Cache

Indexed data is stored in `<project-root>/.ast-cache/files/<relative-path>.json`. The cache uses per-file and per-item hashing so re-indexing only updates what changed.

## License

MIT