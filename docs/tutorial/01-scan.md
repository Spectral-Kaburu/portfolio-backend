# Tutorial 1: Scanning Git Repositories

This tutorial covers the Rust concepts introduced in Milestone 1 (M1) of the portfolio engine.

## Concepts Covered

1. **Ownership and Borrowing** - How Rust manages memory
2. **`Result` and the `?` Operator** - Error handling
3. **Structs and `serde` Derive** - Data structures with serialization
4. **`std::process::Command`** - Running external commands
5. **Modules** - Organizing code
6. **`Option`** - Handling absence of values
7. **Iterators** - Processing collections

---

## 1. Ownership and Borrowing

**What it is:** Rust's memory management system where each value has exactly one owner. When the owner goes out of scope, the value is dropped. References (&T, &mut T) let you borrow without taking ownership.

**Why Rust works this way:** Prevents memory leaks, double-frees, and data races at compile time without a garbage collector.

**Where it appears:**
- `src/scan.rs:113` - `repo_path` is borrowed to multiple functions
- `src/git.rs:36` - `dir: &Path` borrowed for git commands
- `src/languages.rs:135` - `tracked_files` borrowed for iteration

**Try breaking this:**
```rust
// In scan.rs, try to use repo_path after moving it
let owned = repo_path.to_path_buf(); // This moves the path
// process_repo(repo_path, ...) // ERROR: use of moved value
```
**Expected error:** `use of moved value: 'repo_path'`

---

## 2. `Result` and the `?` Operator

**What it is:** `Result<T, E>` represents either success (`Ok(T)`) or failure (`Err(E)`). The `?` operator propagates errors up the call stack.

**Why Rust works this way:** Forces explicit error handling. No silent failures, no exceptions.

**Where it appears:**
- `src/config.rs:37` - `read_to_string(...).with_context(...)?`
- `src/git.rs:11` - `Command::new(...).output().with_context(...)?`
- `src/scan.rs:32` - `State::load(...).context(...)?`

**Try breaking this:**
```rust
// In config.rs, remove the ? operator
let content = std::fs::read_to_string(path); // Returns Result, not String
// let config: Config = toml::from_str(&content)?; // ERROR: no `?` on Result
```
**Expected error:** `the `?` operator can only be applied to values that implement `FromResidual`...`

---

## 3. Structs and `serde` Derive

**What it is:** Structs group related data. `#[derive(Serialize, Deserialize)]` generates JSON (de)serialization code automatically.

**Why Rust works this way:** Zero-cost abstractions. Serialization logic is generated at compile time, no runtime reflection.

**Where it appears:**
- `src/facts.rs:8-13` - `Facts` struct with `scanned_at` and `projects`
- `src/facts.rs:17-47` - `Project` struct with all repo metadata
- `src/config.rs:6-16` - `Config` struct for TOML config

**Try breaking this:**
```rust
// In facts.rs, remove the derive
// #[derive(Debug, Clone, Serialize, Deserialize)]
struct Project { ... }
// serde_json::to_string(&project) // ERROR: trait bounds not satisfied
```
**Expected error:** `the trait bound `Project: Serialize` is not satisfied`

---

## 4. `std::process::Command`

**What it is:** Runs external processes. `output()` captures stdout/stderr, `status()` just checks exit code.

**Why Rust works this way:** Safe abstraction over OS process creation. Explicit error handling for failed commands.

**Where it appears:**
- `src/git.rs:7-19` - `run_git()` helper wraps `Command::new("git")`
- `src/git.rs:36-47` - `get_head()` uses `rev-parse`
- `src/git.rs:139-160` - `get_recent_commits()` uses `git log`

**Try breaking this:**
```rust
// In git.rs, don't check status
let output = Command::new("git").args(["invalid"]).output()?;
// let stdout = String::from_utf8_lossy(&output.stdout);
// Use stdout even if command failed // BUG: would use error output as data
```
**Expected issue:** Silent data corruption - but Rust forces you to check `output.status.success()`

---

## 5. Modules

**What it is:** `mod name;` declares a module. Items are private by default, `pub` makes them visible.

**Why Rust works this way:** Explicit visibility boundaries. Encapsulation by default.

**Where it appears:**
- `src/lib.rs:3-12` - Declares all modules: `config`, `init`, `scan`, `facts`, `state`, `git`, `languages`, `dry_run`
- `src/main.rs:3-12` - Same modules declared in binary
- `src/scan.rs:5-9` - Uses `use crate::config::Config;`

**Try breaking this:**
```rust
// In main.rs, try to use private function
// scan::discover_repos(...) // ERROR: function is private
```
**Expected error:** `function 'discover_repos' is private`

---

## 6. `Option`

**What it is:** `Option<T>` represents either `Some(T)` or `None`. Forces handling of absent values.

**Why Rust works this way:** No null pointers. Absence is explicit in the type system.

**Where it appears:**
- `src/git.rs:35-47` - `get_head()` returns `Result<Option<String>>`
- `src/facts.rs:24` - `remote_url: Option<String>`
- `src/scan.rs:121-122` - `head_opt.unwrap_or_else(...)`

**Try breaking this:**
```rust
// In scan.rs, try to use head_opt directly as String
let head = git::get_head(repo_path)?; // Returns Option<String>
// let head_str = head; // Type is Option<String>, not String
// println!("{}", head); // ERROR: Option<String> doesn't implement Display
```
**Expected error:** `Option<String>` cannot be formatted with `{}` or used as `String`

---

## 7. Iterators

**What it is:** `Iterator` trait with `next()` method. Combinators like `map`, `filter`, `collect` chain operations lazily.

**Why Rust works this way:** Zero-cost abstractions. Iterators compile to efficient loops. No intermediate allocations unless you `collect()`.

**Where it appears:**
- `src/languages.rs:158-169` - `tracked_files.iter().map(...).collect()`
- `src/facts.rs:106-112` - `for project in &mut projects { ... }`
- `src/git.rs:176-180` - `lines.peekable()` for lookahead

**Try breaking this:**
```rust
// In languages.rs, try to use iterator after consuming
let files: Vec<String> = tracked_files.iter().map(|s| s.clone()).collect();
// tracked_files.iter() // ERROR: value moved (if into_iter was used)
```
**Expected error:** `value borrowed here after move` (if using `into_iter`)

---

## Running the Code

```bash
# Build
cargo build

# Run tests
cargo test

# Generate example config
cargo run -- init --example

# Scan (after creating config.toml)
cargo run -- scan

# Dry run
cargo run -- scan --dry-run
```

---

## Summary

M1 introduces the foundational Rust patterns used throughout the project:
- **Safety first:** Ownership, `Result`, `Option` prevent entire classes of bugs
- **Explicit is better:** Module visibility, error propagation, borrowing rules
- **Zero-cost abstractions:** Iterators, `serde` derives, `Command` wrapper