# M1: Scan - Implementation Plan

## Project Setup
- `cargo init` in `/home/spectre/Projects/portfolio` (creates Cargo.toml, src/main.rs)
- Add dependencies to Cargo.toml
- Create module structure: `src/config.rs`, `src/scan.rs`, `src/init.rs`, `src/facts.rs`, `src/state.rs`, `src/dry_run.rs`, `src/errors.rs`

## Dependencies (Cargo.toml)
```toml
[dependencies]
anyhow = "1.0"           # error handling with context
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
dialoguer = "0.11"       # TUI multi-select
dirs = "5.0"             # home directory
chrono = { version = "0.4", features = ["serde"] }  # RFC 3339 timestamps
walkdir = "2.5"          # directory traversal
tabled = "0.16"          # dry-run table output
jsonschema = "0.20"      # schema validation
```

## Module Layout
```
src/
├── main.rs          # CLI entry, command dispatch
├── config.rs        # Config struct, load/save/validate, config.example.toml
├── init.rs          # init command: find candidates, TUI, write config.toml
├── scan.rs          # scan command: repo discovery, git operations, fact extraction
├── facts.rs         # facts.json types, serialization, schema validation
├── state.rs         # state.json types, load/save (last-seen HEAD per repo)
├── dry_run.rs       # --dry-run table output
├── languages.rs     # extension -> language mapping, size aggregation
├── git.rs           # git command wrappers (std::process::Command)
└── errors.rs        # custom error types
```

## Config Module (`config.rs`)
- `Config` struct with `roots`, `max_depth`, `active_days`, `exclude_dirs`
- `load_config(path)` -> Result<Config> with clear error messages
- `save_config(path, &Config)` -> Result<()> with overwrite protection
- `config.example.toml` with fake paths (committed)
- Real `config.toml` in .gitignore

## Init Command (`init.rs`)
1. Bounded search under home dir for directories containing `.git`
2. Multi-select TUI with `dialoguer::MultiSelect`
3. "Enter path manually" option
4. Write `config.toml` - refuse overwrite without confirmation

## Scan Command Core (`scan.rs`)
### Repo Discovery
- Walk each root up to `max_depth`, skip `exclude_dirs`, no symlinks
- Find dirs containing `.git` (file or dir)

### Git Operations (via `std::process::Command`)
- `git rev-list --max-parents=0 HEAD` -> root commit (repo_id)
- `git rev-parse HEAD` -> current HEAD
- `git rev-list --count HEAD` -> commit_count
- `git log --format=%ct --reverse` -> first_commit_at
- `git log --format=%ct -1` -> last_commit_at
- `git log --since=<date> --numstat --format=%H%x00%s%x00` -> recent commits
- `git ls-files` -> tracked files for languages
- `git remote get-url origin` -> remote_url

### Language Detection
- `git ls-files` -> list tracked files
- For each file: get size from filesystem metadata
- Group by extension, map to language name (small static table)
- Output: `Vec<Language { name, bytes, share }>` sorted by bytes desc

### Manifests (allowlisted, size cap 8KB)
- Check for: `README*`, `Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`, `go.mod`
- Record file name + kind only; README excerpt capped

### Recent Commits
- Since `active_days` ago: hash, timestamp, subject (first line), files_changed, insertions, deletions

### State & New Commits
- Load `state.json` (last_seen_head per repo_id)
- Compare current HEAD -> `new_commits_since_last_scan`
- Save updated `state.json`

## Output Files
### facts.json
```json
{
  "schema_version": 1,
  "scanned_at": "RFC3339",
  "projects": [...]
}
```
- Sorted by `repo_id`
- Deterministic: sorted keys, sorted lists

### state.json
```json
{
  "repos": { "repo_id": "last_seen_head" }
}
```

### Schema Validation
- `schemas/facts.schema.json` (JSON Schema Draft 2020-12)
- Validate in test, optionally at runtime

## Dry-run (`--dry-run`)
- Table: name, last commit, languages, new commits
- Write nothing

## Tests (in `tests/`)
- Temp dir fixtures with `git init`
- All 10 acceptance criteria:
  1. init writes valid config, refuses silent overwrite
  2. scan produces valid facts.json
  3. .env sentinel never appears in output
  4. .gitignore files don't influence languages/excerpts
  5. Two scans = identical except scanned_at
  6. New commit detected correctly
  7. Edge cases: no commits, no remote, detached HEAD, nested repos, unreadable dir
  8. Missing/invalid config -> non-zero exit with actionable message
  9. --dry-run writes nothing
  10. Tests use temp dirs only

## Documentation
- `docs/tutorial/01-scan.md`: ownership/borrowing, Result/?, structs/serde, std::process::Command, modules, Option, iterators
- "Try breaking this" exercises with expected compiler errors
- `docs/concepts.md`: concept -> file mapping
- `docs/rust-fun-facts.md`: +2 facts per milestone

## .gitignore
```
data/
config.toml
overrides.toml
.env*
.briefs/
target/
```
Commit `config.example.toml`.