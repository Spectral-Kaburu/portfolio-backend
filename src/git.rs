use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Run a git command in the given directory and return stdout as a string.
fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("Failed to run git command: git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a directory is a git repository (contains .git file or directory).
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Get the root commit hash (first commit) for repo_id.
/// Returns first 12 characters.
pub fn get_root_commit(dir: &Path) -> Result<String> {
    let hash = run_git(dir, &["rev-list", "--max-parents=0", "HEAD"])?;
    let first_line = hash.lines().next().unwrap_or("");
    Ok(first_line.chars().take(12).collect())
}

/// Get the current HEAD commit hash (short). Returns None if no commits.
pub fn get_head(dir: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(dir)
        .output()
        .context("Failed to run git rev-parse HEAD")?;

    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// Get total commit count. Returns 0 if no commits.
pub fn get_commit_count(dir: &Path) -> Result<u32> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(dir)
        .output()
        .context("Failed to run git rev-list --count HEAD")?;

    if output.status.success() {
        let count = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(count.parse().unwrap_or(0))
    } else {
        Ok(0)
    }
}

/// Get the timestamp of the first commit (RFC 3339). Returns epoch if no commits.
pub fn get_first_commit_at(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["log", "--format=%ct", "--reverse"])
        .current_dir(dir)
        .output()
        .context("Failed to run git log")?;

    if output.status.success() {
        let ts = String::from_utf8_lossy(&output.stdout);
        let first_ts = ts.lines().next().unwrap_or("0");
        let timestamp: i64 = first_ts
            .parse()
            .context("Failed to parse first commit timestamp")?;
        Ok(chrono::DateTime::from_timestamp(timestamp, 0)
            .context("Invalid timestamp")?
            .to_rfc3339())
    } else {
        // Return epoch for empty repos
        Ok(chrono::DateTime::from_timestamp(0, 0).unwrap().to_rfc3339())
    }
}

/// Get the timestamp of the last commit (RFC 3339). Returns epoch if no commits.
pub fn get_last_commit_at(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["log", "--format=%ct", "-1"])
        .current_dir(dir)
        .output()
        .context("Failed to run git log")?;

    if output.status.success() {
        let ts = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let timestamp: i64 = ts
            .parse()
            .context("Failed to parse last commit timestamp")?;
        Ok(chrono::DateTime::from_timestamp(timestamp, 0)
            .context("Invalid timestamp")?
            .to_rfc3339())
    } else {
        // Return epoch for empty repos
        Ok(chrono::DateTime::from_timestamp(0, 0).unwrap().to_rfc3339())
    }
}

/// Get remote origin URL if it exists.
pub fn get_remote_url(dir: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(dir)
        .output()
        .context("Failed to run git remote get-url origin")?;

    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// Get list of tracked files (for language detection).
pub fn get_tracked_files(dir: &Path) -> Result<Vec<String>> {
    let files = run_git(dir, &["ls-files"])?;
    Ok(files.lines().map(|s| s.to_string()).collect())
}

/// Recent commit information.
#[derive(Debug, Clone)]
pub struct RecentCommit {
    pub hash: String,
    pub at: String, // RFC 3339
    pub subject: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// Get recent commits since a given date. Returns empty vec if no commits.
pub fn get_recent_commits(dir: &Path, since_days: u32) -> Result<Vec<RecentCommit>> {
    let since_date = chrono::Utc::now() - chrono::Duration::days(since_days as i64);
    let since_str = since_date.format("%Y-%m-%d").to_string();

    let output = Command::new("git")
        .args([
            "log",
            &format!("--since={}", since_str),
            "--numstat",
            "--format=%H%x00%s%x00",
        ])
        .current_dir(dir)
        .output()
        .context("Failed to run git log")?;

    if output.status.success() {
        parse_recent_commits(dir, &String::from_utf8_lossy(&output.stdout))
    } else {
        // Empty repo or no commits in range
        Ok(Vec::new())
    }
}

fn parse_recent_commits(dir: &Path, output: &str) -> Result<Vec<RecentCommit>> {
    let mut commits = Vec::new();
    let mut lines = output.lines().peekable();

    while lines.peek().is_some() {
        let header = lines.next().unwrap();
        let parts: Vec<&str> = header.split('\0').collect();
        if parts.len() < 2 {
            continue;
        }
        let hash = parts[0].chars().take(12).collect::<String>();
        let subject = parts[1].lines().next().unwrap_or("").to_string();

        let ts_output = run_git_for_commit(dir, &hash)?;
        let timestamp: i64 = ts_output
            .trim()
            .parse()
            .context("Failed to parse commit timestamp")?;
        let at = chrono::DateTime::from_timestamp(timestamp, 0)
            .context("Invalid timestamp")?
            .to_rfc3339();

        let mut files_changed = 0u32;
        let mut insertions = 0u32;
        let mut deletions = 0u32;

        while lines.peek().is_some() {
            let line = lines.peek().unwrap();
            if line.contains('\t') {
                let line = lines.next().unwrap();
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    if let Ok(ins) = parts[0].parse::<u32>() {
                        insertions += ins;
                    }
                    if let Ok(dels) = parts[1].parse::<u32>() {
                        deletions += dels;
                    }
                    files_changed += 1;
                }
            } else {
                break;
            }
        }

        commits.push(RecentCommit {
            hash,
            at,
            subject,
            files_changed,
            insertions,
            deletions,
        });
    }

    Ok(commits)
}

/// Helper to get timestamp for a specific commit hash.
fn run_git_for_commit(dir: &Path, hash: &str) -> Result<String> {
    run_git(dir, &["log", "-1", "--format=%ct", hash])
}

/// Read a file from the repo (for manifests/README), with size cap.
pub fn read_file_capped(dir: &Path, file: &str, max_bytes: usize) -> Result<Option<String>> {
    let path = dir.join(file);
    if !path.exists() {
        return Ok(None);
    }
    let metadata = std::fs::metadata(&path).context("Failed to read file metadata")?;
    if metadata.len() > max_bytes as u64 {
        let mut file_handle = std::fs::File::open(&path).context("Failed to open file")?;
        let mut buffer = vec![0u8; max_bytes];
        use std::io::Read;
        let n = file_handle
            .read(&mut buffer)
            .context("Failed to read file")?;
        buffer.truncate(n);
        Ok(Some(String::from_utf8_lossy(&buffer).to_string()))
    } else {
        let content = std::fs::read_to_string(&path).context("Failed to read file")?;
        Ok(Some(content))
    }
}
