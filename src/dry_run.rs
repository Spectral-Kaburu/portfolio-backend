use crate::facts::Facts;
use anyhow::Result;
use tabled::{Table, Tabled};

/// Row for dry-run table output.
#[derive(Tabled)]
struct DryRunRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Last Commit")]
    last_commit: String,
    #[tabled(rename = "Languages")]
    languages: String,
    #[tabled(rename = "New Commits")]
    new_commits: u32,
}

/// Print dry-run table to stdout.
pub fn print_dry_run(facts: &Facts) -> Result<()> {
    let rows: Vec<DryRunRow> = facts
        .projects
        .iter()
        .map(|p| {
            let languages = if p.languages.is_empty() {
                "no commits".to_string()
            } else {
                p.languages
                    .iter()
                    .map(|l| format!("{} ({:.0}%)", l.name, l.share * 100.0))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let last_commit = if p.last_commit_at.timestamp() == 0 {
                "no commits".to_string()
            } else {
                p.last_commit_at.format("%Y-%m-%d").to_string()
            };

            DryRunRow {
                name: p.name.clone(),
                last_commit,
                languages,
                new_commits: p.new_commits_since_last_scan,
            }
        })
        .collect();

    let table = Table::new(rows).to_string();
    println!("{}", table);

    println!(
        "\nScanned at: {}",
        facts.scanned_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Total projects: {}", facts.projects.len());

    Ok(())
}
