use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::facts::{Facts, Project};
use crate::git;
use crate::languages;
use crate::state::State;

/// Maximum size for reading allowlisted files (8 KB).
const MAX_FILE_READ_SIZE: usize = 8 * 1024;

/// Allowlisted manifest files.
const MANIFESTS: &[(&str, &str)] = &[
    ("Cargo.toml", "rust"),
    ("package.json", "node"),
    ("pyproject.toml", "python"),
    ("requirements.txt", "python"),
    ("go.mod", "go"),
];

/// Allowlisted README patterns.
const README_PATTERNS: &[&str] = &["README.md", "README.rst", "README.txt", "README"];

/// Scan all configured roots and produce Facts.
pub fn scan(config: &Config, dry_run: bool) -> Result<Facts> {
    let mut all_projects = Vec::new();

    // Load previous state for new commit detection
    let state_path = Path::new("data/state.json");
    let state = State::load(state_path).context("Failed to load state")?;

    for root in &config.roots {
        let repos = discover_repos(root, config)?;
        for repo_path in repos {
            match process_repo(&repo_path, config, &state) {
                Ok(project) => all_projects.push(project),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to process repo {}: {}",
                        repo_path.display(),
                        e
                    );
                    // Continue with other repos
                }
            }
        }
    }

    // Sort by repo_id for deterministic output
    all_projects.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));

    let facts = Facts::new(all_projects);

    if !dry_run {
        // Save updated state
        let mut new_state = state.clone();
        for project in &facts.projects {
            new_state.set_last_seen(project.repo_id.clone(), project.head.clone());
        }
        std::fs::create_dir_all("data").context("Failed to create data directory")?;
        new_state.save(state_path).context("Failed to save state")?;

        // Write facts.json
        facts
            .write_to_file(Path::new("data/facts.json"))
            .context("Failed to write facts.json")?;
    }

    Ok(facts)
}

/// Discover git repositories under a root directory.
fn discover_repos(root: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();
    let mut skip_paths: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(root)
        .max_depth(config.max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip paths that are inside already-discovered repos
        if skip_paths.iter().any(|skip| path.starts_with(skip)) {
            continue;
        }

        // Skip excluded directories
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && config.exclude_dirs.contains(&file_name.to_string())
        {
            continue;
        }

        // Check if this is a git repo
        if git::is_git_repo(path) {
            repos.push(path.to_path_buf());
            skip_paths.push(path.to_path_buf());
        }
    }

    Ok(repos)
}

/// Process a single repository and extract all facts.
fn process_repo(repo_path: &Path, config: &Config, state: &State) -> Result<Project> {
    // Get repo_id from root commit
    let (repo_id, id_source) = match git::get_root_commit(repo_path) {
        Ok(root_commit) if !root_commit.is_empty() => (root_commit, None),
        _ => {
            // Fallback: hash of directory name
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            repo_path.hash(&mut hasher);
            let hash = format!("{:x}", hasher.finish())[..12].to_string();
            (hash, Some("path".to_string()))
        }
    };

    let name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let remote_url = git::get_remote_url(repo_path)?;
    let head_opt = git::get_head(repo_path)?;
    let head = head_opt.unwrap_or_else(|| "000000000000".to_string());
    let commit_count = git::get_commit_count(repo_path)?;
    let first_commit_at = git::get_first_commit_at(repo_path)?;
    let last_commit_at = git::get_last_commit_at(repo_path)?;

    // Get tracked files for language detection
    let tracked_files = git::get_tracked_files(repo_path)?;
    let languages = languages::detect_languages(repo_path, &tracked_files)?;

    // Get manifests
    let manifests = detect_manifests(repo_path)?;

    // Get README excerpt
    let readme_excerpt = detect_readme_excerpt(repo_path)?;

    // Get recent commits
    let recent_commits = git::get_recent_commits(repo_path, config.active_days)?;

    // Calculate new commits since last scan
    let new_commits_since_last_scan = state.calc_new_commits(&repo_id, &head, repo_path)?;

    // Convert recent commits to facts format
    let recent_commits: Result<Vec<crate::facts::RecentCommit>> = recent_commits
        .into_iter()
        .map(|rc| {
            Ok(crate::facts::RecentCommit {
                hash: rc.hash,
                at: rc.at.parse().context("Failed to parse commit timestamp")?,
                subject: rc.subject,
                files_changed: rc.files_changed,
                insertions: rc.insertions,
                deletions: rc.deletions,
            })
        })
        .collect();
    let recent_commits = recent_commits?;

    // Convert languages from languages::Language to facts::Language
    let languages: Vec<crate::facts::Language> = languages
        .into_iter()
        .map(|l| crate::facts::Language {
            name: l.name,
            bytes: l.bytes,
            share: l.share,
        })
        .collect();

    let project = Project {
        repo_id,
        name,
        remote_url,
        head,
        commit_count,
        first_commit_at: first_commit_at
            .parse()
            .context("Failed to parse first commit timestamp")?,
        last_commit_at: last_commit_at
            .parse()
            .context("Failed to parse last commit timestamp")?,
        languages,
        manifests,
        readme_excerpt,
        recent_commits,
        new_commits_since_last_scan,
        id_source,
    };

    Ok(project)
}

/// Detect manifest files in the repository.
fn detect_manifests(repo_path: &Path) -> Result<Vec<crate::facts::Manifest>> {
    let mut manifests = Vec::new();

    for (file, kind) in MANIFESTS {
        if repo_path.join(file).exists() {
            // Verify it's tracked by git (not in .gitignore)
            if is_tracked(repo_path, file)? {
                manifests.push(crate::facts::Manifest {
                    file: file.to_string(),
                    kind: kind.to_string(),
                });
            }
        }
    }

    Ok(manifests)
}
#[allow(clippy::collapsible_if)]
fn detect_readme_excerpt(repo_path: &Path) -> Result<Option<String>> {
    for pattern in README_PATTERNS {
        let path = repo_path.join(pattern);
        if path.exists() && is_tracked(repo_path, pattern)? {
            if let Some(content) = git::read_file_capped(repo_path, pattern, MAX_FILE_READ_SIZE)? {
                return Ok(Some(content));
            }
        }
    }
    Ok(None)
}

/// Check if a file is tracked by git (not ignored).
fn is_tracked(repo_path: &Path, file: &str) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["check-ignore", file])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git check-ignore")?;

    // git check-ignore returns 0 if ignored, 1 if not ignored
    Ok(!output.status.success())
}
