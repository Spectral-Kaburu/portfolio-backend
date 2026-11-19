use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Serialize DateTime<Utc> as RFC 3339 string.
mod rfc3339 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&dt.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

/// Top-level facts container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facts {
    pub schema_version: u32,
    #[serde(with = "rfc3339")]
    pub scanned_at: DateTime<Utc>,
    pub projects: Vec<Project>,
}

/// A single project/repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub repo_id: String,
    pub name: String,
    pub remote_url: Option<String>,
    pub head: String,
    pub commit_count: u32,
    #[serde(with = "rfc3339")]
    pub first_commit_at: DateTime<Utc>,
    #[serde(with = "rfc3339")]
    pub last_commit_at: DateTime<Utc>,
    pub languages: Vec<Language>,
    pub manifests: Vec<Manifest>,
    pub readme_excerpt: Option<String>,
    pub recent_commits: Vec<RecentCommit>,
    pub new_commits_since_last_scan: u32,
    pub id_source: Option<String>,
}

/// Language statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub bytes: u64,
    pub share: f64,
}

/// Manifest file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub file: String,
    pub kind: String,
}

/// Recent commit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentCommit {
    pub hash: String,
    #[serde(with = "rfc3339")]
    pub at: DateTime<Utc>,
    pub subject: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

impl Facts {
    /// Create a new Facts container with current timestamp.
    pub fn new(projects: Vec<Project>) -> Self {
        // Sort projects by repo_id for deterministic output
        let mut projects = projects;
        projects.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));

        // Ensure languages and manifests are sorted within each project
        for project in &mut projects {
            project
                .languages
                .sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
            project.manifests.sort_by(|a, b| a.file.cmp(&b.file));
            project
                .recent_commits
                .sort_by_key(|a| std::cmp::Reverse(a.at));
        }

        Self {
            schema_version: 1,
            scanned_at: Utc::now(),
            projects,
        }
    }

    /// Write facts to a JSON file with deterministic ordering.
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize facts to JSON")?;
        std::fs::write(path, json)
            .with_context(|| format!("Failed to write facts to {}", path.display()))?;
        Ok(())
    }

    /// Validate facts against JSON schema.
    pub fn validate_schema(&self, schema_path: &Path) -> Result<()> {
        let schema_content = std::fs::read_to_string(schema_path)
            .with_context(|| format!("Failed to read schema: {}", schema_path.display()))?;
        let schema = jsonschema::Validator::new(&serde_json::from_str(&schema_content)?)
            .context("Failed to compile JSON schema")?;

        let instance =
            serde_json::to_value(self).context("Failed to convert facts to JSON value")?;

        if let Err(errors) = schema.validate(&instance) {
            let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            anyhow::bail!("Schema validation failed: {}", error_msgs.join("; "));
        }
        Ok(())
    }

    /// Deserialize Facts from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        let facts: Facts = serde_json::from_str(json).context("Failed to parse facts JSON")?;
        Ok(facts)
    }
}

/// RUST NOTE: Timestamp parsing is strict; malformed RFC 3339 timestamps will
/// fail deserialization with an error. This ensures data integrity for facts.json
/// produced by the scanner. Lenient parsing (e.g., accepting "invalid" formats)
/// is intentionally not supported to prevent silent data corruption.

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use serde_json;

    /// Test that valid RFC 3339 timestamps deserialize correctly.
    #[test]
    fn test_rfc3339_valid_timestamp() {
        let json = r#""2024-01-15T10:30:00Z""#;
        let result: Result<DateTime<Utc>, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    /// Test that malformed RFC 3339 timestamps fail deserialization.
    #[test]
    fn test_rfc3339_malformed_timestamp_fails() {
        let json = r#""not-a-timestamp""#;
        let result: Result<DateTime<Utc>, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Expected error for malformed timestamp");
    }

    /// Test that incomplete timestamps fail deserialization.
    #[test]
    fn test_rfc3339_incomplete_timestamp_fails() {
        let json = r#""2024-01-15""#;
        let result: Result<DateTime<Utc>, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Expected error for incomplete timestamp");
    }

    /// Test Facts struct with valid timestamps deserializes correctly.
    #[test]
    fn test_facts_valid_deserialization() {
        let json = r#"{
            "schema_version": 1,
            "scanned_at": "2024-01-15T10:30:00Z",
            "projects": []
        }"#;
        let result: Result<Facts, _> = serde_json::from_str(json);
        assert!(result.is_ok());
    }

    /// Test Facts struct with malformed timestamp in scanned_at fails deserialization.
    #[test]
    fn test_facts_malformed_scanned_at_fails() {
        let json = r#"{
            "schema_version": 1,
            "scanned_at": "invalid-timestamp",
            "projects": []
        }"#;
        let result: Result<Facts, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Expected error for malformed timestamp in Facts"
        );
    }
}
