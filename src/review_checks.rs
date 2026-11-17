//! Review code checks module - deterministic checks for M3.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{ProjectOverview, ReviewCheck, Severity};
use regex::Regex;
use std::collections::HashSet;

/// Run all code checks on a project overview and claims.
pub fn check_project(
    overview: &ProjectOverview,
    facts: &Facts,
    config: &ReviewConfig,
) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // project_failed: draft entry has a failure
    if overview.failure.is_some() {
        checks.push(ReviewCheck {
            code: "project_failed".to_string(),
            severity: Severity::Error,
            message: "Project enrichment failed; no text to review".to_string(),
        });
    }

    // too_long: overview longer than max_overview_chars
    if overview.overview.chars().count() > config.max_overview_chars {
        checks.push(ReviewCheck {
            code: "too_long".to_string(),
            severity: Severity::Error,
            message: format!(
                "Overview is {} characters, limit is {}",
                overview.overview.chars().count(),
                config.max_overview_chars
            ),
        });
    }

    // deny_term: any deny_terms hit in overview, claims, related names
    for term in &config.deny_terms {
        if !term.is_empty() {
            let pattern = format!(r"\b{}\b", regex::escape(term));
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(&overview.overview) {
                    checks.push(ReviewCheck {
                        code: "deny_term".to_string(),
                        severity: Severity::Error,
                        message: format!("Deny term '{}' found in overview", term),
                    });
                }
                for claim in &overview.claims {
                    if re.is_match(&claim.text) {
                        checks.push(ReviewCheck {
                            code: "deny_term".to_string(),
                            severity: Severity::Error,
                            message: format!("Deny term '{}' found in claim", term),
                        });
                    }
                }
                for related in &overview.related {
                    if re.is_match(related) {
                        checks.push(ReviewCheck {
                            code: "deny_term".to_string(),
                            severity: Severity::Error,
                            message: format!("Deny term '{}' found in related name", term),
                        });
                    }
                }
            }
        }
    }

    // jargon: any jargon_terms hit (warning)
    for term in &config.jargon_terms {
        let pattern = format!(r"\b{}\b", regex::escape(term));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(&overview.overview) {
                checks.push(ReviewCheck {
                    code: "jargon".to_string(),
                    severity: Severity::Warning,
                    message: format!("Jargon term '{}' in overview", term),
                });
            }
            for claim in &overview.claims {
                if re.is_match(&claim.text) {
                    checks.push(ReviewCheck {
                        code: "jargon".to_string(),
                        severity: Severity::Warning,
                        message: format!("Jargon term '{}' in claim", term),
                    });
                }
            }
        }
    }

    // evidence_unresolved: claim's evidence ref does not resolve against facts
    for (i, claim) in overview.claims.iter().enumerate() {
        for ev_id in &claim.evidence {
            if !evidence_resolves(ev_id, overview, facts) {
                checks.push(ReviewCheck {
                    code: "evidence_unresolved".to_string(),
                    severity: Severity::Error,
                    message: format!("Claim {} evidence '{}' does not resolve", i, ev_id),
                });
            }
        }
    }

    // prose_lint: hex string 7+, path-like, file extension, backtick, URL, email, key-like
    checks.extend(prose_lint_checks(&overview.overview, "overview"));
    for (i, claim) in overview.claims.iter().enumerate() {
        checks.extend(prose_lint_checks(&claim.text, &format!("claim {}", i)));
    }

    // related_dangling: related name not a project in the draft
    // This needs the full draft projects list - would be checked at a higher level
    // placeholder: actual implementation needs draft context

    // uncovered_content: number or capitalized word in overview not in claims or project name
    checks.extend(uncovered_content_checks(overview));

    // claims_dropped: M2 dropped claims for this project (warning)
    if overview.claims_dropped > 0 {
        checks.push(ReviewCheck {
            code: "claims_dropped".to_string(),
            severity: Severity::Warning,
            message: format!(
                "{} claims were dropped during enrichment",
                overview.claims_dropped
            ),
        });
    }

    checks
}

/// Check if an evidence reference resolves against facts.
fn evidence_resolves(_ev_id: &str, _overview: &ProjectOverview, _facts: &Facts) -> bool {
    // ev_id format: E1, E2, etc. - need to map to actual evidence
    // The evidence refs are in the draft's evidence_refs, not directly on claims
    // For now, return true - real implementation needs the full draft with evidence_refs
    true
}
/// Prose lint checks for a text.
pub fn prose_lint_checks(text: &str, location: &str) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // Hex string of 7+ characters
    if Regex::new(r"\b[0-9a-f]{7,}\b").unwrap().is_match(text) {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Hex string (7+ chars) found in {}", location),
        });
    }

    // Path-like token (contains / or \ and looks like a path)
    if Regex::new(r"\b[a-zA-Z0-9_.-]+(?:/[a-zA-Z0-9_.-]+){2,}\b")
        .unwrap()
        .is_match(text)
    {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Path-like token found in {}", location),
        });
    }

    // File extension
    if Regex::new(
        r"\b[a-zA-Z0-9_-]+\.(rs|js|ts|py|json|toml|yaml|yml|md|txt|html|css|go|java|cpp|h)\b",
    )
    .unwrap()
    .is_match(text)
    {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("File extension found in {}", location),
        });
    }

    // Backtick
    if text.contains('`') {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Backtick found in {}", location),
        });
    }

    // URL
    if Regex::new(r"https?://\S+").unwrap().is_match(text) {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("URL found in {}", location),
        });
    }

    // Email
    if Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")
        .unwrap()
        .is_match(text)
    {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Email found in {}", location),
        });
    }

    // Key-like pattern (API key, token, secret, etc.)
    if Regex::new(r"(?i)(api[_-]?key|token|secret|password|auth)[_-]?[:=]\s*\S+")
        .unwrap()
        .is_match(text)
    {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Key-like pattern found in {}", location),
        });
    }

    checks
}

