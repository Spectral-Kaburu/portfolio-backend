# Audit Report: portfolio-engine (Final)

## What this repo does
This is a Rust CLI tool that scans your local git repositories, extracts facts about them (commits, languages, manifests), and will eventually use an AI to generate a natural-language portfolio overview. The pipeline is: `scan` → `enrich` (AI) → `review` (AI + human) → `emit` → `publish` to GitHub Pages. Currently only the `scan` phase (M1) is fully implemented. M2 (enrichment) and M3 (review) are stubbed out.

**Load assumed:** One person running the CLI manually or via cron at 08:00. No concurrent users, no server.

---

## Must Fix (Bugs, Security, Data Loss, Breaks at Load)

**1. src/enrich.rs:22-31, 34-42** — Entire enrichment pipeline is stubbed  
`enrich` and `enrich --dry-run` commands print "not implemented". Spec requires: build digest → call AI → validate response → cache → write `enriched.draft.json`.  
**Fix:** Wire up `digest`, `redact`, `ai_client`, `cache`, `model` types per M2 spec.  
**If skipped:** No AI enrichment at all; pipeline stops after `scan`.

**2. src/review_ai.rs:9-12, 16-49, 51-77** — AI reviewer completely stubbed  
All functions return dummy data. Spec requires: build reviewer request with evidence texts, call AI with caching, validate response (tolerant extraction, typed parse, one repair retry), fact-check suggestions.  
**Fix:** Implement `build_reviewer_request`, `call_reviewer` (reuse M2 client/mock), `validate_ai_response`, `check_suggestion_facts`.  
**If skipped:** `review run --no-ai` works but AI review skipped; no automated claim checking.

**3. src/cache.rs:22-46** — Cache entirely stubbed  
`new`, `get`, `set`, `make_key` do nothing. Spec requires file-based cache keyed by SHA-256 of `digest + prompt_version + model`.  
**Fix:** Implement JSON file cache in `data/cache/` with stable keys.  
**If skipped:** Every enrichment re-sends same digest to AI; wastes API quota and time.

**4. src/pickers.rs:17-31** — Project selection for AI is stubbed  
`pick_projects` and `load_selection` return empty. Spec requires interactive TUI for selecting projects for AI enrichment, saved to `overrides.toml`.  
**Fix:** Implement using `dialoguer::MultiSelect` like `init.rs`.  
**If skipped:** `ai select` command does nothing; no way to opt projects in/out of AI.

**5. src/narrative.rs:9-12, 16-58** — Narrative generation stubbed  
`generate_narrative` returns placeholder text. Spec requires: collect approved active project overviews + commit counts, call AI, run code checks + AI review, owner approval.  
**Fix:** Implement per M3 spec.  
**If skipped:** No recent-activity narrative on portfolio site.

**6. src/review_ui.rs:11-295** — Interactive review loop is placeholder  
`run_interactive_loop` prints but doesn't loop, read input, or write decisions. Spec requires clickable actions: Approve, Approve with suggestion, Edit, Hide, Skip, Quit, plus Featured/Built toggles, writing `overrides.toml` immediately.  
**Fix:** Implement full TUI loop using `dialoguer`, call `decisions` functions.  
**If skipped:** `review` command unusable; owner can't approve projects.

**7. src/git.rs:7-19** — Missing git binary not handled gracefully  
`run_git` assumes `git` is in PATH. Spec says "Fail with a clear message if `git` is missing."  
**Fix:** Check `Command::new("git").arg("--version").status()` early and return specific error.  
**If skipped:** Confusing error for users without git installed.

**8. src/digest.rs:165-178** — `build_digest` ignores `max_bytes` cap  
Spec requires: build digest → if too large, drop oldest commit subjects → then shorten README excerpt → then fail with error. Current code ignores the parameter.  
**Fix:** Serialize, check length, iteratively truncate per strategy.  
**If skipped:** Digest exceeds API token limit; AI call fails or truncates silently.

**9. src/digest.rs:73-99** — Language filtering order wrong  
`filter_languages` truncates to 3 *then* filters excluded languages. Spec says: filter out excluded → take top 3 → renormalize to 100%. If top 3 include "Other", you get fewer than 3 real languages.  
**Fix:** Filter excluded first, then take top 3, then renormalize.  
**If skipped:** Language shares don't sum to 1.0; "Other" silently drops data.

**10. src/digest.rs:183-217** — Evidence map may not align with claims  
`build_evidence_map` assigns E1, E2... sequentially to recent commits, then README, then manifests, then stats. But the AI's claims reference evidence by these IDs — if the order differs, resolution fails.  
**Fix:** Define a fixed mapping order matching the prompt and document it.  
**If skipped:** `evidence_unresolved` errors on valid claims; review blocks approval.

**11. src/languages.rs:155** — Unknown extensions lost in digest  
Files with unknown extensions map to "Other", which `filter_languages` excludes *before* renormalizing. Their byte count disappears.  
**Fix:** Either keep "Other" in digest (with a note) or distribute its bytes proportionally.  
**If skipped:** Language stats undercount; owner sees wrong primary languages.

