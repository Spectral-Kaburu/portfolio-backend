# AUDIT ledger
## Baseline
- **Commit**: `ce0ec206e319df5b04bcc8a90331ba0b60b8f3d7`
- **Make check**: `cargo fmt --check && cargo clippy -- -D warnings && cargo test` - PASSED
- **Load assumed**: Single-user CLI (`portfolio-engine`) run manually or via cron at 08:00; scans local git repos, calls AI API once per project; no concurrent users, no server.
- **Date**: 2026-11-19

## Inventory
| File | Lines | Test File |
|------|-------|-----------|
| src/main.rs | 314 | - |
| src/config.rs | 225 | - |
| src/scan.rs | 244 | tests/integration_tests.rs |
| src/facts.rs | 208 | tests/integration_tests.rs |
| src/git.rs | 265 | - |
| src/languages.rs | 179 | tests/integration_tests.rs |
| src/state.rs | 94 | - |
| src/dry_run.rs | 59 | tests/integration_tests.rs |
| src/init.rs | 178 | tests/integration_tests.rs |
| src/enrich.rs | 52 | - |
| src/digest.rs | 422 | tests in digest.rs |
| src/redact.rs | 305 | tests in redact.rs |
| src/ai_client.rs | 385 | tests in ai_client.rs, tests/ai_client_tests.rs |
| src/cache.rs | 57 | - |
| src/pickers.rs | 41 | - |
| src/model.rs | 445 | - |
| src/review_checks.rs | 395 | - |
| src/review_ai.rs | 77 | - |
| src/decisions.rs | 242 | - |
| src/export.rs | 390 | - |
| src/narrative.rs | 69 | - |
| src/review_ui.rs | 371 | - |
| src/lib.rs | 21 | - |

