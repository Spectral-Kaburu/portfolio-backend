//! Narrative module - generates and reviews recent activity narrative for M3.
//!
//! RUST NOTE: Uses only approved projects' published texts.
//! Never adds facts not present in the original data.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{NarrativeResult, ProjectOverview};
use crate::review_ai::{check_suggestion_facts, validate_ai_response, AiReviewResponse};
use anyhow::{Context, Result};
use regex::Regex;

/// Run narrative review command.
pub fn run_narrative_review(config: &ReviewConfig, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("Narrative review dry-run: would generate narrative from approved active projects");
        return Ok(());
    }
    
    println!("Narrative review not fully implemented yet - requires approved projects input");
    Ok(())
}

/// Generate narrative from approved active projects.
pub fn generate_narrative(
    approved_projects: &[&ProjectOverview],
    facts: &Facts,
    _config: &ReviewConfig,
) -> Result<NarrativeResult> {
    // Collect published texts and commit counts
    let mut project_texts = Vec::new();
    let mut project_commit_counts = Vec::new();
    let mut repo_ids = Vec::new();

    for project in approved_projects {
        project_texts.push(project.overview.clone());
        repo_ids.push(project.repo_id.clone());
        
        // Find commit count from facts
        if let Some(fact_project) = facts.projects.iter().find(|p| p.repo_id == project.repo_id) {
            project_commit_counts.push(fact_project.recent_commits.len() as u32);
        } else {
            project_commit_counts.push(0);
        }
    }

    // Build prompt with project summaries
    let mut prompt_parts = Vec::new();
    prompt_parts.push("Generate a recent activity summary from these approved projects:\n".to_string());
    
    for (i, (text, count)) in project_texts.iter().zip(project_commit_counts.iter()).enumerate() {
        prompt_parts.push(format!("Project {}: {} ({} recent commits)", i + 1, text, count));
    }

    let prompt = format!(
        "Summarize the recent activity across all projects into a single coherent paragraph. Do not add any facts not present in the project texts above. Use the commit counts as context only.\n\n{}",
        prompt_parts.join("\n")
    );

    // For now, build a simple narrative (real implementation would call AI)
    let summary = if approved_projects.is_empty() {
        "No approved active projects to summarize.".to_string()
    } else {
        let names: Vec<&str> = approved_projects.iter().map(|p| p.name.as_str()).collect();
        format!(
            "Recent activity across {} approved projects: {}.",
            names.len(),
            names.join(", ")
        )
    };

    Ok(NarrativeResult {
        summary,
        claims: Vec::new(),
        failure: None,
    })
}

/// Build narrative reviewer request.
pub fn build_narrative_reviewer_request(narrative: &str, approved_overviews: &[String]) -> String {
    // Build JSON request body for AI reviewer
    let evidence_json = serde_json::to_string(approved_overviews).unwrap_or_default();
    
    // Use json! macro for proper escaping
    serde_json::json!({
        "narrative": narrative,
        "evidence": serde_json::from_str::<serde_json::Value>(&evidence_json).unwrap_or_default()
    }).to_string()
}

/// Validate AI narrative response.
pub fn validate_narrative_response(response: &str) -> Result<NarrativeResult> {
    // Try to parse as AI review response first
    let parsed: AiReviewResponse = serde_json::from_str(response)
        .map_err(|e| anyhow::eyre("Failed to parse narrative response: {}", e))?;
    
    // Extract summary - in real implementation this would be in the response
    let summary = parsed.overall;
    
    // Check if there's a suggested overview (which we use as the narrative)
    let final_summary = parsed.suggested_overview.unwrap_or_else(|| {
        "Narrative generation not fully implemented".to_string()
    });

    Ok(NarrativeResult {
        summary: final_summary,
        claims: vec![],
        failure: if parsed.failed { parsed.failure_reason } else { None },
    })
}

/// Check if narrative suggestion adds new facts.
pub fn check_narrative_facts(
    suggestion: &str,
    approved_texts: &[String],
) -> bool {
    let text_combined = approved_texts.join(" ");
    
    // Extract digit sequences
    let digit_pattern = Regex::new(r"\b\d+\b").unwrap();
    for m in digit_pattern.find_iter(suggestion) {
        if !text_combined.contains(m.as_str()) {
            return false;
        }
    }
    
    // Extract capitalized words (proper nouns, names, etc.)
    let capitalized_pattern = Regex::new(r"\b[A-Z][a-zA-Z]*\b").unwrap();
    for m in capitalized_pattern.find_iter(suggestion) {
        let word = m.as_str();
        // Skip common words
        if ["The", "A", "An", "I", "And", "But", "Or", "If", "Of", "In", "On", "At", "To", "For"].contains(word) {
            continue;
        }
        if !text_combined.contains(word) {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_narrative_facts_preserves_digits() {
        let approved = vec!["Version 1.2.3 was released".to_string()];
        
        assert!(check_narrative_facts("The project released version 1.2.3", &approved));
    }

    #[test]
    fn test_check_narrative_facts_rejects_new_digits() {
        let approved = vec!["Version 1.0.0 was released".to_string()];
        
        // 9.9.9 is not in approved text
        assert!(!check_narrative_facts("The project released version 9.9.9", &approved));
    }

    #[test]
    fn test_generate_narrative_empty_projects() {
        let facts = Facts::default();
        let projects: Vec<&ProjectOverview> = vec![];
        let config = ReviewConfig::default();
        
        let result = generate_narrative(&projects, &facts, &config).unwrap();
        assert!(result.summary.contains("No approved active projects"));
    }
}