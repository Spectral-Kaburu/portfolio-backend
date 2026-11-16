# M3: Review

## Goal
Verify the M2 draft, then let the owner decide what gets published. Code checks catch mechanical problems, an AI reviewer running a different model flags weak claims and jargon, and the owner approves, edits or hides each project in a clickable loop. Decisions persist across regeneration. The output is `data/reviewed.json`, the only file M4 reads.

## Out of scope
Frontend shaping and public JSON (M4), publishing (M5), changing scan or enrich prompts, any automatic text edits, the GitHub API.

## Inputs
`data/enriched.draft.json` with `schemas/enriched.schema.json`, `data/facts.json` with `schemas/facts.schema.json`, and `overrides.toml` (the `ai` keys from M2). If field names here differ from the schemas, the schemas win. Reuse M2's redaction, `ChatClient` trait, retry and throttle logic, stable hash, atomic writes and mock server. Do not duplicate them.

## Commands
- `review run [--project <name>] [--no-ai] [--dry-run] [--force] [--allow-same-model]`: for each project in the draft, run the code checks and then, unless `--no-ai`, the AI reviewer. Writes `data/review.report.json` and a readable `data/review.report.md`. `--no-ai` makes no network call. `--dry-run` prints the exact redacted reviewer request bodies, never the key, sends nothing and writes nothing. Results are cached by a hash of text plus prompt version plus review model; `--force` ignores the cache.
- `review` (interactive): needs an up-to-date report (otherwise say to run `review run`). Loops over projects whose effective decision is pending or stale, those with errors first, then by name. For each it shows the overview, claims with verdict marks, flags and any suggested overview. Clickable actions: **Approve**, **Approve with suggestion**, **Edit** (`$EDITOR` if set, else a line prompt; verify the crates), **Hide**, **Skip**, **Quit**. After approving it offers toggles for **Featured** and **Built**. Every action is written to `overrides.toml` immediately, so quitting mid-loop loses nothing.
- `review narrative`: generate the recent-activity narrative from **approved projects only** (see Narrative), check it, review it, and let the owner approve, edit or reject it.
- `review status`: a table of project, effective decision, error and warning counts, and stale marker, plus a narrative row.
- `review export`: write `data/reviewed.json` from approved, non-stale projects.
- `ai models --role review`: M2's picker, saving to `[review] model` instead of `[ai] model`, with the same probe.

Exit codes: 0 success; 1 fatal (config, key, a blocking error at export); 2 the run completed but some reviewer calls failed; 3 (`review status` only) pending or stale items remain.

## Config (`config.example.toml`)
```toml
[review]
model = ""                  # must differ from [ai] model
max_overview_chars = 700
jargon_terms = ["refactor", "middleware", "endpoint", "pipeline", "daemon", "regex", "boilerplate", "scaffold", "async", "mutex", "API", "CLI", "TUI", "ORM", "JSON", "YAML"]
deny_terms = []             # the owner's private words; checked in code only, NEVER sent to any API
```
The reviewer reuses `[ai]` for base URL, key variable, rate limit, retries and timeouts.

**Model rule:** refuse the AI review when `[review] model` equals `[ai] model`, unless `--allow-same-model`. Warn when both share a provider prefix (the text before the first `/`), as a heuristic for the same model family. Reason: the same model shares its writer's blind spots.

## Code checks (always run, no AI)
Matching of terms is case-insensitive and on word boundaries. Checks collect all problems; they do not stop at the first.

| Code | Severity | Condition |
|---|---|---|
| `project_failed` | error | the draft entry has a `failure`, so there is no text to review |
| `evidence_unresolved` | error | a claim's evidence ref does not resolve against `facts.json` (commit absent from the project's recent commits, no README, no such manifest kind, no stats) |
| `prose_lint` | error | M2's lint: hex string of 7+ characters, path-like token, file extension, backtick, URL, email, key-like pattern |
| `too_long` | error | overview longer than `max_overview_chars` |
| `deny_term` | error | any `deny_terms` hit in the overview, claims, related names or narrative |
| `related_dangling` | error | a related name is not a project in the draft |
| `jargon` | warning | any `jargon_terms` hit, listing each |
| `uncovered_content` | warning | a number or capitalized word in the overview appears in none of the claim texts and not in the project name |
| `claims_dropped` | warning | M2 dropped claims for this project |

**Errors block approval until the text is edited. Warnings and AI flags are shown and need one confirmation.** No error is overridable. Edited text is re-checked before it can be saved.

## AI reviewer (one call per project)
**Input**, all redacted with M2's redaction: the project's overview; its claims, each with the **evidence texts** looked up from `facts.json` through the refs (commit subject by hash, README excerpt, manifest kind, stats), truncated like the M2 digest; and the code-check flags already found. Nothing about other projects and nothing else from the facts. No new data leaves the machine beyond what the writer already saw.

