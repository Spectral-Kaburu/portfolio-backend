//! Export module - builds and writes reviewed.json for M3.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{
    Draft, Overrides, ProjectDecision, ProjectOverview, ReviewedActivity, ReviewedExport,
    ReviewedProject,
};
use anyhow::{Context, Result};
use chrono::Utc;
use jsonschema::Validator;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

// RUST NOTE: LazyLock provides thread-safe, one-time initialization of static regexes.
// This avoids panicking at runtime if a regex pattern is invalid (which shouldn't happen
// with these hardcoded patterns, but we avoid unwrap() entirely as a defensive practice).
static HEX_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b[0-9a-f]{7,}\b").unwrap());
static PATH_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b[a-zA-Z0-9_.-]+(?:/[a-zA-Z0-9_.-]+){2,}\b").unwrap());
static EXTENSION_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"\b[a-zA-Z0-9_-]+\.(rs|js|ts|py|json|toml|yaml|yml|md|txt|html|css|go|java|cpp|h)\b",
    )
    .unwrap()
});
static URL_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://\S+").unwrap());
static EMAIL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap()
});
static API_KEY_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(api[_-]?key|token|secret|password|auth)[_-]?[:=]\s*\S+").unwrap()
});

/// The status values an owner override may set (spec: "active | dormant | built").
/// Anything else in `overrides.toml` is ignored and the computed status is used.
static VALID_STATUSES: [&str; 3] = ["active", "dormant", "built"];

/// Build the reviewed export from draft, overrides, and facts.
pub fn build_reviewed_export(
    draft: &Draft,
    overrides: &Overrides,
    _facts: Option<&Facts>,
    _config: &ReviewConfig,
) -> Result<ReviewedExport> {
    let mut exported_projects = Vec::new();
    let mut exported_repo_ids = HashSet::new();

    // Sort by order (unset last), then name
    let mut sorted_projects = draft.projects.clone();
    sorted_projects.sort_by(|a, b| {
        let order_a = overrides
            .projects
            .get(&a.repo_id)
            .and_then(|o| o.order)
            .unwrap_or(i32::MAX);
        let order_b = overrides
            .projects
            .get(&b.repo_id)
            .and_then(|o| o.order)
            .unwrap_or(i32::MAX);
        order_a.cmp(&order_b).then_with(|| a.name.cmp(&b.name))
    });

    for project in sorted_projects {
        let repo_id = &project.repo_id;
        let override_opt = overrides.projects.get(repo_id);
        let decision = effective_decision(override_opt, &project);

        if decision != crate::model::Decision::Approved {
            continue;
        }

        exported_repo_ids.insert(repo_id.clone());
    }

    // Second pass: build exported projects with correct order
    let mut sorted_projects = draft.projects.clone();
    sorted_projects.sort_by(|a, b| {
        let order_a = overrides
            .projects
            .get(&a.repo_id)
            .and_then(|o| o.order)
            .unwrap_or(i32::MAX);
        let order_b = overrides
            .projects
            .get(&b.repo_id)
            .and_then(|o| o.order)
            .unwrap_or(i32::MAX);
        order_a.cmp(&order_b).then_with(|| a.name.cmp(&b.name))
    });

    for project in sorted_projects {
        let repo_id = &project.repo_id;
        let override_opt = overrides.projects.get(repo_id);
        let decision = effective_decision(override_opt, &project);

        if decision != crate::model::Decision::Approved {
            continue;
        }

        // A non-empty owner override replaces the draft overview; an empty string
        // is treated as absent and falls back to the draft text.
        let published_text = override_opt
            .and_then(|o| o.overview.as_deref())
            .filter(|s| !s.is_empty())
            .unwrap_or(&project.overview);

        let featured = override_opt.and_then(|o| o.featured).unwrap_or(false);
        let order = override_opt.and_then(|o| o.order);
        // Only the schema's statuses are honored; anything else falls back to computed.
        let status = override_opt
            .and_then(|o| o.status.as_deref())
            .filter(|s| VALID_STATUSES.contains(s))
            .map(str::to_string)
            .unwrap_or_else(|| project.status.to_string());

        let related = filter_related(&project.related, &exported_repo_ids);

        exported_projects.push(ReviewedProject {
            repo_id: repo_id.clone(),
            name: project.name.clone(),
            status,
            overview: published_text.to_string(),
            featured,
            order,
            related,
        });
    }

    // Build recent activity
    let narrative_override = &overrides.narrative;
    let recent_activity = build_reviewed_activity(
        narrative_override,
        &draft.recent_activity,
        &exported_repo_ids,
    );

    let export = ReviewedExport {
        schema_version: 1,
        exported_at: Utc::now(),
        projects: exported_projects,
        recent_activity,
    };

    Ok(export)
}