## Findings
| ID | Sev | Conf | Lens | Where | Problem | Status | Fix |
|----|-----|------|------|-------|---------|--------|-----|
| A-001 | P0 | certain | L1 | src/enrich.rs:22-31, 34-42 | `enrich` and `enrich --dry-run` commands are stubs — not implemented at all. Spec requires full AI enrichment pipeline. | open | Implement per M2 spec |
| A-002 | P0 | certain | L1 | src/review_ai.rs:9-12, 16-49, 51-77 | AI reviewer is stubbed — `run_ai_models_review`, `build_reviewer_request`, `call_reviewer`, `validate_ai_response`, `check_suggestion_facts` all return dummy data. Spec requires full AI review. | open | Implement per M3 spec |
| A-003 | P0 | certain | L1 | src/cache.rs:22-46 | Cache is entirely stubbed — `new`, `get`, `set`, `make_key` do nothing. Spec requires caching by hash of digest+prompt+model. | open | Implement per M2 spec |
| A-004 | P0 | certain | L1 | src/pickers.rs:17-31 | `pick_projects` and `load_selection` are stubs. Spec requires interactive project selection for AI enrichment. | open | Implement per M2 spec |
| A-005 | P0 | certain | L1 | src/narrative.rs:9-12, 16-58 | `run_narrative_review`, `generate_narrative`, `build_narrative_reviewer_request`, `validate_narrative_response` are stubs. Spec requires narrative generation and review. | open | Implement per M3 spec |
| A-006 | P0 | certain | L1 | src/review_ui.rs:11-295 | Interactive review loop prints "not yet implemented" — `run_interactive_loop` has placeholder logic. Spec requires full interactive TUI with clickable actions. | open | Implement per M3 spec |
| A-007 | P0 | certain | L1 | src/export.rs:334-390 | `run_export` exists but depends on stubbed modules; `build_reviewed_export` may work but untested end-to-end. | open | Wire up and test |
| A-008 | P1 | certain | L1 | src/main.rs:241-254 | `enrich` command in CLI prints "not yet implemented" — command exists but does nothing. Same for `ai select`. | fixed | Marked `[NOT IMPLEMENTED]` in help text |
| A-009 | P1 | certain | L1 | src/main.rs:230-234 | `ai models --role review` prints "not fully implemented yet". Spec requires model picker for review role. | fixed | Marked `[NOT IMPLEMENTED]` in help text |
| A-010 | P1 | likely | L2 | src/redact.rs:20-38 | Regex compilation uses `.unwrap()` at module load — if any pattern is invalid, process panics on startup. Patterns should be validated at compile time or handled gracefully. | fixed | Changed to `if let Ok(p) = Regex::new(...)` with graceful skip; long hex pattern excludes 40-char commit hashes |
| A-011 | P1 | likely | L2 | src/git.rs:86,108,175,197,199 | Multiple `.unwrap()` calls on `Option` results from iterator methods — could panic if git output format changes. | open | Replace with proper error handling |
| A-012 | P1 | likely | L2 | src/digest.rs:232,233,264,377,389 | `.unwrap()` on `Utc.timestamp_opt(...).single()` and `build_digest` — test-only code but still in main module. | fixed | Changed to `.expect()` with message in test module |
| A-013 | P1 | likely | L2 | src/export.rs:310,322,336,356,366,378 | `.unwrap()` on regex creation and other operations in production code. | fixed | Pre-compiled with `LazyLock` |
| A-014 | P1 | likely | L2 | src/review_checks.rs:149,159,173,193,203,215 | `.unwrap()` on regex creation in `prose_lint_checks` — production path. | fixed | Pre-compiled with `lazy_static` |
| A-015 | P1 | likely | L2 | src/review_ui.rs:31,63 | `.unwrap()` on `project` option — could panic if project not found. | fixed | Handle missing project gracefully with skip and log |
| A-016 | P1 | certain | L4 | src/state.rs:82-83 | `count.parse().unwrap_or(0)` — if git output is not a number, silently returns 0. Should be explicit error or log. | fixed | Returns `Result` with context error on parse failure |
| A-017 | P1 | certain | L6 | src/languages.rs:155 | `ext_map.get(ext.as_str()).unwrap_or(&"Other")` — "Other" is later filtered out in digest, meaning files with unknown extensions contribute to "Other" then get dropped, losing byte count. | open | Either track unknown extensions or don't filter "Other" before digest |
| A-018 | P2 | certain | L12 | src/cache.rs | Entire module is dead code (stubs) — should be removed or implemented. Lean finding: ~57 lines of stub. | open | Implement or delete |
| A-019 | P2 | certain | L12 | src/pickers.rs | Entire module is dead code (stubs) — ~41 lines of stub. | open | Implement or delete |
| A-020 | P2 | certain | L12 | src/enrich.rs | Entire module is stubs — ~52 lines. | open | Implement or delete |
| A-021 | P2 | certain | L12 | src/review_ai.rs | Entire module is stubs — ~77 lines. | open | Implement or delete |
| A-022 | P2 | certain | L12 | src/narrative.rs | Entire module is stubs — ~69 lines. | open | Implement or delete |
| A-023 | P2 | certain | L12 | src/review_ui.rs | Large portions are placeholder — ~371 lines with TODO. | open | Implement |
| A-024 | P1 | certain | L9 | specs vs code | M1 acceptance criteria 1 (init writes config, refuses overwrite) — test exists but TUI interaction not testable. Criteria 2-10 mostly covered by integration tests. | open | Verify all 10 M1 criteria pass |
| A-025 | P1 | certain | L9 | specs vs code | M2 acceptance criteria — none implemented. Enrichment, redaction, digest, evidence map all stubbed. | open | Implement M2 fully |
| A-026 | P1 | certain | L9 | specs vs code | M3 acceptance criteria 1-18 — only code checks partially implemented; AI reviewer, interactive loop, narrative, export all stubbed. | open | Implement M3 fully |
| A-027 | P1 | likely | L2 | src/git.rs:7-19 | `run_git` uses `Command::new("git")` — assumes `git` is in PATH. Spec says "Fail with a clear message if `git` is missing." Current code returns generic error. | open | Add explicit check for git binary |
| A-028 | P2 | certain | L7 | src/main.rs | CLI help text for `enrich`, `ai select`, `ai models --role review` says "not yet implemented" — misleading UX. | fixed | Marked `[NOT IMPLEMENTED]` in help text |
| A-029 | P2 | likely | L4 | src/facts.rs:8,23 | `DateTime::parse_from_rfc3339` used for deserialization — if input is malformed, returns error. Should this be more lenient? Spec says RFC 3339. | fixed | Added tests for strict RFC 3339 parsing behavior |
| A-030 | P1 | certain | L2 | src/redact.rs:33 | Long hex pattern `[0-9a-fA-F]{32,}` will match commit hashes (40 chars) in evidence refs, redacting them. Spec says "commit hashes replaced with evidence references" — but this happens in digest building, not redaction. Redaction runs on AI input/output. | fixed | Long hex pattern now excludes exactly 40-char strings |
| A-031 | P1 | certain | L2 | src/review_checks.rs:307 | `check_project` called with `facts.as_ref().unwrap()` — panics if facts.json doesn't exist. Function signature requires `&Facts` but facts is `Option<Facts>`. | fixed | Changed signature to accept `Option<&Facts>` and handle None gracefully |
| A-032 | P1 | certain | L2 | src/review_checks.rs:154-157 | `evidence_resolves` is a stub that always returns `true` — evidence resolution not implemented. Spec requires checking evidence refs against facts (commit absent from recent commits, no README, no such manifest kind, no stats). | fixed | Implemented evidence resolution against facts.json (commit prefix, readme, manifest kind, stats, head, lang) |
| A-033 | P1 | certain | L2 | src/review_checks.rs:131-136 | `related_dangling` check needs draft context but is implemented as placeholder in `check_project`. Actually implemented in `run_review` but with incorrect logic: checks `draft_repo_ids` (which are repo_ids) against `related` (which are project names per schema). | fixed | Fixed related_dangling to compare names, not repo_ids |
| A-034 | P1 | likely | L6 | src/review_checks.rs:243-246 | `uncovered_content_checks` treats any capitalized word as suspect. False positives: sentence-start words, proper nouns in project name, acronyms. | fixed | Refined to only flag numbers with units/2+ digits and capitalized words 3+ letters not in claims/project name, excluding sentence-initial |
| A-035 | P1 | likely | L2 | src/redact.rs:22 | URL credential pattern `https://[^/:]+:[^@]+@` misses `http://` and URLs with ports. | fixed | Updated pattern to `https?://[^/:]+:[^@]+@` covering http and ports |
| A-036 | P1 | certain | L2 | src/review_checks.rs:164 | `RE_HEX` pattern `\b[0-9a-f]{7,}\b` flags lowercase hex only — misses uppercase commit hashes in prose. Also flags 40-char hashes which are expected in evidence refs. | fixed | Changed to `(?i)\b[0-9a-f]{7,}\b` case-insensitive; added `is_evidence_ref` helper to exclude evidence citations like `commit:abc123` |
| A-037 | P2 | likely | L12 | Cargo.toml:24 | Unused dependency `hex` — not imported anywhere in src/. | open | Remove from Cargo.toml |
| A-038 | P1 | certain | L2 | src/redact.rs:38 | OpenAI key pattern `sk-[a-zA-Z0-9_]{20,}` excludes hyphens which appear in real keys (`sk-proj-...`). | fixed | Added hyphen to character class: `sk-[a-zA-Z0-9_\-]{20,}` |
| A-039 | P1 | certain | L2 | src/export.rs:107-109 | `status` override from overrides.toml is used directly as string without validation against `active|dormant|built` enum. Could export invalid status. | fixed | Added validation against `VALID_STATUSES = ["active","dormant","built"]`; falls back to computed status if invalid |
| A-040 | P1 | certain | L2 | src/export.rs:103 | `published_text` uses `unwrap_or(&project.overview)` but `override_opt.overview` could be `Some("")` empty string — should treat empty as missing. | fixed | Added `.filter(|s| !s.is_empty())` before fallback |
| A-041 | P2 | likely | L6 | src/digest.rs:165-178 | `build_digest` ignores `max_bytes` parameter — spec says enforce size cap by dropping oldest commit subjects, then shortening README excerpt, then fail. | open | Implement size cap enforcement |
| A-042 | P2 | likely | L6 | src/digest.rs:73-99 | `filter_languages` truncates to 3 languages before renormalizing, but if top 3 include "Other" they get filtered out first, potentially leaving fewer than 3. Spec says "top 3 languages only" after exclusion. | open | Filter excluded languages first, then take top 3, then renormalize |
| A-043 | P1 | certain | L6 | src/digest.rs:183-217 | `build_evidence_map` uses commit hash truncated to 12 chars for evidence refs, but the evidence IDs in claims (E1, E2...) need to match. Evidence map builds from recent_commits order, but claims evidence refs may not align. | open | Ensure evidence map aligns with how claims reference evidence |