The system prompt says: you are an editor checking portfolio copy for a student developer's site; for each claim decide whether its evidence text supports it; list jargon with plain alternatives; list private-looking phrases (personal names, employers, client names, course codes, internal hostnames); optionally suggest a plainer overview using only facts already in the claims; reply with JSON only. Output:
```json
{
  "claims": [{"index": 0, "verdict": "supported | partial | unsupported", "reason": "one short sentence"}],
  "jargon": [{"phrase": "string", "plain": "plain wording"}],
  "private": [{"phrase": "string", "why": "short reason"}],
  "suggested_overview": "string or null",
  "overall": "ok | needs_attention"
}
```
**Validation** follows M2: tolerant extraction, typed parse, one repair retry, then record the failure.
- Exactly one verdict per claim, indices in range, `reason` at most 160 characters.
- A jargon or private phrase must literally appear in the overview or claims, otherwise it is dropped (this stops invented flags).
- `suggested_overview` must pass the prose lint, stay within `max_overview_chars`, and **add no facts**: every digit sequence and every capitalized word in it must appear in the original overview, its claims or the project name. Otherwise discard it and record `suggestion_rejected`.
- `overall` is `needs_attention` if any claim is `unsupported` or any private phrase is flagged, whatever the model says.
- On failure record `ai_review_failed` with a short reason. It never blocks the owner, whose report then shows the code checks only.

The reviewer never changes text. Its output is advisory.

## Decision model
State lives in `overrides.toml`:
```toml
[projects."<repo_id>"]
ai = true
decision = "approved"            # pending | approved | hidden (absent means pending)
approved_hash = "<sha256 hex>"   # of the published text at approval time
overview = "owner-edited text"   # optional, replaces the draft overview
featured = true                  # optional
status = "built"                 # optional; otherwise computed active or dormant
order = 3                        # optional; lower first; ties by name
```
- **Published text** is `overview` if set, otherwise the draft overview.
- **Effective decision:** `approved` only if `approved_hash` equals the SHA-256 of the current published text. Otherwise it is `stale` and treated as pending. `hidden` is never stale.
- Editing sets `overview`, re-runs the code checks on the new text, and blocks on errors.
- `enrich` never touches decision keys. `review` never touches keys it does not own. Comments and unknown keys are preserved.
- Only projects present in the current draft can be exported. A project un-ticked for AI drops out of the draft and out of the export, and its old decision stays in the file harmlessly.

## Narrative
The M2 narrative was written from every opted-in project, so a project hidden here could leave a trace in it. `review narrative` therefore generates a new one:
- **Input:** the published texts (not claims or evidence) of approved projects whose status is `active`, plus their recent-window commit counts from `facts.json`. With no approved active project, say so and skip.
- **Generation and validation:** M2's narrative prompt and validator, unchanged. Claims citing a name outside the approved list are dropped.
- Then the code checks (`prose_lint`, `jargon`, `deny_term`) and the reviewer, which uses the approved overviews as the evidence. The owner approves, edits or rejects.
- Result is saved to `data/narrative.review.json` and the decision to `overrides.toml`:
```toml
[narrative]
decision = "approved"
approved_hash = "<sha256 of published narrative text>"
approved_set_hash = "<sha256 of the sorted approved repo_ids with their approved hashes>"
summary = "owner-edited text"    # optional
```
- The narrative is **stale** when its own text changes or the approved set or any approved text changes. Export omits a stale or unapproved narrative (`recent_activity: null`) and prints a warning.

## Output: `data/reviewed.json` and `schemas/reviewed.schema.json`
```json
{
  "schema_version": 1,
  "exported_at": "RFC 3339 timestamp",
  "projects": [
    {"repo_id": "string", "name": "string", "status": "active | dormant | built",
     "overview": "final text", "featured": false, "order": null,
     "related": ["name of an exported project"]}
  ],
  "recent_activity": {"summary": "final text"}
}
```
`recent_activity` is `null` when not approved. There are **no claims, evidence, flags, paths, remote URLs or commit hashes**. `repo_id` stays only for M4's joins and is removed at emit. Projects are sorted by `order` ascending (unset last), then name. `related` is filtered to exported projects, so hidden and pending names leave no trace. Validate against the schema and write atomically. Re-run the error-level checks on every included text at export and fail with exit 1 if any fail.

## Report: `data/review.report.json` and `.md`
Per project: the checks (code, severity, message), the AI result or `ai_review_failed`, the text hash, the review model, the prompt version and a timestamp. The Markdown is for the owner: a summary table, then flags per project.

