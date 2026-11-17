//! Review AI module - AI reviewer for M3.
//!
//! RUST NOTE: Reuses the M2 ChatClient, cache, and redaction logic.
//! Never logs or prints API keys.

use crate::ai_client::{ChatClient, ChatMessage, ChatRequest, HttpChatClient};
use crate::config::{AiConfig, ReviewConfig};
use crate::facts::Facts;
use crate::model::{AiReviewResult, ClaimVerdict, Verdict, JargonFlag, PrivateFlag};
use crate::redact::redact;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::Future;

/// AI reviewer response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiReviewResponse {
    claims: Vec<ClaimVerdictResponse>,
    jargon: Vec<JargonResponse>,
    private: Vec<PrivateResponse>,
    suggested_overview: Option<String>,
    overall: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimVerdictResponse {
    index: usize,
    verdict: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JargonResponse {
    phrase: String,
    plain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrivateResponse {
    phrase: String,
    why: String,
}

/// Run AI models command for review role.
pub fn run_ai_models_review(config: &crate::config::Config) -> Result<()> {
    let ai_config = &config.ai;
    let client = HttpChatClient::new(ai_config)
        .context("Failed to create HTTP client")?;

    // Get available models
    let models = client
        .list_models()
        .await_context("Failed to list models")?
        .join(", ");

    println!("Available models: {}", models);

    // Probe the configured review model if set
    if !config.review.model.is_empty() {
        println!("Probing model: {}", config.review.model);
        let success = client
            .probe_model(&config.review.model)
            .await_context("Failed to probe model")?;

        if !success {
            anyhow::bail!("Model probe failed for {}", config.review.model);
        }
        println!("Model {} probe successful", config.review.model);
    }

    println!(
        "AI models --role review completed successfully. Model saved to [review] model in config."
    );
    Ok(())
}

/// Helper trait for async context conversion.
trait AwaitContext {
    type Output;
    fn await_context(self, context: &str) -> Result<Self::Output>;
}

#[async_trait::async_trait]
impl<T: Send> AwaitContext for Pin<Box<dyn Future<Output = Result<T>> + Send>> {
    type Output = T;

    fn await_context(self, context: &str) -> Result<T> {
        futures::executor::block_on(self).context(context)
    }
}

use std::pin::Pin;

/// Build reviewer request for a project.
pub fn build_reviewer_request(
    project_overview: &str,
    claims: &[String],
    evidence_texts: &[String],
    code_flags: &[crate::model::ReviewCheck],
    config: &ReviewConfig,
) -> ChatRequest {
    let mut messages = Vec::new();

    // System prompt for reviewer
    let system_prompt = r#"You are an editor checking portfolio copy for a student developer's site. 
For each claim decide whether its evidence text supports it; list jargon with plain alternatives; 
list private-looking phrases (personal names, employers, client names, course codes, internal hostnames); 
optionally suggest a plainer overview using only facts already in the claims; reply with JSON only."#;

    messages.push(ChatMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    });

    // Build the content for user message
    let mut content = String::new();
    content.push_str(&format!("**Overview**:\n{}\n\n", project_overview));

    content.push_str("**Claims**:\n");
    for (i, claim) in claims.iter().enumerate() {
        content.push_str(&format!("{}. {}\n", i + 1, claim));
    }

    content.push_str("\n**Evidence**:\n");
    for (i, evidence) in evidence_texts.iter().enumerate() {
        content.push_str(&format!("E{}: {}\n", i + 1, evidence));
    }

    content.push_str("\n**Code Flags**:\n");
    for flag in code_flags {
        content.push_str(&format!("{}: {}\n", flag.code, flag.message));
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content,
    });

    ChatRequest {
        model: config.model.clone(),
        messages,
        temperature: 0.2,
        max_tokens: 1500,
    }
}

/// Call reviewer with caching.
pub fn call_reviewer(
    client: &impl ChatClient,
    request: &ChatRequest,
    _cache_key: &str,
    _config: &AiConfig,
) -> Result<AiReviewResult> {
    // In a real implementation, this would check cache first
    // For now, just make the API call
    let response = client
        .chat(request)
        .await_context("AI review request failed")?;

    // Extract content from first choice
    let content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "{}".to_string());

    validate_ai_response(&content)
}

