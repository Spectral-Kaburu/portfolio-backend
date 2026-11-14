# M1: Scan

## Goal
Two commands: `init` (interactive directory picker that writes `config.toml`) and `scan` (finds git repos in the chosen directories and writes `data/facts.json`). No AI, no network.

## Out of scope
AI calls, GitHub API, review, emit, publish, any frontend.

## Commands
- `portfolio-engine init`: Text User Interface (TUI) flow. Find candidate directories (bounded search under the home directory for folders that contain git repos), show them as a **multi-select list** the owner can tick, plus an option to type a path manually. Write `config.toml`. Never overwrite an existing config without asking.
- `portfolio-engine scan`: read `config.toml`, scan, write `data/facts.json` and `data/state.json`.
- `portfolio-engine scan --dry-run`: print a human-readable table (name, last commit, languages, new commits) and write nothing.

## config.toml
```toml
roots = ["/home/user/projects"]   # chosen in init
max_depth = 4                     # how deep to look for repos
active_days = 30                  # recent window
exclude_dirs = ["node_modules", "target", ".venv", "vendor"]
```
Provide `config.example.toml` with fake paths. Real `config.toml` is gitignored.

## Scanner behavior
- Find git repos under each root (a directory containing `.git`) up to `max_depth`. Do not follow symlinks. Skip `exclude_dirs`.
- Use the `git` command-line tool via `std::process::Command` rather than a git library. This is deliberate: simpler, and it teaches process handling and `Result`. Fail with a clear message if `git` is missing.
- Starting points to verify with `git help` (do not trust this list blindly): `git rev-list --max-parents=0 HEAD` (root commit), `git rev-parse HEAD`, `git rev-list --count HEAD`, `git remote get-url origin`, `git ls-files`, `git log --since=<date> --numstat --format=<custom>`.
- Languages: from tracked files (`git ls-files`), sum file sizes from filesystem metadata grouped by extension, map extensions to language names with a small table. Do not read file contents for this.
- Allowlisted content reads, with a size cap per file (8 KB): `README*`, `Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`, `go.mod`. For manifests record only the file name and kind. For README keep a capped excerpt.
- Never read anything else. Never read `.env*`, key files or untracked files.
- Never record author names or emails.

## facts.json record (one per repo, sorted by `repo_id`)
```json
{
  "repo_id": "root commit hash, first 12 chars",
  "name": "directory name",
  "remote_url": "string or null",
  "head": "short hash",
  "commit_count": 0,
  "first_commit_at": "RFC 3339 (internet date/time standard) timestamp",
  "last_commit_at": "RFC 3339 timestamp",
  "languages": [{"name": "Rust", "bytes": 0, "share": 0.0}],
  "manifests": [{"file": "Cargo.toml", "kind": "rust"}],
  "readme_excerpt": "string or null",
  "recent_commits": [
    {"hash": "short", "at": "timestamp", "subject": "first line only",
     "files_changed": 0, "insertions": 0, "deletions": 0}
  ],
  "new_commits_since_last_scan": 0
}
```
Top level: `{"schema_version": 1, "scanned_at": "...", "projects": [...]}`.
`repo_id` is the root commit hash because it survives renames and moves and leaks no path. Fallback for repos with no commits: a stable hash of the directory name, flagged `"id_source": "path"`.

Output must be deterministic: sorted keys and lists, so two runs on an unchanged tree differ only in `scanned_at`. Ship `schemas/facts.schema.json` and validate output against it in a test.

## Acceptance criteria
1. `init` writes a valid `config.toml` from ticked directories and refuses to overwrite silently.
2. `scan` on fixture repos produces `facts.json` matching the schema and the expected values.
3. A fixture repo containing `.env` with a sentinel string: the sentinel appears nowhere in `facts.json`, `state.json`, stdout or stderr.
4. Files ignored by `.gitignore` never influence languages or excerpts.
5. Two consecutive scans of an unchanged tree produce identical output except `scanned_at`.
6. After adding a commit to a fixture repo, the next scan reports `new_commits_since_last_scan` correctly.
7. Handles: repo with no commits, repo with no remote, detached HEAD, nested repos, unreadable directory (warn and continue).
8. Missing or invalid config exits non-zero with a message that says what to fix.
9. `--dry-run` writes nothing.
10. Tests use temp-dir fixtures created with `git init`. No test touches real directories.

## Lesson 1 (required)
`docs/tutorial/01-scan.md` must cover whichever of these the code actually uses: ownership and borrowing, `Result` and the `?` operator, structs and `serde` derive, `std::process::Command`, modules, `Option`, iterators. Each with a "try breaking this" exercise.
