# Delegation protocol

Load this file only when a limit in `AGENTS.md` ("Delegation") is crossed. Its purpose is to keep every agent's context small and keep parallel work from colliding.

## 1. Trigger
The limits live in `AGENTS.md` so they can be checked at plan time without opening this file. They are starting values, and the human may tune them there. Re-check them whenever the plan grows. If you cross a limit mid-work, stop, re-plan, and submit the split for approval.

## 2. Roles
- **Lead:** owns the plan, the shared files, the merge and the final report to the human. Does not implement tasks that were handed to workers.
- **Workers:** at most **3 running at once**. Each does one slice and nothing else. A worker never spawns another agent.

## 3. Where workers live and how they are spawned
Use the **herdr skill** for every Herdr operation. Take exact command syntax from the skill and `--help`, never from memory, because the CLI (Command Line Interface) changes. If a creation command returns an error or JSON (JavaScript Object Notation) that lacks what you need, stop and report. Do not retry blindly.

**Layout:** every worker gets its own Herdr **workspace**, created together with its own git worktree and branch. Never run two workers in one checkout.

**Harness:** workers run the same harness the lead is running. Define it once as `WORKER_CMD` (default `omp`) and use that variable everywhere below.

**Spawn procedure (per worker):**
1. **Commit first.** A worktree contains only committed files. Commit the interface stubs and `AGENTS.md` on the base branch before spawning. Check with `--help` which commit a new worktree branch starts from, and confirm it includes them. `AGENTS.md` is discovered automatically by the harness inside the worktree, so briefs do not restate its rules.
2. **Write the brief** to `.briefs/<worker>.md` in the main repo (gitignored). Use absolute paths.
3. **Create the worktree workspace:** `herdr worktree create --cwd <repo> --branch worker/<name> --no-focus`. Read the workspace and pane ids from its JSON output. Never guess ids.
4. **Launch with the brief already loaded:** `herdr pane run <pane> '<WORKER_CMD> @<abs>/.briefs/<worker>.md "Execute the attached brief."'`. The brief is delivered at launch, not typed in after the interface is up, which avoids readiness races.
5. **Wait, don't poll.** Use Herdr's agent wait until the worker is blocked or done.
6. **Read the result from the return file** (section 5), not from the screen. Use `pane read` only when a worker is blocked, to see what it is asking.
7. **Follow-ups:** send a message to a running worker (for example the exact error from a failed check) with Herdr's agent prompt. Do not use it to deliver the initial brief.

## 4. How to split
1. **Split by file ownership, not by step.** Every file has exactly one owner.
2. **Shared files are lead-only:** `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `docs/concepts.md`, `docs/rust-fun-facts.md`, `config.example.toml`, schemas used by several modules. Workers who need a change ask the lead.
3. **Interfaces first.** Before spawning, the lead writes the shared types and function signatures (stubs that compile) and commits them. Workers code against that contract.
4. Prefer a split along module lines (`scan`, `enrich`, `review`, `emit`, `publish`) or along tests versus implementation, whichever gives the fewest shared edges.
5. The split goes in the plan: which task, which worker, which files each worker owns, and the merge order.

## 5. Brief file and return file
Brief (`.briefs/<worker>.md`), keep it short:
```
Goal: <one sentence>
Branch/worktree: <name>
You own only: <files>. Do not edit any other file; ask via the return file instead.
Spec: <path and section to read; nothing else>
Contract: <signatures or types you must conform to>
Acceptance: <the exact criteria for your slice>
Rules: AGENTS.md in your worktree applies in full. Never put API keys anywhere.
If blocked or the spec is ambiguous: stop and say so in the return file. Do not guess.
Return: as your last act, write <abs>/.briefs/<worker>.return.md, then stop.
  Format, 10 lines or fewer: status (done or blocked), what you did,
  files changed, check results, open questions.
```
The return file is the one allowed write outside a worker's owned files.

## 6. Lead context
- The lead reads return files, not transcripts or full diffs, unless checks fail.
- The lead keeps a running status of 5 lines or fewer per worker.

## 7. Merge protocol
1. Merge one branch at a time into the base branch.
2. After each merge run `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`.
3. If a check fails, identify the owning worker and send it the exact error. Apply the debugging rule: read the real error, and after two failed fixes stop and report to the human with the error, a labelled hypothesis, and 2-3 next steps.
4. The lead resolves merge conflicts in shared files.
5. When all branches are merged and green, remove the worktrees and delete the `.briefs/` files.

## 8. Teaching work comes last
Workers write `// RUST NOTE:` comments in their own files. The tutorial lesson, `docs/concepts.md` entries and fun facts are written **after the final merge** by one agent reading the merged code, so every `file:line` reference is correct. The milestone is not done without them.

## 9. Safety
- Workers inherit every hard rule in `AGENTS.md` through the worktree copy.
- Briefs and return files never contain secrets. Workers read keys from env vars only.
- Workers never touch real directories, `overrides.toml` or `config.toml`. They use fixtures.
- Nothing is published, sent to an API beyond the milestone spec, or pushed without the human's approval.

## 10. Report
The lead ends with a report of 15 lines or fewer: what was built per worker, merge result, check results, anything unresolved, and what the human must decide.

## 11. Deferred
A guard hook that mechanically blocks edits outside a worker's owned files and reads of `.env*` is **not built yet**. Revisit after the first delegated run shows whether the rules are being broken. Verify what the harness's hook mechanism can intercept before designing it.