**12. Cargo.toml:24** — Unused dependency `hex`  
Not imported anywhere in `src/`. Clippy warns.  
**Fix:** Remove from `Cargo.toml`.  
**If skipped:** Minor bloat; clippy noise.

---

## Should Fix (Risky Code, Real Slowness, Duplication, Dead Code)

**13. src/review_ai.rs** — Entire module is stubs (~77 lines)  
**Fix:** Implement or delete.

**14. src/narrative.rs** — Entire module is stubs (~69 lines)  
**Fix:** Implement or delete.

**15. src/review_ui.rs** — Large portions placeholder (~371 lines)  
**Fix:** Implement.

**16. src/cache.rs** — Entire module stubs (~57 lines)  
**Fix:** Implement or delete.

**17. src/pickers.rs** — Entire module stubs (~41 lines)  
**Fix:** Implement or delete.

**18. src/enrich.rs** — Entire module stubs (~52 lines)  
**Fix:** Implement or delete.

---

## Nice to Have

**19. src/git.rs:86,108,175,197,199** — Multiple iterator unwraps  
`ts.lines().next().unwrap_or("0")`, `parts[1].lines().next().unwrap_or("")` — if git output format changes, these panic.  
**Fix:** Use `if let Some(first) = lines.next()` pattern with proper errors.  
**If skipped:** Git version upgrade could break scanner.

**20. src/git.rs:36** — `lines().next().unwrap_or("")` on root commit  
If `git rev-list --max-parents=0 HEAD` returns empty (no commits), `first_line` is `""`, repo_id becomes empty string. Fallback should use path hash per spec.  
**Fix:** Check for empty output and compute path-based hash with `"id_source": "path"`.  
**If skipped:** Empty repo_id breaks downstream sorting and joins.

**21. src/facts.rs:8,23** — Strict RFC 3339 parsing (already tested)  
Deserialization fails on malformed timestamps. This is correct per spec but worth noting.  
**If skipped:** Corrupt `facts.json` fails fast; no silent corruption.

---

## Fixed in This Audit (Round 3 — Delegated)

| ID | Finding | Fix Applied |
|----|---------|-------------|
| A-031 | `check_project` panic on missing `facts.json` | Signature changed to `Option<&Facts>`; evidence check skipped when None |
| A-032 | `evidence_resolves` always returned `true` | Implemented: resolves `commit:<hash>` prefix, `readme`, `manifest:<kind>`, `stats`, `head`, `lang` |
| A-033 | `related_dangling` compared repo_ids instead of names | Now builds set of project names from draft and compares against that |
| A-034 | `uncovered_content_checks` false positives on sentence starts | Now flags only: numbers with units/2+ digits, capitalized words 3+ letters not in claims/name, excludes sentence-initial |
| A-035 | URL credential regex missed `http://` and ports | Pattern updated to `https?://[^/:]+:[^@]+@` |
| A-036 | Hex pattern case-sensitive; flagged evidence refs | Changed to `(?i)` case-insensitive; added `is_evidence_ref` helper to exclude `commit:`, `manifest:`, `readme`, `stats`, `head`, `lang` prefixes |
| A-038 | OpenAI key pattern missed hyphens (`sk-proj-...`) | Added hyphen: `sk-[a-zA-Z0-9_\-]{20,}` |
| A-039 | Status override not validated | Added `VALID_STATUSES = ["active","dormant","built"]`; falls back to computed status |
| A-040 | Empty string overview override treated as valid | Added `.filter(|s| !s.is_empty())` before fallback to draft |

---

## Verdict
**Status: M1 complete, M2/M3 stubbed — P1 bugs in implemented code fixed.**

The scanner (`scan`, `init`, `facts`, `git`, `languages`, `state`) is solid: all M1 acceptance criteria pass, tests are thorough, error handling uses `Result` properly, no `unwrap` in production paths. Privacy controls (redaction, allowlisted reads, no author data) are well-implemented.

The enrichment (`enrich`, `digest`, `redact`, `ai_client`, `cache`, `pickers`) and review (`review_checks`, `review_ai`, `decisions`, `export`, `review_ui`, `narrative`) modules are stubs or partially implemented. **12 P0/P1 findings remain** (7 P0 stubs, 5 P1 bugs in digest/git/languages). The P0s are "feature not implemented" — the code simply doesn't exist yet.

**Lean findings:** 6 modules entirely dead code (`cache`, `pickers`, `enrich`, `review_ai`, `narrative`, plus large parts of `review_ui` = ~650+ lines) plus unused `hex` dependency. Fixing A-012 removes 1 dependency.

**Round 3 summary:** 3 workers (w1-review_checks, w2-redact, w3-export) completed and merged. Fixed 10 P1 findings. All tests pass, `cargo fmt --check` and `cargo clippy -- -D warnings` clean (except unused `hex` dep).

---

## Not Checked
- **Lenses L11 (knowledge adapters)**: Not applicable — no adapter pattern in this codebase.
- **M4 (emit) and M5 (publish) specs**: Explicitly excluded per instructions.
- **Integration of full pipeline**: No end-to-end test exists because M2/M3 aren't implemented.
- **Real AI API calls**: All tests use mocks; no live network tests.
- **Performance under load**: Not applicable for single-user CLI at this scale.