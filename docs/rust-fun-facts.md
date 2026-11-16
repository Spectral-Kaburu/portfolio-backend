# Rust Fun Facts

## M1 Facts

## Fact 1: The `?` Operator is Syntactic Sugar for `match`

```rust
// This:
let x = some_result?;

// Expands to something like:
let x = match some_result {
    Ok(val) => val,
    Err(e) => return Err(e.into()),
};
```

The `?` operator automatically converts the error type using `From::from`, which is why `anyhow::Error` works so well - it implements `From` for almost any error type.

## Fact 2: `serde` Derives Generate Code at Compile Time, Not Runtime

When you write `#[derive(Serialize)]`, the Rust compiler runs the `serde_derive` procedural macro during compilation. This generates optimized serialization code specialized for your struct's exact layout. There's no reflection, no runtime type inspection - just pure compiled code that knows exactly which fields to serialize in which order.

## Fact 3: `std::process::Command` Does Not Use a Shell by Default

```rust
Command::new("git").args(["log", "--oneline"]).output()?
```

This runs `git` directly via `execvp()`, not through `/bin/sh -c`. This is faster, more secure (no shell injection), and avoids shell-specific parsing quirks. If you need shell features (pipes, globs, `~` expansion), you must explicitly use `Command::new("sh").arg("-c").arg("git log | head")`.

## Fact 4: `Option<T>` and `Result<T, E>` Are Enums, Not Special Language Features

```rust
enum Option<T> { None, Some(T) }
enum Result<T, E> { Ok(T), Err(E) }
```

They're defined in the standard library like any other enum. The `?` operator, `if let`, and pattern matching work on them because they're just enums - the compiler has special syntax support, but the types themselves are ordinary.

## Fact 5: Iterators Are Zero-Cost Abstractions

```rust
// This iterator chain:
let sum: i32 = (0..100).filter(|x| x % 2 == 0).map(|x| x * 2).sum();

// Compiles to essentially the same assembly as:
let mut sum = 0;
for x in 0..100 {
    if x % 2 == 0 {
        sum += x * 2;
    }
}
```

The compiler inlines and optimizes iterator chains completely. No heap allocation, no virtual dispatch, no runtime overhead.

---

## M2 Facts

## Fact 6: Trait Objects Are Fat Pointers

```rust
let clock: Arc<dyn Clock> = Arc::new(SystemClock);
// `dyn Clock` is a fat pointer: (data_ptr, vtable_ptr)
// Two machine words instead of one
```

The vtable contains function pointers for all trait methods. When you call `clock.now()`, it looks up the function in the vtable and calls it. This is how dynamic dispatch works - one extra indirection, but enables heterogeneous collections and test injection.

## Fact 7: `async_trait` Uses Heap Allocation

```rust
#[async_trait::async_trait]
trait ChatClient {
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse>;
}
```

Expands to something that returns `Pin<Box<dyn Future + Send + '_>>`. Each async call allocates a `Box` on the heap. For high-throughput services, native async traits (Rust 2024+) avoid this. For CLI tools with few calls, the overhead is negligible.

## Fact 8: `Mutex` Doesn't Prevent Deadlocks, Only Data Races

```rust
let lock1 = mutex1.lock().unwrap();
let lock2 = mutex2.lock().unwrap(); // If another thread does this in reverse order -> DEADLOCK
```

`Mutex` guarantees mutual exclusion for one piece of data. It doesn't know about your locking order across multiple mutexes. Always acquire locks in a consistent global order, or use `parking_lot::Mutex` with deadlock detection in debug builds.

## Fact 9: `Regex` in Rust Is Guaranteed Linear Time

Unlike PCRE/Perl regex which can backtrack exponentially, Rust's `regex` crate uses finite automata (Thompson NFA → DFA). Worst-case matching is O(n) in input length. No catastrophic backtracking attacks possible - safe for untrusted input.

## Fact 10: `rename()` Is Atomic on POSIX

```rust
let temp = NamedTempFile::new()?;
temp.write_all(data)?;
temp.persist("output.json")?; // Internally: rename(temp_path, "output.json")
```

If the process crashes after write but before rename, the original file is untouched. If it crashes after rename, the new file is complete. No intermediate state visible to readers. Windows has similar guarantees with `MoveFileEx` + `MOVEFILE_REPLACE_EXISTING`.

## Fact 11: SHA-256 Is Deterministic Across Platforms

```rust
use sha2::{Sha256, Digest};
let mut hasher = Sha256::new();
hasher.update(b"same input");
let hash = hasher.finalize(); // Always same 32 bytes, same order, everywhere
```

Unlike `std::collections::hash_map::DefaultHasher` (randomized per process for DoS protection), cryptographic hashes are stable. Same input → same output forever, across architectures, OSes, Rust versions. Perfect for cache keys.