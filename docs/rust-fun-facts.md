# Rust Fun Facts (M1)

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