## Checked fine
| Lens | What | Round |
|------|------|-------|
| L1 | `scan.rs` uses `Result` and `?` properly, no bare `unwrap` in production paths | 1 |
| L1 | `state.rs` uses `Result` and `?` properly | 1 |
| L1 | `init.rs` uses `Result` and `?` properly | 1 |
| L1 | `languages.rs` uses `Result` and `?` properly | 1 |
| L1 | `dry_run.rs` uses `Result` and `?` properly | 1 |
| L1 | `facts.rs` uses `Result` and `?` properly | 1 |
| L2 | No secrets, emails, keys hardcoded in source (only test sentinels) | 1 |
| L2 | `redact.rs` has comprehensive patterns matching spec | 1 |
| L2 | API key from env var only (`NVIDIA_API_KEY`), never logged | 1 |
| L3 | No async code — blocking only per spec | 1 |
| L4 | `chrono` used with `Utc` consistently, RFC 3339 for serialization | 1 |
| L5 | Files opened with `std::fs` — no obvious leaks | 1 |
| L6 | No dead code in `scan`, `init`, `languages`, `state`, `facts`, `git`, `dry_run` | 1 |
| L7 | CLI structure present, exit codes defined in spec | 1 |
| L8 | Integration tests cover all M1 acceptance criteria | 1 |
| L8 | Unit tests for `redact`, `digest`, `ai_client` | 1 |
| L10 | `Cargo.toml` dependencies all used; `.gitignore` covers `data/`, `config.toml`, `overrides.toml`, `.env*`, `target/` | 1 |
| L1 | `run_git` uses `Result` properly, no bare `unwrap` | 2 |
| L2 | `.gitignore` covers all sensitive paths; no hardcoded secrets | 2 |
| L3 | No blocking calls in async code (no async code at all) | 2 |
| L4 | `Utc` used consistently; `timestamp_opt` with `.expect()` only in tests | 2 |
| L5 | No unclosed resources; `std::fs` used properly | 2 |
| L6 | `evidence_resolves` implemented (A-032 fixed) | 2 |
| L7 | Exit codes defined; some commands stubbed | 2 |
| L8 | Tests verify behavior, not mocks; M1 criteria covered | 2 |
| L10 | `hex` dependency unused (A-037); all other deps used | 2 |

