//! Digest module - builds AI request digest from facts.
//!
//! RUST NOTE: The digest is the only thing that may leave the machine.
//! It contains no paths, hashes, author data, or sub-date timestamps.

use crate::facts::Facts;
use crate::model::{DigestLanguage, ProjectStatus};

/// A digest item for a single project (sent to AI).
#[derive(Debug, Clone)]
pub struct ProjectDigest {
    pub name: String,
    pub status: ProjectStatus,
    pub commit_count: u32,
    pub first_commit_at: String, // Date only, no time
    pub last_commit_at: String,  // Date only, no time
    pub recent_commit_count: usize,
    pub languages: Vec<DigestLanguage>,
    pub manifest_kinds: Vec<String>,
    pub readme_excerpt: Option<String>,
    pub recent_commits: Vec<RecentCommitDigest>,
    pub stats: DigestStats,
}

/// Recent commit for digest (subject only, truncated).
#[derive(Debug, Clone)]
pub struct RecentCommitDigest {
    pub subject: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// Aggregate commit stats for digest.
#[derive(Debug, Clone)]
pub struct DigestStats {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// Build digest for a single project.
pub fn build_project_digest(project: &crate::facts::Project, active_days: i64) -> ProjectDigest {
    // Stub - implementation in M2
    ProjectDigest {
        name: project.name.clone(),
        status: ProjectStatus::Dormant,
        commit_count: project.commit_count,
        first_commit_at: String::new(),
        last_commit_at: String::new(),
        recent_commit_count: 0,
        languages: Vec::new(),
        manifest_kinds: Vec::new(),
        readme_excerpt: None,
        recent_commits: Vec::new(),
        stats: DigestStats {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        },
    }
}

/// Build the complete AI request digest.
pub fn build_digest(facts: &Facts, active_days: i64, max_bytes: usize) -> String {
    // Stub - implementation in M2
    String::new()
}

/// The evidence map for resolving IDs to references.
pub type EvidenceMap = std::collections::HashMap<String, String>;

/// Build evidence map for a project's digest.
pub fn build_evidence_map(project: &crate::facts::Project) -> EvidenceMap {
    // Stub - implementation in M2
    EvidenceMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub() {
        // Stub test
    }
}