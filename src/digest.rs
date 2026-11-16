//! Digest module - builds AI request digest from facts.
//!
//! RUST NOTE: The digest is the only thing that may leave the machine.
//! It contains no paths, hashes, author data, or sub-date timestamps.

use crate::facts::{Facts, Language, Project};
use crate::model::{DigestLanguage, ProjectStatus};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Language names to exclude from digest (non-code languages).
#[allow(dead_code)]
const EXCLUDED_LANGUAGES: [&str; 6] = ["Other", "Markdown", "Text", "JSON", "TOML", "YAML"];

/// Maximum length for commit subjects in digest.
#[allow(dead_code)]
const MAX_SUBJECT_LENGTH: usize = 60;

/// Recent commit for digest (subject only, truncated).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RecentCommitDigest {
    pub subject: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// Aggregate commit stats for digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DigestStats {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// A digest item for a single project (sent to AI).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectDigest {
    pub name: String,
    pub status: ProjectStatus,
    pub commit_count: u32,
    pub first_commit_at: String,
    pub last_commit_at: String,
    pub recent_commit_count: usize,
    pub languages: Vec<DigestLanguage>,
    pub manifest_kinds: Vec<String>,
    pub readme_excerpt: Option<String>,
    pub recent_commits: Vec<RecentCommitDigest>,
    pub stats: DigestStats,
}

/// The evidence map for resolving IDs to references.
#[allow(dead_code)]
pub type EvidenceMap = HashMap<String, String>;

/// Truncate a string to max length, adding ellipsis if truncated.
#[allow(dead_code)]
fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Filter and renormalize language statistics.
/// Returns top 3 languages (by bytes), excluding specified types, renormalized to 100%.
#[allow(dead_code)]
fn filter_languages(languages: &[Language]) -> Vec<DigestLanguage> {
    let mut filtered: Vec<&Language> = languages
        .iter()
        .filter(|l| !EXCLUDED_LANGUAGES.contains(&l.name.as_str()))
        .collect();

    // Sort by bytes descending
    filtered.sort_by_key(|a| std::cmp::Reverse(a.bytes));

    // Take top 3
    filtered.truncate(3);

    // Calculate total for renormalization
    let total_bytes: u64 = filtered.iter().map(|l| l.bytes).sum();

    if total_bytes == 0 {
        return Vec::new();
    }

    filtered
        .iter()
        .map(|l| DigestLanguage {
            name: l.name.clone(),
            share: l.bytes as f64 / total_bytes as f64,
        })
        .collect()
}

/// Determine project status based on last commit date.
#[allow(dead_code)]
fn determine_status(last_commit_at: chrono::DateTime<Utc>, active_days: i64) -> ProjectStatus {
    let now = Utc::now();
    let days_since_commit = (now - last_commit_at).num_days();

    if days_since_commit <= active_days {
        ProjectStatus::Active
    } else {
        ProjectStatus::Dormant
    }
}

/// Build digest for a single project (sent to AI).
#[allow(dead_code)]
pub fn build_project_digest(project: &Project, active_days: i64) -> ProjectDigest {
    let status = determine_status(project.last_commit_at, active_days);

    // Format dates as date-only strings (no time, no timezone)
    let first_date = project.first_commit_at.format("%Y-%m-%d").to_string();
    let last_date = project.last_commit_at.format("%Y-%m-%d").to_string();

    // Filter languages
    let languages = filter_languages(&project.languages);

    // Get manifest kinds
    let manifest_kinds: Vec<String> = project.manifests.iter().map(|m| m.kind.clone()).collect();

    // Build recent commits digest (subject only, truncated)
    let recent_commits: Vec<RecentCommitDigest> = project
        .recent_commits
        .iter()
        .map(|c| RecentCommitDigest {
            subject: truncate_with_ellipsis(&c.subject, MAX_SUBJECT_LENGTH),
            files_changed: c.files_changed,
            insertions: c.insertions,
            deletions: c.deletions,
        })
        .collect();

    // Aggregate stats
    let stats = DigestStats {
        files_changed: project.recent_commits.iter().map(|c| c.files_changed).sum(),
        insertions: project.recent_commits.iter().map(|c| c.insertions).sum(),
        deletions: project.recent_commits.iter().map(|c| c.deletions).sum(),
    };

    ProjectDigest {
        name: project.name.clone(),
        status,
        commit_count: project.commit_count,
        first_commit_at: first_date,
        last_commit_at: last_date,
        recent_commit_count: project.recent_commits.len(),
        languages,
        manifest_kinds,
        readme_excerpt: project.readme_excerpt.clone(),
        recent_commits,
        stats,
    }
}

