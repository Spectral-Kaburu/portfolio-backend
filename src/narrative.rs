//! Narrative module - generates and reviews recent activity narrative for M3.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{NarrativeResult, ProjectOverview};
use anyhow::Result;

/// Run narrative review command.
pub fn run_narrative_review(_config: &ReviewConfig, _dry_run: bool) -> Result<()> {
    println!("Narrative review not fully implemented yet");
    Ok(())
}

/// Generate narrative from approved active projects.
#[allow(dead_code)]
pub fn generate_narrative(
    approved_projects: &[&ProjectOverview],
    facts: &Facts,
    _config: &ReviewConfig,
) -> Result<NarrativeResult> {
    // Collect published texts and commit counts
    let mut project_texts = Vec::new();
    let mut project_commit_counts = Vec::new();

    for project in approved_projects {
        project_texts.push(project.overview.clone());
        // Find commit count from facts
        if let Some(fact_project) = facts.projects.iter().find(|p| p.repo_id == project.repo_id) {
            project_commit_counts.push(fact_project.recent_commits.len() as u32);
        } else {
            project_commit_counts.push(0);
        }
    }

    // Build prompt (simplified - real implementation would use M2's narrative prompt)
    let summary = if project_texts.is_empty() {
        "No approved active projects to summarize.".to_string()
    } else {
        format!("Recent activity across {} projects.", project_texts.len())
    };

    Ok(NarrativeResult {
        summary,
        claims: Vec::new(),
        failure: None,
    })
}

/// Build narrative reviewer request.
#[allow(dead_code)]
pub fn build_narrative_reviewer_request(narrative: &str, approved_overviews: &[String]) -> String {
    // Return JSON request body for AI reviewer
    format!(
        r#"{{"narrative": "{}", "evidence": {}}}"#,
        narrative.replace('"', "\\\""),
        serde_json::to_string(approved_overviews).unwrap_or_default()
    )
}

/// Validate AI narrative response.
#[allow(dead_code)]
pub fn validate_narrative_response(response: &str) -> Result<NarrativeResult> {
    // Simplified validation
    Ok(NarrativeResult {
        summary: response.to_string(),
        claims: Vec::new(),
        failure: None,
    })
}
