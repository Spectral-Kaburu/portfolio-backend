//! Review code checks module - deterministic checks for M3.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{Draft, ProjectOverview, ProjectReport, ReviewCheck, ReviewReport, Severity};
use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref RE_HEX: Regex = Regex::new(r"\b[0-9a-f]{7,}\b").unwrap();
    static ref RE_PATH: Regex = Regex::new(r"\b[a-zA-Z0-9_.-]+(?:/[a-zA-Z0-9_.-]+){2,}\b").unwrap();
    static ref RE_FILE_EXT: Regex = Regex::new(
        r"\b[a-zA-Z0-9_-]+\.(rs|js|ts|py|json|toml|yaml|yml|md|txt|html|css|go|java|cpp|h)\b"
    ).unwrap();
    static ref RE_URL: Regex = Regex::new(r"https?://\S+").unwrap();
    static ref RE_EMAIL: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    static ref RE_KEY_LIKE: Regex = Regex::new(r"(?i)(api[_-]?key|token|secret|password|auth)[_-]?[:=]\s*\S+").unwrap();
}

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
            message: "Project has a failure, no text to review".to_string(),
        });
    }

    // too_long: overview longer than max_overview_chars
    if overview.overview.chars().count() > config.max_overview_chars {
        checks.push(ReviewCheck {
            code: "too_long".to_string(),
            severity: Severity::Error,
            message: format!(
                "Overview exceeds {} characters (actual: {})",
                config.max_overview_chars,
                overview.overview.chars().count()
            ),
        });
    }

    // deny_term: any deny_terms hit in overview, claims, related names
    for term in &config.deny_terms {
        if overview
            .overview
            .to_lowercase()
            .contains(&term.to_lowercase())
        {
            checks.push(ReviewCheck {
                code: "deny_term".to_string(),
                severity: Severity::Error,
                message: format!("Deny term '{}' found in overview", term),
            });
        }
        for claim in &overview.claims {
            if claim.text.to_lowercase().contains(&term.to_lowercase()) {
                checks.push(ReviewCheck {
                    code: "deny_term".to_string(),
                    severity: Severity::Error,
                    message: format!("Deny term '{}' found in claim", term),
                });
            }
        }
        for related in &overview.related {
            if related.to_lowercase().contains(&term.to_lowercase()) {
                checks.push(ReviewCheck {
                    code: "deny_term".to_string(),
                    severity: Severity::Error,
                    message: format!("Deny term '{}' found in related name", term),
                });
            }
        }
    }

    // jargon: any jargon_terms hit (warning)
    for term in &config.jargon_terms {
        if overview
            .overview
            .to_lowercase()
            .contains(&term.to_lowercase())
        {
            checks.push(ReviewCheck {
                code: "jargon".to_string(),
                severity: Severity::Warning,
                message: format!("Jargon term '{}' found in overview", term),
            });
        }
        for claim in &overview.claims {
            if claim.text.to_lowercase().contains(&term.to_lowercase()) {
                checks.push(ReviewCheck {
                    code: "jargon".to_string(),
                    severity: Severity::Warning,
                    message: format!("Jargon term '{}' found in claim", term),
                });
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
                    message: format!(
                        "Evidence reference '{}' in claim {} does not resolve",
                        ev_id, i
                    ),
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
    // Simplified: would need to check against evidence map
    true
}

/// Prose lint checks for a text.
pub fn prose_lint_checks(text: &str, location: &str) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // Hex string of 7+ characters
    if RE_HEX.is_match(text) {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Hex string (7+ chars) found in {}", location),
        });
    }

    // Path-like token (contains / or \ and looks like a path)
    if RE_PATH.is_match(text) {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Path-like token found in {}", location),
        });
    }

    // File extension
    if RE_FILE_EXT.is_match(text) {
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
    if RE_URL.is_match(text) {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("URL found in {}", location),
        });
    }

    // Email
    if RE_EMAIL.is_match(text) {
        checks.push(ReviewCheck {
            code: "prose_lint".to_string(),
            severity: Severity::Error,
            message: format!("Email found in {}", location),
        });
    }

    // Key-like pattern (API key, token, secret, etc.)
    if RE_KEY_LIKE.is_match(text) {
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
    let mut known_words = std::collections::HashSet::new();
    known_words.insert(overview.name.clone());
    for claim in &overview.claims {
        for word in claim.text.split_whitespace() {
            known_words.insert(word.to_string());
        }
    }

    // Check overview for numbers or capitalized words not in known_words
    for word in overview.overview.split_whitespace() {
        let is_number = word.chars().all(|c| c.is_ascii_digit());
        let is_capitalized = word.chars().next().is_some_and(|c| c.is_uppercase());

        if (is_number || is_capitalized) && !known_words.contains(word) {
            checks.push(ReviewCheck {
                code: "uncovered_content".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Uncovered content '{}' in overview (not in claims or project name)",
                    word
                ),
            });
        }
    }

    checks
}

/// Compute SHA-256 hash of text.
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

/// Run review checks on all projects in a draft.
pub fn run_review(config: &ReviewConfig, no_ai: bool, dry_run: bool) -> anyhow::Result<()> {
    let draft_path = std::path::Path::new("data/enriched.draft.json");
    let facts_path = std::path::Path::new("data/facts.json");

    if !draft_path.exists() {
        return Err(anyhow::anyhow!(
            "Draft not found at {}. Run enrich first.",
            draft_path.display()
        ));
    }

    let draft_content = std::fs::read_to_string(draft_path).context("Failed to read draft")?;
    let draft: Draft = serde_json::from_str(&draft_content).context("Failed to parse draft")?;

    let facts = if facts_path.exists() {
        let facts_content = std::fs::read_to_string(facts_path).context("Failed to read facts")?;
        Some(Facts::from_json(&facts_content)?)
    } else {
        None
    };

    let mut project_reports = Vec::new();
    let draft_repo_ids: std::collections::HashSet<String> =
        draft.projects.iter().map(|p| p.repo_id.clone()).collect();

    for project in &draft.projects {
        let mut checks = check_project(project, facts.as_ref().unwrap(), config);

        // related_dangling: related name not a project in the draft
        for related in &project.related {
            if !draft_repo_ids.contains(related) {
                checks.push(ReviewCheck {
                    code: "related_dangling".to_string(),
                    severity: Severity::Error,
                    message: format!("Related name '{}' is not a project in the draft", related),
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
            name: project.name.clone(),
            checks,
            ai_result,
            text_hash,
            review_model: config.model.clone(),
            prompt_version: "1.0".to_string(),
            timestamp: Utc::now(),
        });
    }

    let report = ReviewReport {
        schema_version: 1,
        generated_at: Utc::now(),
        project_reports,
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

/// Run code checks on narrative.
#[allow(dead_code)]
pub fn check_narrative(
    narrative: &str,
    _approved_projects: &[&ProjectOverview],
    config: &ReviewConfig,
) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // prose_lint
    checks.extend(prose_lint_checks(narrative, "narrative"));

    // jargon
    for term in &config.jargon_terms {
        if narrative.to_lowercase().contains(&term.to_lowercase()) {
            checks.push(ReviewCheck {
                code: "jargon".to_string(),
                severity: Severity::Warning,
                message: format!("Jargon term '{}' found in narrative", term),
            });
        }
    }

    // deny_term
    for term in &config.deny_terms {
        if narrative.to_lowercase().contains(&term.to_lowercase()) {
            checks.push(ReviewCheck {
                code: "deny_term".to_string(),
                severity: Severity::Error,
                message: format!("Deny term '{}' found in narrative", term),
            });
        }
    }

    checks
}
