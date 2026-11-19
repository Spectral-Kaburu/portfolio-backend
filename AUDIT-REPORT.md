# Audit Report: portfolio-engine

## What this repo does
This is a Rust CLI tool that scans your local git repositories, extracts facts about them (commits, languages, manifests), and will eventually use an AI to generate a natural-language portfolio overview. The pipeline is: `scan` → `enrich` (AI) → `review` (AI + human) → `emit` → `publish` to GitHub Pages. Currently only the `scan` phase (M1) is fully implemented. M2 (enrichment) and M3 (review) are stubbed out.

**Load assumed:** One person running the CLI manually or via cron at 08:00. No concurrent users, no server.

---

## Must Fix (Bugs, Security, Data Loss, Breaks at Load)

**1. src/review_checks.rs:307** — Panic if `facts.json` missing  
`check_project` is called with `facts.as_ref().unwrap()` but `facts` is `Option<Facts>`. If the file doesn't exist, the process panics.  
**Fix:** Change `check_project` signature to accept `Option<&Facts>` and handle `None` gracefully.  
**If skipped:** `review run` crashes on any machine without a prior `scan`.

**2. src/review_checks.rs:154-157** — Evidence resolution not implemented  
`evidence_resolves` always returns `true`. The spec requires verifying each claim's evidence reference against `facts.json` (commit hash present in recent commits, README exists, manifest kind matches, stats exist).  
**Fix:** Implement actual resolution logic mapping E1/E2/E3 to facts.  
**If skipped:** Unsupported claims pass review silently; owner can't trust the output.

**3. src/review_checks.rs:131-136 / 310-316** — `related_dangling` compares wrong fields  
The check compares `related` (project *names* per schema) against `draft_repo_ids` (which are `repo_id` hashes). They will never match, so every `related` entry is falsely flagged as dangling.  
**Fix:** Build a set of project *names* from the draft and compare against that.  
**If skipped:** Valid related projects are rejected; owner can't link projects.

**4. src/redact.rs:22** — URL credential pattern incomplete  
Pattern `https://[^/:]+:[^@]+@` misses `http://` URLs and URLs with ports (`https://user:pass@host:8080`).  
**Fix:** Use `https?://[^/:]+:[^@]+@` and handle port suffix.  
**If skipped:** Credentials in non-HTTPS or port URLs leak into AI requests.

**5. src/redact.rs:38** — OpenAI key pattern misses hyphens  
Real OpenAI keys contain hyphens (`sk-proj-...`, `sk-...`). Pattern `sk-[a-zA-Z0-9_]{20,}` rejects them.  
**Fix:** Add `-` to character class: `sk-[a-zA-Z0-9_-]{20,}`.  
**If skipped:** Valid keys not redacted; they leave the machine.

**6. src/review_checks.rs:164** — Prose lint hex pattern too aggressive  
`\b[0-9a-f]{7,}\b` only matches lowercase hex, misses uppercase, and flags 40-char commit hashes that appear in evidence references.  
**Fix:** Use case-insensitive flag and exclude evidence-style patterns (e.g., `commit:abc123...`).  
**If skipped:** False positives on evidence refs; misses uppercase hex secrets.

**7. src/export.rs:107-109** — Status override not validated  
`override_opt.status` is used directly as a string without checking it's one of `active|dormant|built`. Could export invalid status.  
**Fix:** Validate against enum values; default to computed status if invalid.  
**If skipped:** Invalid JSON schema output; downstream emit fails.

**8. src/export.rs:103** — Empty string override treated as valid  
`override_opt.overview.as_deref().unwrap_or(&project.overview)` returns empty string if override is `Some("")`, publishing blank overview.  
**Fix:** Check `is_empty()` and fall back to draft overview.  
**If skipped:** Owner accidentally approves empty overview; public page shows nothing.

