use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// State tracking last-seen HEAD per repository.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    #[serde(default)]
    pub repos: BTreeMap<String, String>, // repo_id -> last_seen_head
}

impl State {
    /// Load state from file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read state file: {}", path.display()))?;
        let state: State = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse state file: {}", path.display()))?;
        Ok(state)
    }

    /// Save state to file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self).context("Failed to serialize state")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write state file: {}", path.display()))?;
        Ok(())
    }

    /// Get the last seen HEAD for a repo.
    pub fn get_last_seen(&self, repo_id: &str) -> Option<&String> {
        self.repos.get(repo_id)
    }

    /// Update the last seen HEAD for a repo.
    pub fn set_last_seen(&mut self, repo_id: String, head: String) {
        self.repos.insert(repo_id, head);
    }

    /// Calculate new commits since last scan.
    /// Returns the count of new commits.
    pub fn calc_new_commits(
        &self,
        repo_id: &str,
        current_head: &str,
        repo_path: &Path,
    ) -> Result<u32> {
        if let Some(last_head) = self.get_last_seen(repo_id) {
            if last_head == current_head {
                return Ok(0);
            }
            // First check if last_head is an ancestor of current_head
            // If not (e.g., rebased history), we can't reliably count new commits
            let is_ancestor = std::process::Command::new("git")
                .args(["merge-base", "--is-ancestor", last_head, current_head])
                .current_dir(repo_path)
                .status()
                .context("Failed to run git merge-base --is-ancestor")?;

            if !is_ancestor.success() {
                // History was rewritten (rebased, force-pushed, etc.)
                // Can't reliably determine new commits, return 0
                return Ok(0);
            }

            // Count commits between last_head and current_head
            let output = std::process::Command::new("git")
                .args([
                    "rev-list",
                    "--count",
                    &format!("{}..{}", last_head, current_head),
                ])
                .current_dir(repo_path)
                .output()
                .context("Failed to run git rev-list for new commits")?;

            if output.status.success() {
                let count = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(count.parse().unwrap_or(0))
            } else {
                Ok(0)
            }
        } else {
            // First scan, no previous state
            Ok(0)
        }
    }
}