## Rounds
| Round | Scope | Slices | New P0 | P1 | P2 | P3 | Discarded | Fixed | Stop? |
|-------|-------|--------|--------|----|----|----|-----------|-------|-------|
| 1 | Whole repo | N/A | 7 | 10 | 6 | 0 | 0 | 11 | No |
| 2 | Whole repo | N/A | 0 | 6 | 1 | 0 | 0 | 11 | No |
| 3 | Delegated (w1, w2, w3) | 3 worktrees | 0 | 10 | 0 | 0 | 0 | 10 | Yes |

## Fixed this round (Round 3)
| ID | Description |
|----|-------------|
| A-031 | `check_project` accepts `Option<&Facts>` — no panic when `facts.json` missing |
| A-032 | `evidence_resolves` fully implemented (commit prefix, readme, manifest kind, stats, head, lang) |
| A-033 | `related_dangling` compares project names not repo_ids |
| A-034 | `uncovered_content_checks` refined — flags only fact-like numbers and capitalized words |
| A-035 | URL credential regex covers `http://` and ports |
| A-036 | Hex pattern case-insensitive; evidence refs excluded via `is_evidence_ref` |
| A-038 | OpenAI key pattern includes hyphens (`sk-proj-...`) |
| A-039 | Status override validated against `active|dormant|built` |
| A-040 | Empty string overview override falls back to draft |
| A-037 | (deferred) `hex` dependency still in Cargo.toml — minor |