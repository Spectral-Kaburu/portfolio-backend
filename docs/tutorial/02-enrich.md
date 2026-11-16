# Tutorial 2: AI Enrichment Pipeline

This tutorial covers the Rust concepts introduced in Milestone 2 (M2) of the portfolio engine - the AI enrichment pipeline that turns scanned git data into natural language prose.

## Concepts Covered

1. **Traits and Trait Objects** - Abstraction for testable HTTP clients
2. **Async Traits** - Asynchronous interfaces with `async_trait`
3. **Dependency Injection** - Injectable clock for testing time-based logic
4. **Thread-Safe Interior Mutability** - `Arc<Mutex<T>>` for shared state
5. **Blocking HTTP with `reqwest`** - Synchronous API calls
6. **Regex for Pattern Matching** - Redacting sensitive data
7. **SHA-256 Hashing** - Stable cache keys
8. **Atomic File Writes** - Temp file + rename for durability
9. **Schema Validation** - Validating AI output against JSON Schema

---

## 1. Traits and Trait Objects

**What it is:** Traits define shared behavior. `dyn Trait` is a trait object - a fat pointer to data + vtable for dynamic dispatch.

**Why Rust works this way:** Enables polymorphism without inheritance. Tests can swap real HTTP client for mock.

**Where it appears:**
- `src/ai_client.rs:56-65` - `ChatClient` trait with `chat()`, `list_models()`, `probe_model()`
- `src/ai_client.rs:136-140` - `HttpChatClient` struct implementing the trait
- `tests/ai_client_tests.rs` - Uses `HttpChatClient::new_with_clock()` for mock server tests

**Try breaking this:**
```rust
// In ai_client.rs, remove the trait bound
fn use_client(client: HttpChatClient) { ... } // Concrete type only
// Tests can no longer substitute a mock
```
**Expected error:** No compile error, but tests lose ability to inject mock behavior

---

## 2. Async Traits

**What it is:** `#[async_trait::async_trait]` generates a trait with async methods using heap allocation (BoxFuture).

**Why Rust works this way:** Native async traits require Generic Associated Types (GATs) not yet stable. The crate provides a workaround.

**Where it appears:**
- `src/ai_client.rs:55` - `#[async_trait::async_trait]` on `ChatClient`
- `src/ai_client.rs:57-64` - Async methods returning `anyhow::Result<...>`

**Try breaking this:**
```rust
// Remove the attribute
// #[async_trait::async_trait]
pub trait ChatClient {
    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse>;
}
// ERROR: `async fn` in traits are not supported without async_trait
```
**Expected error:** `async fn in traits are only supported in Rust 2024` (or similar GATs error)

---

## 3. Dependency Injection (Injectable Clock)

**What it is:** Passing a `Clock` trait implementation instead of using `chrono::Utc::now()` directly.

**Why Rust works this way:** Makes time a controllable dependency. Tests can control time precisely without sleeps.

**Where it appears:**
- `src/ai_client.rs:69-71` - `Clock` trait with `now() -> DateTime<Utc>`
- `src/ai_client.rs:74-81` - `SystemClock` production implementation
- `src/ai_client.rs:93-133` - `Throttle` takes `Arc<dyn Clock>`
- `tests/ai_client_tests.rs:333-337` - `TestClock` for deterministic tests

**Try breaking this:**
```rust
// In Throttle::wait(), use Utc::now() directly
let now = Utc::now(); // Instead of self.clock.now()
// Tests can no longer control time - become flaky/slow
```
**Expected issue:** Tests can't verify throttle behavior deterministically

---

## 4. Thread-Safe Interior Mutability (`Arc<Mutex<T>>`)

**What it is:** `Arc` (atomic reference count) + `Mutex` (mutual exclusion) = shared mutable state across threads.

**Why Rust works this way:** Rust's ownership allows either many immutable refs OR one mutable ref. `Mutex` enforces the rule at runtime; `Arc` allows shared ownership.

**Where it appears:**
- `src/ai_client.rs:88-89` - `Throttle` fields: `last_request: Arc<Mutex<DateTime<Utc>>>`
- `src/ai_client.rs:102-112` - `wait()` locks, checks time, sleeps if needed

**Try breaking this:**
```rust
// Remove the Mutex
last_request: Arc<DateTime<Utc>>, // Won't compile - can't mutate through Arc
// Or use RefCell instead of Mutex
last_request: Rc<RefCell<...>>, // ERROR: Rc not Send, can't cross threads
```
**Expected error:** `cannot borrow data in an 'Arc' as mutable` or `RefCell` not `Send`

---

## 5. Blocking HTTP with `reqwest`