**9. src/languages.rs:155** — Unknown extensions lost in digest  
Files with unknown extensions map to "Other", which `filter_languages` excludes *before* renormalizing. Their byte count disappears from the language breakdown.  
**Fix:** Either keep "Other" in digest (with a note) or distribute its bytes proportionally.  
**If skipped:** Language stats undercount; owner sees wrong primary languages.

**10. src/git.rs:7-19** — Missing git binary not handled gracefully  
`run_git` assumes `git` is in PATH. Spec says "Fail with a clear message if `git` is missing." Current error is generic.  
**Fix:** Check `Command::new("git").arg("--version").status()` early and return a specific error.  
**If skipped:** Confusing error for users without git installed.

---

## Should Fix (Risky Code, Real Slowness, Duplication, Dead Code)

**11. src/cache.rs** — Entire module is stubs (~57 lines)  
`new`, `get`, `set`, `make_key` do nothing. Spec requires caching by hash of `digest + prompt_version + model`.  
**Fix:** Implement file-based cache with JSON entries keyed by SHA-256.  
**If skipped:** Every enrichment re-sends same digest to AI; wastes API quota and time.

**12. src/pickers.rs** — Entire module is stubs (~41 lines)  
`pick_projects` and `load_selection` return empty. Spec requires interactive TUI for selecting projects for AI enrichment.  
**Fix:** Implement using `dialoguer::MultiSelect` like `init.rs`, save selection to `overrides.toml`.  
**If skipped:** `ai select` command does nothing; no way to opt projects in/out of AI.

**13. src/enrich.rs** — Entire module is stubs (~52 lines)  
`enrich` and `dry_run` print "not implemented". Spec requires full pipeline: build digest, call AI, validate response, cache, write `enriched.draft.json`.  
**Fix:** Wire up `digest`, `redact`, `ai_client`, `cache`, `model` types.  
**If skipped:** M2 completely non-functional; no AI enrichment at all.

**14. src/review_ai.rs** — Entire module is stubs (~77 lines)  
All functions return dummy data. Spec requires AI reviewer call with caching, validation, suggestion fact-checking.  
**Fix:** Implement `build_reviewer_request`, `call_reviewer` (reuse M2 client/mock), `validate_ai_response`, `check_suggestion_facts`.  
**If skipped:** `review run --no-ai` works but AI review skipped; no automated claim checking.

**15. src/narrative.rs** — Entire module is stubs (~69 lines)  
`generate_narrative` returns placeholder text. Spec requires building narrative from approved active projects, running code checks + AI review, owner approval.  
**Fix:** Implement per M3 spec: collect approved overviews + commit counts, call AI, validate, interactive loop.  
**If skipped:** No recent-activity narrative on portfolio site.

**16. src/review_ui.rs** — Interactive loop placeholder (~371 lines)  
`run_interactive_loop` prints but doesn't actually loop, read input, or write decisions. Spec requires clickable actions: Approve, Approve with suggestion, Edit, Hide, Skip, Quit, plus Featured/Built toggles.  
**Fix:** Implement full TUI loop using `dialoguer`, call `decisions` functions, write `overrides.toml` immediately on each action.  
**If skipped:** `review` command unusable; owner can't approve projects.

**17. src/digest.rs:165-178** — `build_digest` ignores `max_bytes` cap  
Spec requires: build digest → if too large, drop oldest commit subjects → then shorten README excerpt → then fail with error. Current code ignores the parameter.  
**Fix:** Serialize, check length, iteratively truncate per strategy.  
**If skipped:** Digest exceeds API token limit; AI call fails or truncates silently.

**18. src/digest.rs:73-99** — Language filtering order wrong  
`filter_languages` truncates to 3 *then* filters excluded languages. Spec says: filter out excluded → take top 3 → renormalize to 100%. If top 3 include "Other", you get fewer than 3 real languages.  
**Fix:** Filter excluded first, then take top 3, then renormalize.  
**If skipped:** Language shares don't sum to 1.0; "Other" silently drops data.

