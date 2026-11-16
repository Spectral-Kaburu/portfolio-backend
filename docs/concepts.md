# Concepts to File Mapping

This document maps Rust concepts to the files where they are introduced or heavily used.

## M1 (Scan)

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

## M2 (Enrich)

| Concept | Files |
|---------|-------|
| Traits and Trait Objects | `src/ai_client.rs`, `tests/ai_client_tests.rs` |
| Async Traits (`async_trait`) | `src/ai_client.rs` |
| Dependency Injection (Clock) | `src/ai_client.rs`, `tests/ai_client_tests.rs` |
| Thread-Safe Interior Mutability (`Arc<Mutex<T>>`) | `src/ai_client.rs` |
| Blocking HTTP with `reqwest` | `src/ai_client.rs` |
| Regex for Redaction | `src/redact.rs`, `tests/redact_tests.rs` |
| SHA-256 Hashing for Cache Keys | `src/cache.rs` |
| Atomic File Writes | `src/enrich.rs` |
| JSON Schema Validation for AI Output | `src/enrich.rs`, `schemas/enriched.schema.json` |
| Mock HTTP Server Testing | `tests/ai_client_tests.rs` |
| Retry Logic with Exponential Backoff | `src/ai_client.rs` |