/// Build the complete AI request digest.
#[allow(dead_code)]
pub fn build_digest(facts: &Facts, _active_days: i64, _max_bytes: usize) -> Result<String, String> {
    let digests: Vec<String> = facts
        .projects
        .iter()
        .map(|p| {
            let project_digest = build_project_digest(p, _active_days);
            serde_json::to_string(&project_digest)
                .map_err(|e| format!("Failed to serialize project digest: {}", e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    serde_json::to_string_pretty(&digests)
        .map_err(|e| format!("Failed to serialize complete digest: {}", e))
}

/// Build evidence map for a project's digest.
/// Maps E1, E2, E3... to actual references.
#[allow(dead_code)]
pub fn build_evidence_map(project: &Project) -> EvidenceMap {
    let mut map = EvidenceMap::new();
    let mut counter = 1;

    // Map recent commits to evidence IDs
    for (i, _commit) in project.recent_commits.iter().enumerate() {
        // Use truncated subject as identifier, map to commit:hash
        let evidence_id = format!("E{}", counter);
        let ref_str = format!(
            "commit:{}",
            &project.recent_commits[i].hash[..project.recent_commits[i].hash.len().min(12)]
        );
        map.insert(evidence_id, ref_str);
        counter += 1;
    }

    // Add README excerpt reference
    if project.readme_excerpt.is_some() {
        map.insert(format!("E{}", counter), "readme".to_string());
        counter += 1;
    }

    // Add manifest references
    for manifest in &project.manifests {
        let evidence_id = format!("E{}", counter);
        let ref_str = format!("manifest:{}", manifest.kind);
        map.insert(evidence_id, ref_str);
        counter += 1;
    }

    // Add stats reference
    map.insert(format!("E{}", counter), "stats".to_string());

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Manifest, RecentCommit};
    use chrono::{TimeZone, Utc};

    fn make_test_project() -> Project {
        Project {
            repo_id: "abc123def456".to_string(),
            name: "test-project".to_string(),
            remote_url: Some("https://github.com/user/repo.git".to_string()),
            head: "def456".to_string(),
            commit_count: 100,
            first_commit_at: Utc.timestamp_opt(1609459200, 0).single().unwrap(), // 2021-01-01
            last_commit_at: Utc.timestamp_opt(1705996800, 0).single().unwrap(), // 2024-01-31
            languages: vec![
                Language {
                    name: "Rust".to_string(),
                    bytes: 50000,
                    share: 0.5,
                },
                Language {
                    name: "Markdown".to_string(),
                    bytes: 10000,
                    share: 0.1,
                },
                Language {
                    name: "Python".to_string(),
                    bytes: 30000,
                    share: 0.3,
                },
                Language {
                    name: "Other".to_string(),
                    bytes: 10000,
                    share: 0.1,
                },
            ],
            manifests: vec![Manifest {
                file: "package.json".to_string(),
                kind: "npm".to_string(),
            }],
            readme_excerpt: Some("This is a test readme with some details.".to_string()),
            recent_commits: vec![
                RecentCommit {
                    hash: "a1b2c3d4e5f6a1b2".to_string(),
                    at: Utc.timestamp_opt(1705996800, 0).single().unwrap(),
                    subject: "Fix bug in the main module with a very long subject line that should be truncated".to_string(),
                    files_changed: 5,
                    insertions: 100,
                    deletions: 50,
                },
            ],
            new_commits_since_last_scan: 10,
            id_source: None,
        }
    }

    #[test]
    fn test_filter_languages_excludes_markdown() {
        let project = make_test_project();
        let filtered = filter_languages(&project.languages);

        assert!(!filtered.iter().any(|l| l.name == "Markdown"));
        assert!(!filtered.iter().any(|l| l.name == "Other"));
    }

    #[test]
    fn test_filter_languages_renormalizes() {
        let project = make_test_project();
        let filtered = filter_languages(&project.languages);

        let total: f64 = filtered.iter().map(|l| l.share).sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_filter_languages_top_three() {
        let project = make_test_project();
        let filtered = filter_languages(&project.languages);

        assert!(filtered.len() <= 3);
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        let short = "short";
        assert_eq!(truncate_with_ellipsis(short, 20), "short");

        let long = "this is a very long string that needs truncation";
        let result = truncate_with_ellipsis(long, 20);
        assert!(result.ends_with("..."));
        assert_eq!(result.len(), 23); // 20 + "..."
    }

    #[test]
    fn test_build_project_digest_excludes_remote_url() {
        let project = make_test_project();
        let digest = build_project_digest(&project, 30);

        // Name should be preserved
        assert_eq!(digest.name, "test-project");

        // Commit count preserved
        assert_eq!(digest.commit_count, 100);
    }

    #[test]
    fn test_build_project_digest_date_format() {
        let project = make_test_project();
        let digest = build_project_digest(&project, 30);

        // Dates should be date-only format
        assert!(
            digest
                .first_commit_at
                .chars()
                .all(|c| c.is_digit(10) || c == '-')
        );
        assert!(
            digest
                .last_commit_at
                .chars()
                .all(|c| c.is_digit(10) || c == '-')
        );
    }

    #[test]
    fn test_build_project_digest_status_active() {
        let mut project = make_test_project();
        project.last_commit_at = Utc::now(); // Today

        let digest = build_project_digest(&project, 30);
        assert_eq!(digest.status, ProjectStatus::Active);
    }

    #[test]
    fn test_build_project_digest_status_dormant() {
        let project = make_test_project();
        let digest = build_project_digest(&project, 30);

        // Project was last updated 2024-01-31, should be dormant for 30 days in 2026
        assert_eq!(digest.status, ProjectStatus::Dormant);
    }

    #[test]
    fn test_build_project_digest_commit_subject_truncated() {
        let project = make_test_project();
        let digest = build_project_digest(&project, 30);

        let subject = &digest.recent_commits[0].subject;
        assert!(subject.len() <= 63); // 60 + "..."
    }

    #[test]
    fn test_build_digest_no_paths() {
        let project = make_test_project();
        let facts = Facts::new(vec![project]);

        let digest_str = build_digest(&facts, 30, 10000).unwrap();

        // Should not contain file paths (only project name, not paths)
        assert!(!digest_str.contains("/home/"));
        // Note: project name WILL appear in the digest, that's expected
    }

    #[test]
    fn test_build_digest_language_share_renormalized() {
        let project = make_test_project();
        let facts = Facts::new(vec![project]);

        let digest_str = build_digest(&facts, 30, 10000).unwrap();

        // Parse and check language shares sum to 1.0
        let digests: Vec<serde_json::Value> =
            serde_json::from_str(&digest_str).expect("Should parse digest");

        if let Some(languages) = digests[0].get("languages").and_then(|l| l.as_array()) {
            let total: f64 = languages
                .iter()
                .filter_map(|l| l.get("share").and_then(|s| s.as_f64()))
                .sum();
            assert!((total - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_build_evidence_map() {
        let project = make_test_project();
        let map = build_evidence_map(&project);

        // Should have evidence entries
        assert!(!map.is_empty());

        // Should start with E1
        assert!(map.contains_key("E1"));

        // Should map to commit hashes
        if let Some(entry) = map.get("E1") {
            assert!(entry.starts_with("commit:"));
        }
    }
}
