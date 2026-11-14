# 00 Overview

## Goal
Keep a portfolio current with almost no manual work. The engine runs on the owner's laptop, scans chosen directories, and writes an **overview** of recent work in natural language. The owner reviews it, and only the approved, static output is pushed to GitHub Pages.

The landing page shows what the owner has been up to recently. Older work moves to a "built" or "dormant" section. The public site never shows technical specifics.

## Pipeline
```
scan -> facts -> enrich (AI) -> review (agent, then human) -> emit (per frontend) -> publish
```

## Artifacts and who owns them
| File | Written by | Public? | Notes |
|---|---|---|---|
| `config.toml` | human / `init` | no | Local paths, window sizes, model name. Gitignored. |
| `data/facts.json` | `scan` | no | Deterministic facts only. No AI. |
| `data/state.json` | `scan` | no | Last-seen HEAD per repo, to detect what changed. |
| `data/enriched.draft.json` | `enrich` (M2) | no | AI prose plus evidence references. |
| `data/review.report.md` | review agent (M3) | no | Claims checked against evidence, jargon flags. |
| `overrides.toml` | human | no | Review decisions keyed by `repo_id`. Survives regeneration. |
| `frontends/<name>/` | adapter + human (M4) | no | Frontend plan, JSON Schema contract, mapping. |
| `site data` (final JSON) | `emit` (M4) | **yes** | Validated against the contract. No evidence, no hashes. |

## Project record (conceptual, exact schema lives in M1 spec)
`repo_id`, `name`, `remote_url`, `languages`, `commit_count`, `first_commit_at`, `last_commit_at`, `recent_commits`, `manifests`, `readme_excerpt`. Later milestones add `status`, `summary`, `visible`, `featured`, `evidence`.

## Status rules (code decides, never the model)
- `active`: last commit within `active_days` (config, default 30).
- `built`: the owner marked it finished in `overrides.toml`.
- `dormant`: everything else.

## Evidence model
Every claim the AI writes in the draft carries `evidence`: a list of references to facts (commit short hash, or a manifest/README change). The review agent verifies each claim against its evidence and flags anything unsupported. `emit` strips evidence from public output.

## Privacy model
- Only allowlisted file contents are read (see milestone spec).
- Only a redacted digest of facts is sent to the model: commit subjects, diff stats, README excerpt, manifest names. Never raw source files.
- Redaction removes key-like strings and emails before anything leaves the machine.
- Public output contains an overview in plain language only.

## Frontend contract (concept, built in M4)
The facts layer never changes per frontend. For each frontend idea, an adapter agent reads `frontends/<name>/plan.md` and **proposes** a JSON Schema plus a mapping. The human approves and freezes it. On each run the model fills only prose fields inside the frozen schema, and Rust validates the result. The model never invents the output shape at run time.

## Repos
- `portfolio-engine`: this repo. May become public, so local paths and data stay gitignored.
- Pages repo (e.g. `<user>.github.io`): separate. `publish` writes only reviewed output there.
