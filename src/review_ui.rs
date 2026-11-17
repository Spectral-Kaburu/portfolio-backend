//! Review UI module - interactive review loop for M3.

use crate::config::ReviewConfig;
use crate::model::{Decision, ProjectReport, ReviewReport};
use anyhow::Result;
use dialoguer::{Confirm, Input};
use std::path::PathBuf;

/// Run the interactive review loop.
#[allow(dead_code)]
pub fn run_interactive_loop(
    report: &ReviewReport,
    draft: &crate::model::Draft,
    overrides: &mut crate::model::Overrides,
    facts: &crate::facts::Facts,
    config: &ReviewConfig,
) -> Result<()> {
    println!("=== Interactive Review ===");
    println!(
        "Review report has {} projects",
        report.project_reports.len()
    );

    // Filter projects that need review (pending or stale, errors first)
    let mut projects_to_review: Vec<&ProjectReport> = report
        .project_reports
        .iter()
        .filter(|pr| {
            let project = draft.projects.iter().find(|p| p.repo_id == pr.repo_id);
            let override_opt = overrides.projects.get(&pr.repo_id);
            let decision = crate::decisions::effective_decision(override_opt, project.unwrap());
            matches!(decision, Decision::Pending | Decision::Stale)
        })
        .collect();

    // Sort: errors first, then by name
    projects_to_review.sort_by(|a, b| {
        let a_errors = a
            .checks
            .iter()
            .filter(|c| c.severity == crate::model::Severity::Error)
            .count();
        let b_errors = b
            .checks
            .iter()
            .filter(|c| c.severity == crate::model::Severity::Error)
            .count();
        b_errors.cmp(&a_errors).then_with(|| {
            let a_project = draft.projects.iter().find(|p| p.repo_id == a.repo_id);
            let b_project = draft.projects.iter().find(|p| p.repo_id == b.repo_id);
            a_project
                .map(|p| p.name.as_str())
                .unwrap_or("")
                .cmp(b_project.map(|p| p.name.as_str()).unwrap_or(""))
        })
    });

    for project_report in projects_to_review {
        let project = draft
            .projects
            .iter()
            .find(|p| p.repo_id == project_report.repo_id)
            .unwrap();

        println!("\n--- Project: {} ({}) ---", project.name, project.repo_id);
        println!("Overview: {}", project.overview);
        println!("Status: {:?}", project.status);
        println!("Claims: {}", project.claims.len());

        // Show code checks
        for check in &project_report.checks {
            let severity_str = match check.severity {
                crate::model::Severity::Error => "ERROR",
                crate::model::Severity::Warning => "WARN",
            };
            println!("  [{}] {}: {}", severity_str, check.code, check.message);
        }

        // Show AI result if present
        if let Some(ai_result) = &project_report.ai_result {
            println!("AI Review: {}", ai_result.overall);
            for claim in &ai_result.claims {
                println!(
                    "  Claim {}: {:?} - {}",
                    claim.index, claim.verdict, claim.reason
                );
            }
            for jargon in &ai_result.jargon {
                println!("  Jargon: '{}' -> '{}'", jargon.phrase, jargon.plain);
            }
            for private in &ai_result.private {
                println!("  Private: '{}' ({})", private.phrase, private.why);
            }
            if let Some(suggestion) = &ai_result.suggested_overview {
                println!("  Suggestion: {}", suggestion);
            }
        }

        // Interactive actions
        loop {
            println!("\nActions:");
            println!("  1) Approve");
            if project_report.ai_result.is_some() {
                println!("  2) Approve with suggestion");
            }
            println!("  3) Edit overview");
            println!("  4) Hide");
            println!("  5) Skip");
            println!("  6) Quit");

            let choice: usize = Input::new()
                .with_prompt("Choose action (1-6)")
                .default(5)
                .interact_text()?;

            match choice {
                1 => {
                    // Approve
                    let published_text = overrides
                        .projects
                        .get(&project.repo_id)
                        .and_then(|o| o.overview.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or(&project.overview);
                    *overrides = crate::decisions::apply_approve(
                        overrides.clone(),
                        &project.repo_id,
                        published_text,
                    );
                    println!("Approved!");

                    // Ask about featured
                    if Confirm::new()
                        .with_prompt("Featured?")
                        .default(false)
                        .interact()?
                    {
                        *overrides = crate::decisions::apply_featured_toggle(
                            overrides.clone(),
                            &project.repo_id,
                            true,
                        );
                    }

                    // Ask about built
                    if Confirm::new()
                        .with_prompt("Built?")
                        .default(false)
                        .interact()?
                    {
                        *overrides = crate::decisions::apply_built_toggle(
                            overrides.clone(),
                            &project.repo_id,
                            true,
                        );
                    }
                    break;
                }
                2 => {
                    // Get suggestion if available
                    let suggestion_opt = project_report
                        .ai_result
                        .as_ref()
                        .and_then(|r| r.suggested_overview.as_ref());

                    if let Some(suggestion) = suggestion_opt {
                        // Verify suggestion passes checks
                        let checks = crate::review_checks::check_project(project, facts, config);
                        let has_errors = checks
                            .iter()
                            .any(|c| c.severity == crate::model::Severity::Error);
                        if has_errors {
                            println!("Suggestion has errors, cannot approve with suggestion");
                            continue;
                        }

                        *overrides = crate::decisions::apply_approve_with_suggestion(
                            overrides.clone(),
                            &project.repo_id,
                            suggestion,
                            suggestion,
                        )?;
                        println!("Approved with suggestion!");

                        if Confirm::new()
                            .with_prompt("Featured?")
                            .default(false)
                            .interact()?
                        {
                            *overrides = crate::decisions::apply_featured_toggle(
                                overrides.clone(),
                                &project.repo_id,
                                true,
                            );
                        }
                        if Confirm::new()
                            .with_prompt("Built?")
                            .default(false)
                            .interact()?
                        {
                            *overrides = crate::decisions::apply_built_toggle(
                                overrides.clone(),
                                &project.repo_id,
                                true,
                            );
                        }
                        break;
                    }
                    println!("No suggestion available");
                }
                3 => {
                    // Edit overview
                    let current = overrides
                        .projects
                        .get(&project.repo_id)
                        .and_then(|o| o.overview.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or(&project.overview);

                    let new_overview: String = Input::new()
                        .with_prompt("New overview")
                        .default(current.to_string())
                        .interact_text()?;

                    // Run code checks on new text
                    let code_checks_fn = |text: &str| -> Vec<crate::model::ReviewCheck> {
                        crate::review_checks::prose_lint_checks(text, "overview")
                    };

                    match crate::decisions::apply_edit(
                        overrides.clone(),
                        &project.repo_id,
                        &new_overview,
                        code_checks_fn,
                    ) {
                        Ok(new_overrides) => {
                            *overrides = new_overrides;
                            println!("Overview updated!");

                            if Confirm::new()
                                .with_prompt("Featured?")
                                .default(false)
                                .interact()?
                            {
                                *overrides = crate::decisions::apply_featured_toggle(
                                    overrides.clone(),
                                    &project.repo_id,
                                    true,
                                );
                            }
                            if Confirm::new()
                                .with_prompt("Built?")
                                .default(false)
                                .interact()?
                            {
                                *overrides = crate::decisions::apply_built_toggle(
                                    overrides.clone(),
                                    &project.repo_id,
                                    true,
                                );
                            }
                            break;
                        }
                        Err(e) => {
                            println!("Edit rejected: {}", e);
                        }
                    }
                }
                4 => {
                    *overrides = crate::decisions::apply_hide(overrides.clone(), &project.repo_id);
                    println!("Hidden!");
                    break;
                }
                5 => {
                    // Skip - no change
                    println!("Skipped");
                    break;
                }
                6 => {
                    println!("Quitting...");
                    return Ok(());
                }
                _ => {
                    println!("Invalid choice");
                }
            }
        }

        // Save after each action
        crate::decisions::save_overrides(overrides, &PathBuf::from("overrides.toml"))?;
    }

    println!("\nReview complete!");
    Ok(())
}

/// Print status table.
#[allow(dead_code)]
pub fn print_status_table(
    report: &ReviewReport,
    draft: &crate::model::Draft,
    overrides: &crate::model::Overrides,
) -> Result<()> {
    use tabled::{Table, Tabled};

    #[derive(Tabled)]
    struct StatusRow {
        project: String,
        decision: String,
        errors: usize,
        warnings: usize,
        stale: bool,
    }

    let mut rows = Vec::new();

    for project_report in &report.project_reports {
        let project = draft
            .projects
            .iter()
            .find(|p| p.repo_id == project_report.repo_id);
        let project_name = project.map(|p| p.name.as_str()).unwrap_or("unknown");
        let override_opt = overrides.projects.get(&project_report.repo_id);
        let decision = if let Some(project) = project {
            crate::decisions::effective_decision(override_opt, project)
        } else {
            Decision::Pending
        };

        let errors = project_report
            .checks
            .iter()
            .filter(|c| c.severity == crate::model::Severity::Error)
            .count();
        let warnings = project_report
            .checks
            .iter()
            .filter(|c| c.severity == crate::model::Severity::Warning)
            .count();
        let stale = matches!(decision, Decision::Stale);

        rows.push(StatusRow {
            project: project_name.to_string(),
            decision: format!("{:?}", decision),
            errors,
            warnings,
            stale,
        });
    }

    // Add narrative row
    let narrative_decision = crate::decisions::narrative_effective_decision(
        &overrides.narrative,
        &draft.recent_activity.summary,
        "", // approved_set_hash - would need to compute
    );
    let narrative_stale = matches!(narrative_decision, Decision::Stale);

    rows.push(StatusRow {
        project: "NARRATIVE".to_string(),
        decision: format!("{:?}", narrative_decision),
        errors: 0,
        warnings: 0,
        stale: narrative_stale,
    });

    let table = Table::new(rows);
    println!("{}", table);

    Ok(())
}