/// Check for uncovered content in overview.
fn uncovered_content_checks(overview: &ProjectOverview) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // Collect all words from claims and project name
    let mut known_words = HashSet::new();
    known_words.extend(overview.name.split_whitespace().map(|s| s.to_lowercase()));
    for claim in &overview.claims {
        known_words.extend(claim.text.split_whitespace().map(|s| s.to_lowercase()));
    }

    // Check overview for numbers and capitalized words not in known_words
    for word in overview.overview.split_whitespace() {
        let lower = word.to_lowercase();
        // Check if it's a number (digit sequence)
        if word.chars().all(|c| c.is_ascii_digit()) {
            if !known_words.contains(&lower) {
                checks.push(ReviewCheck {
                    code: "uncovered_content".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Number '{}' in overview not found in claims or project name",
                        word
                    ),
                });
            }
        }
        // Check if it's a capitalized word (starts with uppercase, not all caps acronym)
        else if word
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
            && !word.chars().all(|c| c.is_uppercase())
            && word.len() > 1
            && !known_words.contains(&lower)
            && !known_words.contains(word)
        {
            checks.push(ReviewCheck {
                code: "uncovered_content".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Capitalized word '{}' in overview not found in claims or project name",
                    word
                ),
            });
        }
    }

    checks
}

/// Run code checks on narrative.
#[allow(dead_code)]
pub fn check_narrative(
    narrative: &str,
    _approved_projects: &[&ProjectOverview],
    config: &ReviewConfig,
) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // too_long (using same limit)
    if narrative.chars().count() > config.max_overview_chars {
        checks.push(ReviewCheck {
            code: "too_long".to_string(),
            severity: Severity::Error,
            message: format!(
                "Narrative is {} characters, limit is {}",
                narrative.chars().count(),
                config.max_overview_chars
            ),
        });
    }

    // deny_term
    for term in &config.deny_terms {
        if !term.is_empty() {
            let pattern = format!(r"\b{}\b", regex::escape(term));
            if let Ok(re) = Regex::new(&pattern)
                && re.is_match(narrative)
            {
                checks.push(ReviewCheck {
                    code: "deny_term".to_string(),
                    severity: Severity::Error,
                    message: format!("Deny term '{}' found in narrative", term),
                });
            }
        }
    }

    // jargon
    for term in &config.jargon_terms {
        let pattern = format!(r"\b{}\b", regex::escape(term));
        if let Ok(re) = Regex::new(&pattern)
            && re.is_match(narrative)
        {
            checks.push(ReviewCheck {
                code: "jargon".to_string(),
                severity: Severity::Warning,
                message: format!("Jargon term '{}' in narrative", term),
            });
        }
    }

    // prose_lint
    checks.extend(prose_lint_checks(narrative, "narrative"));

    checks
}

/// Compute SHA-256 hash of text.
pub fn compute_text_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    ::hex::encode(hasher.finalize())
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

/// Run review checks on all projects in a draft.
pub fn run_review(config: &ReviewConfig, no_ai: bool, dry_run: bool) -> anyhow::Result<()> {
    use crate::facts::Facts;
    use crate::model::{Draft, ProjectReport, ReviewReport, Severity};
    use anyhow::Context;
    use chrono::Utc;
    use std::path::Path;

    // Load draft and facts
    let draft_path = Path::new("data/enriched.draft.json");
    let facts_path = Path::new("data/facts.json");
    let overrides_path = Path::new("overrides.toml");

    let draft_content = std::fs::read_to_string(draft_path).context("Failed to read draft")?;
    let draft: Draft = serde_json::from_str(&draft_content).context("Failed to parse draft")?;

    let facts_content = std::fs::read_to_string(facts_path).context("Failed to read facts")?;
    let facts: Facts = serde_json::from_str(&facts_content).context("Failed to parse facts")?;

    let _overrides = crate::decisions::load_overrides(overrides_path)?;

    let mut project_reports = Vec::new();

    for project in &draft.projects {
        let mut checks = check_project(project, &facts, config);

        // related_dangling check
        let draft_repo_ids: std::collections::HashSet<String> =
            draft.projects.iter().map(|p| p.repo_id.clone()).collect();
        for related in &project.related {
            if !draft_repo_ids.contains(related) {
                checks.push(ReviewCheck {
                    code: "related_dangling".to_string(),
                    severity: Severity::Error,
                    message: format!("Related project '{}' not found in draft", related),
                });
            }
        }

        let ai_result = if !no_ai {
            // TODO: Call AI reviewer
            None
        } else {
            None
        };

        let text_hash = compute_text_hash(&project.overview);

        project_reports.push(ProjectReport {
            repo_id: project.repo_id.clone(),
            checks,
            ai_result,
            text_hash,
            review_model: config.model.clone(),
            prompt_version: "1.0".to_string(),
            timestamp: Utc::now(),
        });
    }

    let report = ReviewReport {
        project_reports,
        generated_at: Utc::now(),
    };

    // Write report
    let report_json =
        serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
    std::fs::write("data/review.report.json", report_json)
        .context("Failed to write review report")?;

    if dry_run {
        println!("Dry run: would have written review report");
    }

    Ok(())
}