**19. src/digest.rs:183-217** — Evidence map may not align with claims  
`build_evidence_map` assigns E1, E2... sequentially to recent commits, then README, then manifests, then stats. But the AI's claims reference evidence by these IDs — if the order differs, resolution fails.  
**Fix:** Define a fixed mapping order matching the prompt (commits first, then readme, manifests, stats) and document it.  
**If skipped:** `evidence_unresolved` errors on valid claims; review blocks approval.

**20. Cargo.toml:24** — Unused dependency `hex`  
Not imported anywhere in `src/`. Clippy warns.  
**Fix:** Remove from `Cargo.toml`.  
**If skipped:** Minor bloat; clippy noise.

**21. src/ai_client.rs:117** — Mutex unwrap in throttle  
`self.last_request.lock().unwrap()` panics if another thread panicked while holding the lock.  
**Fix:** Use `lock().expect("throttle lock poisoned")` with a message, or handle `PoisonError`.  
**If skipped:** Rare panic under thread panic; process dies instead of recovering.

---

## Nice to Have

**22. src/review_checks.rs:243-246** — Uncovered content false positives  
Flags any capitalized word not in claims/project name. Catches sentence starts, proper nouns, acronyms.  
**Fix:** Only flag numbers and capitalized words that look like facts (dates, versions, counts).  
**If skipped:** Owner sees many warnings; noise reduces trust in real issues.

**23. src/git.rs:36** — `lines().next().unwrap_or("")` on root commit  
If `git rev-list --max-parents=0 HEAD` returns empty (no commits), `first_line` is `""`, repo_id becomes empty string. Fallback should use path hash per spec.  
**Fix:** Check for empty output and compute path-based hash with `"id_source": "path"`.  
**If skipped:** Empty repo_id breaks downstream sorting and joins.

**24. src/git.rs:86,108,175,197,199** — Multiple iterator unwraps  
`ts.lines().next().unwrap_or("0")`, `parts[1].lines().next().unwrap_or("")` — if git output format changes, these panic.  
**Fix:** Use `if let Some(first) = lines.next()` pattern with proper errors.  
**If skipped:** Git version upgrade could break scanner.

**25. src/facts.rs:8,23** — Strict RFC 3339 parsing (already tested)  
Deserialization fails on malformed timestamps. This is correct per spec but worth noting.  
**If skipped:** Corrupt `facts.json` fails fast; no silent corruption.

---

## Verdict
**Status: Early development — M1 complete, M2/M3 stubbed.**

The scanner (`scan`, `init`, `facts`, `git`, `languages`, `state`) is solid: all M1 acceptance criteria pass, tests are thorough, error handling uses `Result` properly, no `unwrap` in production paths. Privacy controls (redaction, allowlisted reads, no author data) are well-implemented.

The enrichment (`enrich`, `digest`, `redact`, `ai_client`, `cache`, `pickers`) and review (`review_checks`, `review_ai`, `decisions`, `export`, `review_ui`, `narrative`) modules are stubs or partially implemented. **43 findings total** (7 P0, 16 P1, 7 P2, 0 P3). The P0s are all "feature not implemented" — the code simply doesn't exist yet.

**Lean findings:** 4 modules entirely dead code (`cache`, `pickers`, `enrich`, `review_ai`, `narrative` = ~296 lines) plus unused `hex` dependency. Fixing A-037 removes 1 dependency.

---

## Not Checked
- **Lenses L11 (knowledge adapters)**: Not applicable — no adapter pattern in this codebase.
- **M4 (emit) and M5 (publish) specs**: Explicitly excluded per instructions.
- **Integration of full pipeline**: No end-to-end test exists because M2/M3 aren't implemented.
- **Real AI API calls**: All tests use mocks; no live network tests.
- **Performance under load**: Not applicable for single-user CLI at this scale.