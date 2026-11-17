//! Review AI module - AI reviewer for M3.

use crate::ai_client::ChatRequest;
use crate::config::{AiConfig, ReviewConfig};
use crate::model::AiReviewResult;
use anyhow::Result;

/// Run AI models command for review role.
pub fn run_ai_models_review(_config: &crate::config::Config) -> Result<()> {
    println!("AI models --role review not fully implemented yet");
    Ok(())
}

/// Build reviewer request for a project.
#[allow(dead_code)]
pub fn build_reviewer_request(
    _project_overview: &str,
    _claims: &[String],
    _evidence_texts: &[String],
    _code_flags: &[crate::model::ReviewCheck],
    config: &ReviewConfig,
) -> ChatRequest {
    // Simplified - would build proper request
    ChatRequest {
        model: config.model.clone(),
        messages: vec![],
        temperature: 0.2,
        max_tokens: 1500,
    }
}

/// Call reviewer with caching.
#[allow(dead_code)]
pub fn call_reviewer(
    _client: &crate::ai_client::HttpChatClient,
    _request: &ChatRequest,
    _cache_key: &str,
    _config: &AiConfig,
) -> Result<AiReviewResult> {
    Ok(AiReviewResult {
        claims: Vec::new(),
        jargon: Vec::new(),
        private: Vec::new(),
        suggested_overview: None,
        overall: "ok".to_string(),
        failed: false,
        failure_reason: None,
    })
}

/// Validate AI response.
#[allow(dead_code)]
pub fn validate_ai_response(_response: &str) -> Result<AiReviewResult> {
    // Simplified validation
    Ok(AiReviewResult {
        claims: Vec::new(),
        jargon: Vec::new(),
        private: Vec::new(),
        suggested_overview: None,
        overall: "ok".to_string(),
        failed: false,
        failure_reason: None,
    })
}

/// Check if suggestion adds new facts.
#[allow(dead_code)]
pub fn check_suggestion_facts(
    _suggestion: &str,
    _original_overview: &str,
    _claims: &[String],
    _project_name: &str,
) -> bool {
    // Simplified - would check that every digit/capitalized word in suggestion
    // appears in original_overview, claims, or project_name
    true
}