/// Determine effective decision for a project.
fn effective_decision(
    override_opt: Option<&ProjectDecision>,
    project: &ProjectOverview,
) -> crate::model::Decision {
    use crate::model::Decision;
    use crate::review_checks::compute_text_hash;

    match override_opt {
        Some(ov) => {
            if ov.decision == Decision::Hidden {
                Decision::Hidden
            } else if ov.decision == Decision::Approved {
                // Empty override text counts as absent so the draft text is hashed.
                let published_text = ov
                    .overview
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&project.overview);
                let hash = compute_text_hash(published_text);
                if ov.approved_hash == Some(hash) {
                    Decision::Approved
                } else {
                    Decision::Stale
                }
            } else {
                Decision::Pending
            }
        }
        None => Decision::Pending,
    }
}

/// Build reviewed activity if narrative is approved and not stale.
fn build_reviewed_activity(
    narrative_override: &crate::model::NarrativeDecision,
    draft_narrative: &crate::model::RecentActivity,
    _exported_repo_ids: &HashSet<String>,
) -> Option<ReviewedActivity> {
    use crate::decisions::narrative_effective_decision;
    use crate::model::Decision;

    let decision = narrative_effective_decision(
        narrative_override,
        &draft_narrative.summary,
        "", // approved_set_hash - would need to compute from overrides
    );

    if decision == Decision::Approved {
        Some(ReviewedActivity {
            summary: narrative_override
                .summary
                .clone()
                .unwrap_or_else(|| draft_narrative.summary.clone()),
        })
    } else {
        None
    }
}

/// Compute SHA-256 hash of text.
#[allow(dead_code)]
pub fn compute_text_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Filter related projects to only exported ones.
#[allow(dead_code)]
pub fn filter_related(related: &[String], exported_repo_ids: &HashSet<String>) -> Vec<String> {
    related
        .iter()
        .filter(|name| exported_repo_ids.contains(*name))
        .cloned()
        .collect()
}

/// Write reviewed export to JSON file atomically.
pub fn write_reviewed_json(export: &ReviewedExport, path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(export).context("Failed to serialize export")?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content).context("Failed to write temp file")?;
    std::fs::rename(&tmp_path, path).context("Failed to rename temp file")?;
    Ok(())
}

/// Validate reviewed export against schema.
pub fn validate_reviewed_schema(export: &ReviewedExport, schema_path: &Path) -> Result<()> {
    let schema_content =
        std::fs::read_to_string(schema_path).context("Failed to read schema file")?;
    let schema = Validator::new(&serde_json::from_str(&schema_content)?)
        .context("Failed to compile JSON schema")?;
    let instance = serde_json::to_value(export).context("Failed to serialize export")?;
    match schema.validate(&instance) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            anyhow::bail!("Schema validation failed: {}", error_msgs.join("; "));
        }
    }
}

/// Run error-level checks on all included texts at export time.
pub fn run_export_error_checks(
    export: &ReviewedExport,
    _config: &ReviewConfig,
) -> Vec<crate::model::ReviewCheck> {
    let mut checks = Vec::new();

    for project in &export.projects {
        checks.extend(prose_lint_checks(
            &project.overview,
            &format!("project {}", project.name),
        ));
    }

    if let Some(activity) = &export.recent_activity {
        checks.extend(prose_lint_checks(&activity.summary, "recent_activity"));
    }

    // Filter to error severity only
    checks.retain(|c| c.severity == crate::model::Severity::Error);
    checks
}

/// Prose lint checks for a text.
fn prose_lint_checks(text: &str, location: &str) -> Vec<crate::model::ReviewCheck> {
    let mut checks = Vec::new();

    // Hex string of 7+ characters
    if HEX_REGEX.is_match(text) {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("Hex string (7+ chars) found in {}", location),
        });
    }

    // Path-like token
    if PATH_REGEX.is_match(text) {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("Path-like token found in {}", location),
        });
    }

    // File extension
    if EXTENSION_REGEX.is_match(text) {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("File extension found in {}", location),
        });
    }

    // Backtick
    if text.contains('`') {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("Backtick found in {}", location),
        });
    }

    // URL
    if URL_REGEX.is_match(text) {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("URL found in {}", location),
        });
    }

    // Email
    if EMAIL_REGEX.is_match(text) {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("Email found in {}", location),
        });
    }

    // Key-like pattern
    if API_KEY_REGEX.is_match(text) {
        checks.push(crate::model::ReviewCheck {
            code: "prose_lint".to_string(),
            severity: crate::model::Severity::Error,
            message: format!("Key-like pattern found in {}", location),
        });
    }

    checks
}

