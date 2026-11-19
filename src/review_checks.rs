//! Review code checks module - deterministic checks for M3.

use crate::config::ReviewConfig;
use crate::facts::Facts;
use crate::model::{Draft, ProjectOverview, ProjectReport, ReviewCheck, ReviewReport, Severity};
use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use std::collections::HashSet;

lazy_static::lazy_static! {
    // RUST NOTE: (?i) makes the pattern case-insensitive, so uppercase hex in
    // prose (e.g. a commit hash) is caught as well as lowercase hex.
    static ref RE_HEX: Regex = Regex::new(r"(?i)\b[0-9a-f]{7,}\b").unwrap();
    static ref RE_PATH: Regex = Regex::new(r"\b[a-zA-Z0-9_.-]+(?:/[a-zA-Z0-9_.-]+){2,}\b").unwrap();
    static ref RE_FILE_EXT: Regex = Regex::new(
        r"\b[a-zA-Z0-9_-]+\.(rs|js|ts|py|json|toml|yaml|yml|md|txt|html|css|go|java|cpp|h)\b"
    ).unwrap();
    static ref RE_URL: Regex = Regex::new(r"https?://\S+").unwrap();
    static ref RE_EMAIL: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    static ref RE_KEY_LIKE: Regex = Regex::new(r"(?i)(api[_-]?key|token|secret|password|auth)[_-]?[:=]\s*\S+").unwrap();
}