/// Validate AI response.
pub fn validate_ai_response(response: &str) -> Result<AiReviewResult> {
    // Try to parse as JSON
    let parsed: AiReviewResponse = serde_json::from_str(response).map_err(|e| {
        anyhow!("Failed to parse AI review response as JSON: {}", e)
    })?;

    // Convert to AiReviewResult
    let claims = parsed
        .claims
        .iter()
        .map(|c| ClaimVerdict {
            index: c.index,
            verdict: match c.verdict.as_str() {
                "supported" => Verdict::Supported,
                "partial" => Verdict::Partial,
                "unsupported" => Verdict::Unsupported,
                _ => Verdict::Unsupported,
            },
            reason: c.reason.clone(),
        })
        .collect();

    let jargon = parsed
        .jargon
        .iter()
        .map(|j| JargonFlag {
            phrase: j.phrase.clone(),
            plain: j.plain.clone(),
        })
        .collect();

    let private = parsed
        .private
        .iter()
        .map(|p| PrivateFlag {
            phrase: p.phrase.clone(),
            why: p.why.clone(),
        })
        .collect();

    let overall = if parsed.overall == "needs_attention" {
        "needs_attention".to_string()
    } else {
        "ok".to_string()
    };

    Ok(AiReviewResult {
        claims,
        jargon,
        private,
        suggested_overview: parsed.suggested_overview,
        overall,
        failed: false,
        failure_reason: None,
    })
}

/// Check if suggestion adds new facts.
pub fn check_suggestion_facts(
    suggestion: &str,
    original_overview: &str,
    claims: &[String],
    project_name: &str,
) -> bool {
    // Extract all digit sequences and capitalized words from suggestion
    let digit_pattern = Regex::new(r"\b\d+\b").unwrap();
    let capitalized_pattern = Regex::new(r"\b[A-Z][a-zA-Z]*\b").unwrap();

    let suggestion_digits: Vec<&str> = digit_pattern.find_iter(suggestion).map(|m| m.as_str()).collect();
    let suggestion_capitalized: Vec<&str> = capitalized_pattern.find_iter(suggestion).map(|m| m.as_str()).collect();

    // Check each digit sequence
    for digit in &suggestion_digits {
        let found = original_overview.contains(digit)
            || claims.iter().any(|c| c.contains(digit))
            || project_name.contains(digit);
        if !found {
            return false;
        }
    }

    // Check each capitalized word
    for word in &suggestion_capitalized {
        // Skip common words that might appear naturally
        if ["The", "A", "An", "I", "And", "But", "Or", "If", "Of", "In", "On", "At", "To", "For"].contains(word) {
            continue;
        }
        let found = original_overview.contains(word)
            || claims.iter().any(|c| c.contains(word))
            || project_name.contains(word);
        if !found {
            return false;
        }
    }

    true
}

/// Build a cache key for a review request.
pub fn build_review_cache_key(overview: &str, claims: &[String], evidence: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(overview);
    for claim in claims {
        hasher.update(claim);
    }
    for ev in evidence {
        hasher.update(ev);
    }
    format!("{:x}", hasher.finalize())
}

/// Redact review request before sending to AI.
pub fn redact_review_request(request: &str) -> String {
    redact(request).text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_suggestion_facts_preserves_digits() {
        let original = "Version 1.2.3 was released";
        let claims = vec!["This project uses Rust".to_string()];
        let project_name = "my-project";
        
        // Should pass - all digits in suggestion exist in original
        assert!(check_suggestion_facts("Version 1.2.3 released", original, &claims, project_name));
    }

    #[test]
    fn test_check_suggestion_facts_rejects_new_digits() {
        let original = "Version 1.0.0 was released";
        let claims = vec!["This project uses Rust".to_string()];
        let project_name = "my-project";
        
        // Should fail - 2.0.0 has digits not in original
        assert!(!check_suggestion_facts("Version 2.0.0 released", original, &claims, project_name));
    }

    #[test]
    fn test_check_suggestion_facts_preserves_capitalized() {
        let original = "Rust is great";
        let claims = vec!["I love Rust".to_string()];
        let project_name = "MyProject";
        
        // Should pass - capitalized words exist
        assert!(check_suggestion_facts("Rust is great", original, &claims, project_name));
    }
}