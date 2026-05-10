# AGENTS.md

## Commands

```bash
cargo test              # all tests (unit + integration)
cargo test <substr>     # single test by name substring
cargo test --test integration  # integration tests only
cargo clippy            # lint
cargo fmt               # format
```

No CI, no pre-commit, no codegen.

## Architecture

Single crate, Rust edition 2024. CLI via clap derive, parsing via syn (features: `full`, `extra-traits`, `visit`).

- `src/main.rs` — entrypoint, defines `index`/`get`/`dir` subcommands
- `src/commands/` — one file per subcommand
- `src/extractor/` — syn visitor that walks AST extracting items
- `src/cache/` — sha256-based caching to `.ast-cache/files/<rel-path>.json`

## Key gotchas

- `get` **auto-indexes** if cache is missing — no need to run `index` first
- `get <file>` prints summary (no `raw_source`/`item_hash`); `get <file>::<item>` prints raw source string; multiple matches → JSON array
- Item targeting: `<file>`, `<file>::<name>`, `<file>::<kind>::<name>`, `<file>::<ParentType>::<method>`
- `dir` auto-indexes and sorts output alphabetically
- Cache is hash-addressable: re-indexing unchanged files is a no-op
- Project root discovery walks up looking for `Cargo.toml` or `.git`
- Trait methods are **not** extracted as separate items; impl methods **are** (with `parent` field)
- Unit tests are in-module `#[cfg(test)]`; integration tests in `tests/integration.rs` spawn the binary

## Cache-first workflow

The project indexes itself and other Rust repos. Before reading a `.rs` file:
```bash
cargo run -- get <file>
cargo run -- get <file>::<item>
```
After editing source, re-index: `cargo run -- index <path>`

## References

- `CLAUDE.md` — older guidance for Claude Code (may overlap)
- `.claude/settings.local.json` — local tool permissions