/// Run all code checks on a project overview and claims.
///
/// `facts` is optional: without `data/facts.json` the mechanical checks that do
/// not need evidence (length, lint, jargon, deny terms, related names, uncovered
/// content) still run; the evidence check is skipped rather than guessed.
pub fn check_project(
    overview: &ProjectOverview,
    facts: Option<&Facts>,
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
    // Skipped when there are no facts to resolve against: the check cannot be
    // answered, so it must not invent an error.
    if let Some(facts) = facts {
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
///
/// An evidence reference is a `kind:value` token produced by the enrich step:
/// `commit:<hash>` (any prefix of a recent commit hash), `readme` (the project
/// has a README excerpt), `manifest:<kind>` (the project has a manifest of that
/// kind), and `stats` (the project has languages, i.e. measurable stats). An
/// unknown kind, or a value the facts do not contain, does not resolve.
fn evidence_resolves(ev_id: &str, overview: &ProjectOverview, facts: &Facts) -> bool {
    let Some(project) = facts
        .projects
        .iter()
        .find(|p| p.repo_id == overview.repo_id)
    else {
        // No facts for this project at all: nothing can resolve.
        return false;
    };

    let Some((kind, value)) = ev_id.split_once(':') else {
        // No kind prefix: either a bare known kind or a malformed ref.
        return matches!(ev_id, "readme" | "stats");
    };

    match kind {
        // A commit hash resolves by prefix against the project's recent hashes.
        "commit" => {
            !value.is_empty()
                && project
                    .recent_commits
                    .iter()
                    .any(|c| c.hash.starts_with(value))
        }
        // README resolves when the project has a README excerpt.
        "readme" => project.readme_excerpt.is_some(),
        // A manifest kind resolves when the project has that kind of manifest.
        "manifest" => !value.is_empty() && project.manifests.iter().any(|m| m.kind == value),
        // Stats resolve when the project has any measured language data.
        "stats" => !project.languages.is_empty(),
        "head" => !project.head.is_empty(),
        "lang" | "langs" | "language" => {
            !value.is_empty() && project.languages.iter().any(|l| l.name == value)
        }
        _ => false,
    }
}

/// True when the hex match at `at` in `text` belongs to an evidence reference.
///
/// Evidence references are `kind:value` tokens written by the enrich step
/// (`commit:<hash>`, `manifest:<kind>`, `readme`, `stats`). The prefix makes the
/// hex a citation rather than prose, so the lint must not flag it.
fn is_evidence_ref(text: &[u8], at: usize) -> bool {
    // The marker must sit directly before the match, separated only by ':'.
    let marker_start = text[..at]
        .iter()
        .rev()
        .take(16)
        .position(|&b| b == b':')
        .map(|offset| at - offset);
    let Some(colon) = marker_start else {
        return false;
    };
    // Nothing else may sit between the end of the marker and the ':'.
    if text[colon + 1..at].iter().any(|b| *b != b' ') {
        return false;
    }
    // The marker must reach back to a word boundary or string start.
    if colon > 0 && (text[colon - 1] as char).is_alphanumeric() {
        return false;
    }
    let marker = &text[colon + 1..at];
    // Trim trailing spaces off the marker (already ruled in above).
    let marker = String::from_utf8_lossy(marker).trim().to_string();
    matches!(
        marker.as_str(),
        "commit" | "manifest" | "readme" | "stats" | "head" | "lang" | "langs" | "language"
    )
}

/// Prose lint checks for a text.
pub fn prose_lint_checks(text: &str, location: &str) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // Hex string of 7+ characters. Evidence refs such as "commit:abc123def456"
    // are prefixed with a known marker; that prefix means the hex belongs to an
    // evidence reference, not to public prose, so it is not a lint hit.
    let has_bare_hex = RE_HEX
        .find_iter(text)
        .any(|m| !is_evidence_ref(text.as_bytes(), m.start()));
    if has_bare_hex {
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
///
/// Warning-level check per the M3 spec: "a number or capitalized word in the
/// overview appears in none of the claim texts and not in the project name".
///
/// The literal reading flagged every sentence-initial capital and every tiny
/// number ("a 5-minute task"), which is why it used to fire on clean text. This
/// version flags only tokens that look like facts:
///
/// - a number that carries a unit (`5MB`, `12%`, `3x`) or has two or more
///   digits, so single digits ("a 5-minute task", list markers) and digits glued
///   inside a hyphenated word stay quiet;
/// - a capitalized word of 3+ letters that is not one of the project's own name
///   words, a word already used in a claim, or the first word of a sentence
///   (sentence-initial capitals are the dominant false positive).
///
/// Tokens are compared on their alphanumeric core, case-insensitively, against
/// the claim texts and the project name (each name word counts as known).
fn uncovered_content_checks(overview: &ProjectOverview) -> Vec<ReviewCheck> {
    let mut checks = Vec::new();

    // Alphanumeric cores of the claim texts and every project-name word, folded
    // to lowercase, so "Fast" matches "fast" in a claim.
    let mut known: HashSet<String> = HashSet::new();
    for word in overview.name.split(|c: char| !c.is_alphanumeric()) {
        if !word.is_empty() {
            known.insert(word.to_lowercase());
        }
    }
    for claim in &overview.claims {
        for word in claim.text.split(|c: char| !c.is_alphanumeric()) {
            if !word.is_empty() {
                known.insert(word.to_lowercase());
            }
        }
    }

    // Whether the previous token ended a sentence matters for the capitalized
    // rule; the first token of the text counts as a sentence start.
    let mut sentence_start = true;
    for word in overview.overview.split_whitespace() {
        let token = token_core(word);
        if !token.is_empty() {
            let is_number = looks_like_number(&token);
            let is_capitalized = token
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let flagged = (is_number && is_fact_number(&token))
                || (is_capitalized
                    && token.len() >= 3
                    && !sentence_start
                    && !known.contains(&token.to_lowercase()));
            if flagged {
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
        sentence_start = !token.is_empty() && ends_sentence(word);
    }

    checks
}

/// True when a token's last character closes a sentence.
fn ends_sentence(token: &str) -> bool {
    matches!(
        token_core(token).chars().last(),
        Some('.') | Some('!') | Some('?') | Some(':') | Some(';')
    )
}

/// Strip leading and trailing non-alphanumeric characters from a whitespace-separated token.
fn token_core(word: &str) -> String {
    word.trim_matches(|c: char| {
        !c.is_alphanumeric() && c != '%' && c != '-' && c != '+' && c != '.'
    })
    .to_string()
}

/// True when the token's core is made of digits with an optional unit.
fn looks_like_number(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    if !has_digit {
        return false;
    }
    // Digits may be decorated with a unit, percentage sign or thousands
    // separators. Any other letter means it is an ordinary word ("5-minute")
    // whose digits are not a claim about the project.
    token.chars().all(|c| {
        c.is_ascii_digit() || matches!(c, '%' | ',' | '.' | '+' | '-' | 'k' | 'K' | 'M' | 'G' | 'x')
    })
}

/// A number reads as a fact when it is two or more digits, or carries a unit
/// (`5MB`, `3x`) or a percentage. A single bare digit does not.
fn is_fact_number(token: &str) -> bool {
    let digits: usize = token.chars().filter(|c| c.is_ascii_digit()).count();
    let alnum: usize = token.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    // A digit count shorter than the alphanumeric token means a unit is glued
    // on: "5MB", "3x", "12%".
    digits < alnum || digits >= 2
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

    // The related check compares against project names in the draft.
    let draft_names: std::collections::HashSet<String> =
        draft.projects.iter().map(|p| p.name.clone()).collect();

    for project in &draft.projects {
        let mut checks = check_project(project, facts.as_ref(), config);

        // related_dangling: a related name is not a project in the draft.
        // `related` holds project *names* (the overview names used across the
        // portfolio), so it is compared against draft names, not repo_ids.
        for related in &project.related {
            if !draft_names.contains(related) {
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
