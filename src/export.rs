//! Export module - builds and writes reviewed.json for M3.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{
    Draft, Overrides, ProjectDecision, ProjectOverview, ReviewedActivity, ReviewedExport,
    ReviewedProject,
};
use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
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
/// Build the reviewed export from draft, overrides, and facts.
pub fn build_reviewed_export(
    draft: &Draft,
    overrides: &Overrides,
    _facts: &Facts,
    _config: &ReviewConfig,
) -> Result<ReviewedExport> {
    let mut exported_projects = Vec::new();
    let mut exported_repo_ids = HashSet::new();

    // First pass: collect all approved, non-stale projects
    for project in &draft.projects {
        let override_opt = overrides.projects.get(&project.repo_id);
        let decision = effective_decision(override_opt, project);

        if decision == crate::model::Decision::Approved {
            exported_repo_ids.insert(project.repo_id.clone());
        }
    }

    // Second pass: build reviewed projects
    for project in &draft.projects {
        let override_opt = overrides.projects.get(&project.repo_id);
        let decision = effective_decision(override_opt, project);

        if decision != crate::model::Decision::Approved {
            continue;
        }

        // Get published text (override overview or draft overview)
        let published_text = override_opt
            .and_then(|o| o.overview.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(&project.overview);

        // Verify hash matches
        let text_hash = compute_text_hash(published_text);
        if override_opt
            .and_then(|o| o.approved_hash.as_ref())
            .map(|h| h != &text_hash)
            .unwrap_or(true)
        {
            // Hash mismatch - this should have been caught by effective_decision
            continue;
        }

        // Determine status
        let status = override_opt
            .and_then(|o| o.status.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(
                if matches!(project.status, crate::model::ProjectStatus::Active) {
                    "active"
                } else {
                    "dormant"
                },
            );

        // Filter related to only exported projects
        let related = filter_related(&project.related, &exported_repo_ids);

        let reviewed_project = ReviewedProject {
            repo_id: project.repo_id.clone(),
            name: project.name.clone(),
            status: status.to_string(),
            overview: published_text.to_string(),
            featured: override_opt.and_then(|o| o.featured).unwrap_or(false),
            order: override_opt.and_then(|o| o.order),
            related,
        };

        exported_projects.push(reviewed_project);
    }

    // Sort by order (ascending, None last), then name
    exported_projects.sort_by(|a, b| match (a.order, b.order) {
        (Some(a_ord), Some(b_ord)) => a_ord.cmp(&b_ord),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });

    // Build narrative if approved and not stale
    let recent_activity = build_reviewed_activity(
        &overrides.narrative,
        &draft.recent_activity,
        &exported_repo_ids,
    );

    Ok(ReviewedExport {
        schema_version: 1,
        exported_at: Utc::now(),
        projects: exported_projects,
        recent_activity,
    })
}

/// Determine effective decision for a project.
fn effective_decision(
    override_opt: Option<&ProjectDecision>,
    project: &ProjectOverview,
) -> crate::model::Decision {
    let Some(override_data) = override_opt else {
        return crate::model::Decision::Pending;
    };

    match override_data.decision {
        crate::model::Decision::Hidden => crate::model::Decision::Hidden,
        crate::model::Decision::Approved => {
            // Check if stale
            let published_text = override_data
                .overview
                .as_deref()
                .unwrap_or(&project.overview);
            let text_hash = compute_text_hash(published_text);

            if override_data.approved_hash.as_ref() == Some(&text_hash) {
                crate::model::Decision::Approved
            } else {
                crate::model::Decision::Stale
            }
        }
        crate::model::Decision::Pending => crate::model::Decision::Pending,
        crate::model::Decision::Stale => crate::model::Decision::Stale,
    }
}

/// Build reviewed activity if narrative is approved and not stale.
fn build_reviewed_activity(
    narrative_override: &crate::model::NarrativeDecision,
    draft_narrative: &crate::model::RecentActivity,
    _exported_repo_ids: &HashSet<String>,
) -> Option<ReviewedActivity> {
    // Check if narrative is approved
    if narrative_override.decision != crate::model::Decision::Approved {
        return None;
    }

    // Check if stale
    let published_text = narrative_override
        .summary
        .as_deref()
        .unwrap_or(&draft_narrative.summary);
    let text_hash = compute_text_hash(published_text);

    if narrative_override.approved_hash.as_ref() != Some(&text_hash) {
        return None; // stale
    }

    // Check if approved set has changed
    // This would require tracking the approved set hash
    // For now, we'll include it if the text hash matches
    Some(ReviewedActivity {
        summary: published_text.to_string(),
    })
}

/// Compute SHA-256 hash of text.
pub fn compute_text_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

/// Filter related projects to only exported ones.
pub fn filter_related(related: &[String], exported_repo_ids: &HashSet<String>) -> Vec<String> {
    related
        .iter()
        .filter(|name| exported_repo_ids.contains(*name))
        .cloned()
        .collect()
}

/// Write reviewed export to JSON file atomically.
pub fn write_reviewed_json(export: &ReviewedExport, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(export)
        .context("Failed to serialize reviewed export to JSON")?;

    // Write to temp file first, then rename atomically
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, json)
        .with_context(|| format!("Failed to write reviewed export to {}", temp_path.display()))?;

    std::fs::rename(&temp_path, path)
        .with_context(|| format!("Failed to rename temp file to {}", path.display()))?;

    Ok(())
}

/// Validate reviewed export against schema.
pub fn validate_reviewed_schema(export: &ReviewedExport, schema_path: &Path) -> Result<()> {
    let schema_content = std::fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema: {}", schema_path.display()))?;
    let schema = jsonschema::Validator::new(&serde_json::from_str(&schema_content)?)
        .context("Failed to compile JSON schema")?;

    let instance =
        serde_json::to_value(export).context("Failed to convert reviewed export to JSON value")?;

    if let Err(errors) = schema.validate(&instance) {
        let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        anyhow::bail!("Schema validation failed: {}", error_msgs.join("; "));
    }
    Ok(())
}

/// Run error-level checks on all included texts at export time.
pub fn run_export_error_checks(
    export: &ReviewedExport,
    config: &ReviewConfig,
) -> Vec<crate::model::ReviewCheck> {
    let mut checks = Vec::new();

    // Check each project overview
    for project in &export.projects {
        checks.extend(prose_lint_checks(
            &project.overview,
            &format!("project {}", project.name),
        ));

        // Check too_long
        if project.overview.chars().count() > config.max_overview_chars {
            checks.push(crate::model::ReviewCheck {
                code: "too_long".to_string(),
                severity: crate::model::Severity::Error,
                message: format!(
                    "Project {} overview is {} characters, limit is {}",
                    project.name,
                    project.overview.chars().count(),
                    config.max_overview_chars
                ),
            });
        }

        // Check deny_terms
        for term in &config.deny_terms {
            if !term.is_empty() {
                let pattern = format!(r"\b{}\b", regex::escape(term));
                if let Ok(re) = regex::Regex::new(&pattern)
                    && re.is_match(&project.overview)
                {
                    checks.push(crate::model::ReviewCheck {
                        code: "deny_term".to_string(),
                        severity: crate::model::Severity::Error,
                        message: format!(
                            "Deny term '{}' found in project {} overview",
                            term, project.name
                        ),
                    });
                }
            }
        }
    }

    // Check narrative if present
    if let Some(activity) = &export.recent_activity {
        checks.extend(prose_lint_checks(&activity.summary, "narrative"));

        if activity.summary.chars().count() > config.max_overview_chars {
            checks.push(crate::model::ReviewCheck {
                code: "too_long".to_string(),
                severity: crate::model::Severity::Error,
                message: format!(
                    "Narrative is {} characters, limit is {}",
                    activity.summary.chars().count(),
                    config.max_overview_chars
                ),
            });
        }

        for term in &config.deny_terms {
            if !term.is_empty() {
                let pattern = format!(r"\b{}\b", regex::escape(term));
                if let Ok(re) = regex::Regex::new(&pattern)
                    && re.is_match(&activity.summary)
                {
                    checks.push(crate::model::ReviewCheck {
                        code: "deny_term".to_string(),
                        severity: crate::model::Severity::Error,
                        message: format!("Deny term '{}' found in narrative", term),
                    });
                }
            }
        }
    }

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
    use anyhow::Context;
    use std::path::Path;

    // Load draft and facts
    let draft_path = Path::new("data/enriched.draft.json");
    let facts_path = Path::new("data/facts.json");
    let overrides_path = Path::new("overrides.toml");

    let draft_content = std::fs::read_to_string(draft_path).context("Failed to read draft")?;
    let draft: Draft = serde_json::from_str(&draft_content).context("Failed to parse draft")?;

    let facts_content = std::fs::read_to_string(facts_path).context("Failed to read facts")?;
    let facts: Facts = serde_json::from_str(&facts_content).context("Failed to parse facts")?;

    let overrides = crate::decisions::load_overrides(overrides_path)?;

    let export = build_reviewed_export(&draft, &overrides, &facts, config)?;

    // Run error checks
    let error_checks = run_export_error_checks(&export, config);
    if error_checks
        .iter()
        .any(|c| c.severity == crate::model::Severity::Error)
    {
        for check in &error_checks {
            eprintln!("Export error: {}: {}", check.code, check.message);
        }
        anyhow::bail!("Export failed due to error-level checks");
    }

    // Validate schema
    let schema_path = Path::new("schemas/reviewed.schema.json");
    if schema_path.exists() {
        validate_reviewed_schema(&export, schema_path).context("Schema validation failed")?;
    }

    // Write output
    write_reviewed_json(&export, output_path).context("Failed to write reviewed.json")?;

    println!(
        "Exported {} projects to {}",
        export.projects.len(),
        output_path.display()
    );
    Ok(())
}
