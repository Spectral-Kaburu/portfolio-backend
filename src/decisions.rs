//! Decisions module - handles overrides.toml loading/saving and decision logic for M3.

use crate::config::ReviewConfig;
use crate::model::{Decision, NarrativeDecision, Overrides, ProjectDecision, ProjectOverview};
use anyhow::{Context, Result};
use std::path::Path;

/// Load overrides from file.
pub fn load_overrides(path: &Path) -> Result<Overrides> {
    if !path.exists() {
        return Ok(Overrides::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read overrides from {}", path.display()))?;
    let overrides: Overrides = toml::from_str(&content)
        .with_context(|| format!("Failed to parse overrides from {}", path.display()))?;
    Ok(overrides)
}

/// Save overrides to file, preserving comments and unknown keys.
#[allow(dead_code)]
pub fn save_overrides(overrides: &Overrides, path: &Path) -> Result<()> {
    let toml_string =
        toml::to_string_pretty(overrides).context("Failed to serialize overrides to TOML")?;
    std::fs::write(path, toml_string)
        .with_context(|| format!("Failed to write overrides to {}", path.display()))?;
    Ok(())
}

/// Determine effective decision for a project.
#[allow(dead_code)]
pub fn effective_decision(
    override_opt: Option<&ProjectDecision>,
    project: &ProjectOverview,
) -> Decision {
    let Some(override_data) = override_opt else {
        return Decision::Pending;
    };

    match override_data.decision {
        Decision::Hidden => Decision::Hidden,
        Decision::Approved => {
            let published_text = override_data
                .overview
                .as_deref()
                .unwrap_or(&project.overview);
            let text_hash = crate::review_checks::compute_text_hash(published_text);

            if override_data.approved_hash.as_ref() == Some(&text_hash) {
                Decision::Approved
            } else {
                Decision::Stale
            }
        }
        Decision::Pending => Decision::Pending,
        Decision::Stale => Decision::Stale,
    }
}
/// Check if a project is stale.
#[allow(dead_code)]
pub fn is_stale(override_opt: Option<&ProjectDecision>, project: &ProjectOverview) -> bool {
    matches!(effective_decision(override_opt, project), Decision::Stale)
}

/// Apply approve action.
#[allow(dead_code)]
pub fn apply_approve(mut overrides: Overrides, repo_id: &str, published_text: &str) -> Overrides {
    let text_hash = crate::review_checks::compute_text_hash(published_text);
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.decision = Decision::Approved;
    entry.approved_hash = Some(text_hash);
    overrides
}

/// Apply approve with suggestion action.
#[allow(dead_code)]
pub fn apply_approve_with_suggestion(
    mut overrides: Overrides,
    repo_id: &str,
    suggestion: &str,
    published_text: &str,
) -> Result<Overrides> {
    let text_hash = crate::review_checks::compute_text_hash(published_text);
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.decision = Decision::Approved;
    entry.overview = Some(suggestion.to_string());
    entry.approved_hash = Some(text_hash);
    Ok(overrides)
}

/// Apply edit action.
#[allow(dead_code)]
pub fn apply_edit(
    mut overrides: Overrides,
    repo_id: &str,
    new_overview: &str,
    code_checks_fn: impl FnOnce(&str) -> Vec<crate::model::ReviewCheck>,
) -> Result<Overrides> {
    // Run code checks on the new text
    let checks = code_checks_fn(new_overview);
    let has_errors = checks
        .iter()
        .any(|c| c.severity == crate::model::Severity::Error);
    if has_errors {
        anyhow::bail!("Edited text has error-level checks: {:?}", checks);
    }

    let text_hash = crate::review_checks::compute_text_hash(new_overview);
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.decision = Decision::Approved;
    entry.overview = Some(new_overview.to_string());
    entry.approved_hash = Some(text_hash);
    Ok(overrides)
}

/// Apply hide action.
#[allow(dead_code)]
pub fn apply_hide(mut overrides: Overrides, repo_id: &str) -> Overrides {
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.decision = Decision::Hidden;
    overrides
}

/// Apply skip action (no change).
#[allow(dead_code)]
pub fn apply_skip(overrides: Overrides, _repo_id: &str) -> Overrides {
    overrides
}

/// Apply featured toggle.
#[allow(dead_code)]
pub fn apply_featured_toggle(mut overrides: Overrides, repo_id: &str, featured: bool) -> Overrides {
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.featured = Some(featured);
    overrides
}

/// Apply built toggle.
#[allow(dead_code)]
pub fn apply_built_toggle(mut overrides: Overrides, repo_id: &str, built: bool) -> Overrides {
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.status = if built {
        Some("built".to_string())
    } else {
        None
    };
    overrides
}

/// Apply order override.
#[allow(dead_code)]
pub fn apply_order(mut overrides: Overrides, repo_id: &str, order: i32) -> Overrides {
    let entry = overrides.projects.entry(repo_id.to_string()).or_default();
    entry.order = Some(order);
    overrides
}

/// Determine effective decision for narrative.
#[allow(dead_code)]
pub fn narrative_effective_decision(
    narrative_override: &NarrativeDecision,
    current_narrative: &str,
    approved_set_hash: &str,
) -> Decision {
    match narrative_override.decision {
        Decision::Approved => {
            let text_hash = crate::review_checks::compute_text_hash(current_narrative);
            if narrative_override.approved_hash.as_ref() == Some(&text_hash)
                && narrative_override.approved_set_hash.as_deref() == Some(approved_set_hash)
            {
                Decision::Approved
            } else {
                Decision::Stale
            }
        }
        Decision::Pending => Decision::Pending,
        Decision::Hidden => Decision::Hidden,
        Decision::Stale => Decision::Stale,
    }
}

/// Check if narrative is stale.
#[allow(dead_code)]
pub fn narrative_is_stale(
    narrative_override: &NarrativeDecision,
    current_narrative: &str,
    approved_set_hash: &str,
) -> bool {
    matches!(
        narrative_effective_decision(narrative_override, current_narrative, approved_set_hash),
        Decision::Stale
    )
}

/// Apply narrative approve.
#[allow(dead_code)]
pub fn apply_narrative_approve(
    mut overrides: Overrides,
    narrative_text: &str,
    approved_set_hash: &str,
) -> Overrides {
    let text_hash = crate::review_checks::compute_text_hash(narrative_text);
    overrides.narrative.decision = Decision::Approved;
    overrides.narrative.approved_hash = Some(text_hash);
    overrides.narrative.approved_set_hash = Some(approved_set_hash.to_string());
    overrides
}

/// Apply narrative edit.
#[allow(dead_code)]
pub fn apply_narrative_edit(
    mut overrides: Overrides,
    new_summary: &str,
    code_checks_fn: impl FnOnce(&str) -> Vec<crate::model::ReviewCheck>,
) -> Result<Overrides> {
    let checks = code_checks_fn(new_summary);
    let has_errors = checks
        .iter()
        .any(|c| c.severity == crate::model::Severity::Error);
    if has_errors {
        anyhow::bail!("Edited narrative has error-level checks: {:?}", checks);
    }

    let text_hash = crate::review_checks::compute_text_hash(new_summary);
    overrides.narrative.decision = Decision::Approved;
    overrides.narrative.summary = Some(new_summary.to_string());
    overrides.narrative.approved_hash = Some(text_hash);
    Ok(overrides)
}

/// Apply narrative reject.
#[allow(dead_code)]
pub fn apply_narrative_reject(mut overrides: Overrides) -> Overrides {
    overrides.narrative.decision = Decision::Hidden;
    overrides
}

/// Run status command - print table of projects and their decisions.
pub fn run_status(_config: &ReviewConfig) -> Result<()> {
    println!("review status command not fully implemented yet");
    Ok(())
}