## Acceptance criteria
1. `review run --no-ai` makes no network call. Each planted issue (unresolved evidence ref, hash in prose, path in prose, email, jargon term, deny term, overlong overview, dangling related name, uncovered number) yields the expected check code and severity. A clean fixture yields none.
2. `deny_terms` never appear in any request body, dry-run output, report or cache (sentinel test).
3. A reviewer request contains only that project's overview, claims, resolved evidence texts and code flags, redacted, and nothing about other projects. `--dry-run` prints the exact bodies, sends nothing, writes nothing and never shows the key.
4. With the same review and writer model, the AI review refuses unless `--allow-same-model`. A shared provider prefix warns.
5. Each of these replies is handled as specified: code fences, chatter, empty content, index out of range, missing or unknown verdict, invented jargon phrase, a suggestion that adds a number or capitalized word, a suggestion that fails the lint. One repair retry is sent. A second failure records `ai_review_failed`, the run continues and the exit code is 2.
6. A mock reviewer that returns a suggestion leaves the draft and `overrides.toml` untouched until the owner accepts it.
7. The decision logic, tested as pure functions without a terminal: approve stores decision and hash of the published text; approve-with-suggestion stores `overview` and hash; edit stores `overview`, re-checks and blocks on errors; hide stores hidden; skip changes nothing; quitting mid-loop keeps earlier decisions; other keys and comments in `overrides.toml` survive.
8. Errors block approval and cannot be overridden. Warnings and AI flags need one confirmation before approval.
9. Staleness: after regeneration changes a draft overview, a previously approved project without an override is stale and excluded from export. With an unchanged override it stays approved. A changed override makes it stale. Hidden is never stale.
10. Hidden and pending projects are absent from `reviewed.json`. `related` contains only exported projects. A planted hidden-project name appears nowhere in `reviewed.json`, including the narrative.
11. `review narrative`: the request body contains only approved active projects' published texts (a hidden sentinel is absent). Claims citing non-approved names are dropped. The narrative goes stale when the approved set or any approved text changes. Export omits a stale or unapproved narrative with a warning.
12. `review status` prints the table and exits 0 when no project (and no existing narrative) is pending or stale, otherwise 3.
13. `review export` writes `data/reviewed.json` atomically, sorted, valid against the schema, and containing no claims, evidence, flags, paths, remote URLs or commit hashes other than `repo_id`. It exits 1 if any included text now has an error-level check.
14. `status = "built"`, `featured` and `order` overrides are honored.
15. A second `review run` on unchanged input makes zero requests. A changed text, prompt version or review model makes requests again.
16. The API key never appears in stdout, stderr, files, panic output or error messages, including on the reviewer path (sentinel key plus a mock server that echoes the Authorization header).
17. `ai models --role review` lists models from the mock, probes the chosen one, refuses a failed probe and saves to `[review] model`.
18. No test uses the real network. The M2 mock server is reused. The terminal layer is thin, with at most one smoke test.

## Notes (decisions and why)
- **API call from the engine, not an omp agent:** it reuses the M2 client and mock tests, has no Herdr dependency at runtime, and sees only what the writer saw.
- **Suggest only:** an automatic rewrite can add unsupported claims invisibly. The no-new-facts check plus the owner's acceptance keeps a human in control.
- **A different model from the writer:** it decorrelates blind spots.
- **Hash-bound approvals:** regenerated text can never be published unreviewed.
- **Narrative from approved projects only:** hidden projects must leave no trace.
- **Deny terms in code only:** the list is itself sensitive.
- **Errors block, warnings ask:** keeps the gate strict without making the owner fight noise.

## Handoff
M4 reads only `data/reviewed.json` and `schemas/reviewed.schema.json`. Before advancing, the owner runs `review status` and `review export`, then reads `reviewed.json` end to end as if it were already public.

## Lesson 3 (required)
`docs/tutorial/03-review.md` covers whichever of these the code uses: enums as state machines and exhaustive `match`; a pure core with a thin shell (how to test interactive code); reuse through traits (the M2 fake client); hashing for change detection; word-boundary and case-insensitive matching and its Unicode pitfalls; sorting with `Option` ordering; format-preserving TOML edits; collecting many check errors instead of returning at the first with `?`. Each has a "try breaking this" exercise.

## Expected delegation (starting estimate; the plan decides)
Modules: `review_checks`, `review_ai`, `decisions`, `review_ui`, `narrative`, `export`, `status`. That is over the limit, so a split is mandatory. Likely slices (3 workers): (1) `review_checks`, `export` and `schemas/reviewed.schema.json`, the deterministic core; (2) `review_ai`, `narrative` and `ai models --role`, the API-facing side reusing the M2 client and mock; (3) `decisions`, `review_ui` and `status`, the state machine, overrides IO and terminal layer. The lead owns `src/model.rs` (Check, Severity, Decision, ReviewReport and Reviewed types, the `[review]` config struct), `src/main.rs`, `Cargo.toml`, `config.example.toml` and `schemas/`, and commits the stubs first.