**What it is:** `reqwest::blocking::Client` provides synchronous HTTP calls. No async runtime needed.

**Why Rust works this way:** Simpler for CLI tools. No Tokio/runtime overhead. Blocking calls are fine for sequential requests.

**Where it appears:**
- `src/ai_client.rs:144-158` - Client creation with timeouts
- `src/ai_client.rs:191-197` - POST request with JSON body
- `src/ai_client.rs:199-227` - Status code handling (401/403/404/429/5xx)

**Try breaking this:**
```rust
// Forget to check status code
let response = client.post(...).json(...).send()?;
let data: ChatResponse = response.json()?; // Crashes on 404/500 with parse error
```
**Expected issue:** Silent crashes on API errors instead of proper error messages

---

## 6. Regex for Pattern Matching

**What it is:** `regex::Regex` finds and replaces patterns. Compiled once, used many times.

**Why Rust works this way:** Compile-time checked patterns. No catastrophic backtracking (unlike PCRE).

**Where it appears:**
- `src/redact.rs:16-40` - `get_patterns()` compiles all regexes
- `src/redact.rs:42-75` - `redact()` applies each pattern, counts replacements
- `src/redact.rs:77-150` - Tests for every pattern with planted sentinels

**Try breaking this:**
```rust
// Compile regex inside the loop (slow!)
fn redact_slow(text: &str) -> String {
    let re = Regex::new(r"sk_[a-zA-Z0-9_]{20,}").unwrap();
    re.replace_all(text, "[REDACTED]").to_string()
}
// Performance: recompiles regex every call
```
**Expected issue:** 1000x slower. Always compile regex once.

---

## 7. SHA-256 Hashing

**What it is:** `sha2::Sha256` produces 32-byte deterministic hash. Same input = same output across runs/platforms.

**Why Rust works this way:** Cryptographically strong. Stable cache keys that don't depend on memory addresses or process IDs.

**Where it appears:**
- `src/cache.rs:42-45` - `make_key()` hashes digest + prompt_version + model
- `Cargo.toml` - `sha2 = "0.10"` dependency

**Try breaking this:**
```rust
// Use default hasher (randomized per run)
use std::collections::hash_map::DefaultHasher;
let mut hasher = DefaultHasher::new();
// Hash changes every process run - cache never hits!
```
**Expected issue:** Cache misses on every run. SHA-256 is stable across runs.

---

## 8. Atomic File Writes

**What it is:** Write to temp file, then `rename()` to target. Readers never see partial writes.

**Why Rust works this way:** `rename` is atomic on POSIX. No corruption if process crashes mid-write.

**Where it appears:**
- `src/enrich.rs` - (Will implement in draft writer)
- Pattern: `tempfile::NamedTempFile` + `persist()` or manual `std::fs::rename`

**Try breaking this:**
```rust
// Write directly to target
std::fs::write("draft.json", json)?;
// Crash after writing 50% -> corrupt file readers will parse
```
**Expected issue:** Partial JSON is invalid. Readers crash or get wrong data.

---

## 9. Schema Validation

**What it is:** `jsonschema` crate validates JSON against JSON Schema draft 2020-12.

**Why Rust works this way:** AI output is untrusted. Validation catches hallucinated fields, wrong types, missing required fields.

**Where it appears:**
- `schemas/enriched.schema.json` - Draft output schema
- `src/enrich.rs` - Will validate AI response before writing draft
- `src/facts.rs` - Validates `facts.json` against `facts.schema.json`

**Try breaking this:**
```rust
// Skip validation
let draft: Draft = serde_json::from_str(&ai_output)?;
// AI hallucinates extra field -> silently accepted or panics later
```
**Expected issue:** Invalid draft makes it to static site. Build fails or site breaks.

---

## Running the Code

```bash
# Build
cargo build

# Run all tests (lib + integration + ai_client)
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy

# Generate example config
cargo run -- init --example

# Scan repos (creates data/facts.json)
cargo run -- scan

# Dry run enrichment (prints request bodies)
cargo run -- enrich --dry-run

# Select projects for enrichment
cargo run -- ai-select

# List available AI models
cargo run -- ai-models

# Full enrichment (requires NVIDIA_API_KEY env var)
cargo run -- enrich
```

---

## Summary

M2 introduces patterns for building robust external API integrations:
- **Testability first:** Traits + dependency injection enable fast, deterministic tests
- **Safety with external systems:** Explicit error handling, retries, timeouts, validation
- **No secrets leakage:** Redaction runs on all text before leaving the process
- **Deterministic caching:** SHA-256 keys work across machines and restarts
- **Atomic outputs:** Readers never see partial/invalid files

These patterns generalize to any CLI tool that talks to HTTP APIs.