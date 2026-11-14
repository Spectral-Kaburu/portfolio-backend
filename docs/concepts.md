# Concepts to File Mapping

This document maps Rust concepts to the files where they are introduced or heavily used in M1.

| Concept | Files |
|---------|-------|
| Ownership and Borrowing | `src/scan.rs`, `src/git.rs`, `src/languages.rs` |
| `Result` and `?` Operator | `src/config.rs`, `src/git.rs`, `src/scan.rs`, `src/facts.rs`, `src/state.rs` |
| Structs and `serde` Derive | `src/facts.rs`, `src/config.rs`, `src/git.rs`, `src/languages.rs` |
| `std::process::Command` | `src/git.rs`, `src/state.rs` |
| Modules | `src/lib.rs`, `src/main.rs`, all module files |
| `Option` | `src/git.rs`, `src/facts.rs`, `src/scan.rs` |
| Iterators | `src/languages.rs`, `src/facts.rs`, `src/git.rs` |
| `chrono` DateTime with serde | `src/facts.rs` |
| JSON Schema Validation | `src/facts.rs`, `schemas/facts.schema.json` |
| Temporary Directory Tests | `tests/integration_tests.rs` |
| TOML Configuration | `src/config.rs` |
| TUI with dialoguer | `src/init.rs` |
| Table Output with tabled | `src/dry_run.rs` |
| WalkDir for Directory Traversal | `src/init.rs`, `src/scan.rs` |
| HashMap for Grouping | `src/languages.rs` |
| BTreeMap for Deterministic Ordering | `src/state.rs` |
| anyhow Context for Errors | All source files |