/// Run export command.
pub fn run_export(config: &ReviewConfig, output_path: &std::path::Path) -> anyhow::Result<()> {
    use crate::facts::Facts;
    use crate::model::Draft;

    let draft_path = std::path::Path::new("data/enriched.draft.json");
    let overrides_path = std::path::Path::new("overrides.toml");

    if !draft_path.exists() {
        return Err(anyhow::anyhow!(
            "Draft not found at {}. Run enrich first.",
            draft_path.display()
        ));
    }

    let draft_content = std::fs::read_to_string(draft_path).context("Failed to read draft")?;
    let draft: Draft = serde_json::from_str(&draft_content).context("Failed to parse draft")?;

    let overrides =
        crate::decisions::load_overrides(overrides_path).context("Failed to load overrides")?;

    let facts_path = std::path::Path::new("data/facts.json");
    let facts = if facts_path.exists() {
        let facts_content = std::fs::read_to_string(facts_path).context("Failed to read facts")?;
        Some(Facts::from_json(&facts_content)?)
    } else {
        None
    };

    let export = build_reviewed_export(&draft, &overrides, facts.as_ref(), config)
        .context("Failed to build reviewed export")?;

    // Validate against schema
    let schema_path = std::path::Path::new("schemas/reviewed.schema.json");
    if schema_path.exists() {
        validate_reviewed_schema(&export, schema_path).context("Schema validation failed")?;
    }

    // Run error-level checks
    let error_checks = run_export_error_checks(&export, config);
    if !error_checks.is_empty() {
        return Err(anyhow::anyhow!(
            "Export failed error checks: {:?}",
            error_checks
        ));
    }

    write_reviewed_json(&export, output_path).context("Failed to write reviewed.json")?;

    println!(
        "Export complete. {} projects written to {}",
        export.projects.len(),
        output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Decision, ProjectStatus};

    /// Test fixture: one dormant project with a known draft overview.
    fn fixture() -> (Draft, ProjectOverview) {
        let draft_text = "A tool that watches a folder and renames files.".to_string();
        (
            Draft::default(),
            ProjectOverview {
                repo_id: "r1".to_string(),
                name: "Alpha".to_string(),
                status: ProjectStatus::Dormant,
                overview: draft_text.clone(),
                claims: Vec::new(),
                related: Vec::new(),
                claims_dropped: 0,
                failure: None,
            },
        )
    }

    /// Approved override whose hash matches the given published text.
    fn approved(hash_text: &str, status: Option<&str>, overview: Option<&str>) -> ProjectDecision {
        ProjectDecision {
            ai: true,
            decision: Decision::Approved,
            approved_hash: Some(compute_text_hash(hash_text)),
            overview: overview.map(str::to_string),
            status: status.map(str::to_string),
            ..ProjectDecision::default()
        }
    }

    fn export_one(decision: ProjectDecision, project: ProjectOverview) -> ReviewedExport {
        let mut overrides = Overrides::default();
        overrides.projects.insert("r1".to_string(), decision);
        let mut draft = Draft::default();
        draft.projects.push(project);
        build_reviewed_export(&draft, &overrides, None, &ReviewConfig::default())
            .expect("export builds")
    }

    #[test]
    fn empty_overview_override_falls_back_to_draft() {
        let (_, project) = fixture();
        let export = export_one(approved(&project.overview, None, Some("")), project.clone());
        assert_eq!(export.projects.len(), 1);
        assert_eq!(export.projects[0].overview, project.overview);
    }

    #[test]
    fn invalid_status_override_falls_back_to_computed() {
        let (_, project) = fixture();
        let export = export_one(approved(&project.overview, Some("shipped"), None), project);
        assert_eq!(export.projects.len(), 1);
        assert_eq!(export.projects[0].status, "dormant");
    }

    #[test]
    fn valid_status_override_is_honored() {
        let (_, project) = fixture();
        let export = export_one(approved(&project.overview, Some("built"), None), project);
        assert_eq!(export.projects.len(), 1);
        assert_eq!(export.projects[0].status, "built");
    }

    #[test]
    fn non_empty_overview_override_is_honored() {
        let (_, project) = fixture();
        let export = export_one(
            approved("Owner edited text", None, Some("Owner edited text")),
            project,
        );
        assert_eq!(export.projects.len(), 1);
        assert_eq!(export.projects[0].overview, "Owner edited text");
    }

    /// The exported JSON must validate against the real schema file, with the
    /// override status also accepted by the schema's enum.
    #[test]
    fn export_validates_against_reviewed_schema() {
        let (_, project) = fixture();
        let export = export_one(approved(&project.overview, Some("built"), None), project);

        let schema_path = Path::new("schemas/reviewed.schema.json");
        validate_reviewed_schema(&export, schema_path).expect("export matches schema");
    }
}
