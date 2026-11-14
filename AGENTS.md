# Portfolio Engine: agent rules

A Rust CLI (Command Line Interface) that scans local git repos, turns recent work into a plain-language portfolio overview through an OpenAI-style API (Application Programming Interface), and emits reviewed JSON (JavaScript Object Notation) for a static site on GitHub Pages.

The owner is **learning Rust**. Teaching is a deliverable, not a bonus.

## Context discipline (read this first)
- Read only: this file, then `docs/spec/00-overview.md`, then the spec for the **current milestone**. Do not open other specs. Open `docs/spec/delegation.md` only when a delegation limit below is crossed.
- Current milestone: **M1** (`docs/spec/m1-scan.md`). The human updates this line.
- Read files by line range or with grep. Never paste whole files into chat. After each task, summarize in 10 lines or fewer and drop details you no longer need.
- If your harness has subagents, use them for exploration and large reads so the main context stays small.

## Workflow
1. Before writing code for a milestone, post a plan: files to create, module layout, what goes in each. Wait for the human's go-ahead.
2. If the spec is ambiguous, ask one targeted question. Label any assumption as an assumption. Never guess silently.
3. Before adding a dependency, check its current version and API (`cargo search`, crates.io, docs.rs). Do not rely on memory. Justify each new crate in one line in `docs/concepts.md`.
4. Done means: every acceptance criterion in the milestone spec passes, `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` are green, and the milestone's tutorial lesson exists.
5. Debugging: read the actual error before proposing a fix. If a fix fails twice, stop and report the exact error, the most likely cause (labelled as a hypothesis) and 2-3 next steps. Then wait.

## Delegation (mandatory past these limits)
Check at plan time, and again whenever the plan grows mid-work. You must split the work across several agents, each in its own terminal (its own Herdr workspace and git worktree), if **any** of these is true:
- more than **4 independent tasks** (a task is independent if it can start without another task's output)
- more than **2 modules** touched (a module is one `src/<name>.rs` file or `src/<name>/` directory)
- more than about **300 new lines** of non-test code expected

Use the **herdr skill** for this (Herdr is a terminal multiplexer for coding agents; this line is the explicit mention the skill needs to activate). Then read `docs/spec/delegation.md` and put the split in the plan you submit for approval. Only the lead agent delegates; workers never spawn workers. If Herdr is not running (`HERDR_ENV` is not set), stop and tell the human. Do not improvise parallelism and do not push on alone.

## Hard rules: privacy and safety
- Read file contents only from the allowlist in the milestone spec (README, manifests). Never read, print or log `.env*`, keys, tokens, or anything outside the allowlist.
- API keys come from environment variables only (`NVIDIA_API_KEY`). Never write one to a file, test, log or commit.
- Never record commit author names or emails.
- `.gitignore` must cover `data/`, `config.toml`, `overrides.toml`, `.env*`, `.briefs/` and `target/`. Commit `config.example.toml` instead.
- Nothing is sent to any external API except the redacted digest defined in the M2 spec.
- New projects default to `visible = false`. Nothing is published without the human's explicit approval.
- Public output is an **overview**: no commit hashes, file paths, function names, repo-internal jargon or evidence. Evidence lives in draft files for review only.
- The AI writes prose. Code decides facts (status, dates, counts, languages). The model never sets them.

## Rust rules
- Use the edition `cargo new` generates. No `unsafe`. No `unwrap()` or `expect()` outside tests unless a comment explains why it cannot fail.
- Errors: `Result` with `anyhow` and `.context(...)` messages a human can act on.
- Blocking code only (no `tokio`, no async) until a lesson says otherwise. Use a blocking HTTP client.
- Small modules with one job each: `config`, `scan`, `enrich`, `review`, `emit`, `publish`. Prefer pure functions so they are easy to test.
- Tests build throwaway git repos in temp dirs. Never test against the owner's real directories.

## Teaching rules (hard requirement)
For every milestone:
- Write `docs/tutorial/NN-<topic>.md` covering **only the Rust concepts introduced in that milestone**. For each concept: what it is, *why* Rust works this way, where it appears (`file:line`), and a "try breaking this" exercise naming the compiler error to expect.
- Add `// RUST NOTE:` comments (1-3 plain sentences) wherever the language does something non-obvious. Define jargon on first use. Expand acronyms on first use.
- Append to `docs/concepts.md` (concept to file) and `docs/rust-fun-facts.md` (at least 2 new facts per milestone).
- Explain *why*, not only *what*. If something cannot fit in a comment without jargon, it belongs in the tutorial.

## Commands
(Fill in after `cargo init`: build, test, run, lint.)
