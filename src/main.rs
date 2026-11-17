use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod decisions;
mod dry_run;
mod export;
mod facts;
mod git;
mod init;
mod languages;
mod narrative;
mod review_ai;
mod review_checks;
mod review_ui;
mod scan;
mod state;

// M2 modules
mod ai_client;
mod digest;
mod model;
mod redact;

#[derive(Parser)]
#[command(name = "portfolio-engine")]
#[command(about = "Scan local git repos and build a portfolio overview")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive setup: pick directories to scan
    Init {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Write example config and exit
        #[arg(long)]
        example: bool,
    },
    /// Scan configured directories and write facts.json
    Scan {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Dry run: print table and write nothing
        #[arg(long)]
        dry_run: bool,
        /// Validate output against schema (default: true in tests)
        #[arg(long)]
        validate: bool,
    },
    /// List available AI models and probe the chosen one
    AiModels {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Role: enrich or review (default: enrich)
        #[arg(long, default_value = "enrich")]
        role: String,
    },
    /// Select projects for AI enrichment
    AiSelect {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to facts file (default: data/facts.json)
        #[arg(short, long, default_value = "data/facts.json")]
        facts: PathBuf,
    },
    /// Enrich projects with AI-generated prose
    Enrich {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to facts file (default: data/facts.json)
        #[arg(short, long, default_value = "data/facts.json")]
        facts: PathBuf,
        /// Dry run: print request bodies without sending
        #[arg(long)]
        dry_run: bool,
        /// Force re-fetching from AI (ignore cache)
        #[arg(long)]
        force: bool,
    },
    /// Run review checks and AI review
    ReviewRun {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to facts file (default: data/facts.json)
        #[arg(short, long, default_value = "data/facts.json")]
        facts: PathBuf,
        /// Path to enriched draft (default: data/enriched.draft.json)
        #[arg(short, long, default_value = "data/enriched.draft.json")]
        draft: PathBuf,
        /// Path to overrides file (default: overrides.toml)
        #[arg(short, long, default_value = "overrides.toml")]
        overrides: PathBuf,
        /// Only review a specific project by name
        #[arg(long)]
        project: Option<String>,
        /// Skip AI review (code checks only)
        #[arg(long)]
        no_ai: bool,
        /// Dry run: print request bodies without sending
        #[arg(long)]
        dry_run: bool,
        /// Force re-review (ignore cache)
        #[arg(long)]
        force: bool,
        /// Allow same model as writer
        #[arg(long)]
        allow_same_model: bool,
    },
    /// Interactive review loop
    Review {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to review report (default: data/review.report.json)
        #[arg(short, long, default_value = "data/review.report.json")]
        report: PathBuf,
    },
    /// Show review status table
    ReviewStatus {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to review report (default: data/review.report.json)
        #[arg(short, long, default_value = "data/review.report.json")]
        report: PathBuf,
        /// Path to overrides file (default: overrides.toml)
        #[arg(short, long, default_value = "overrides.toml")]
        overrides: PathBuf,
        /// Path to enriched draft (default: data/enriched.draft.json)
        #[arg(short, long, default_value = "data/enriched.draft.json")]
        draft: PathBuf,
    },
    /// Generate and review narrative
    ReviewNarrative {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to facts file (default: data/facts.json)
        #[arg(short, long, default_value = "data/facts.json")]
        facts: PathBuf,
        /// Path to enriched draft (default: data/enriched.draft.json)
        #[arg(short, long, default_value = "data/enriched.draft.json")]
        draft: PathBuf,
        /// Path to overrides file (default: overrides.toml)
        #[arg(short, long, default_value = "overrides.toml")]
        overrides: PathBuf,
        /// Dry run: print request body without sending
        #[arg(long)]
        dry_run: bool,
        /// Force re-review (ignore cache)
        #[arg(long)]
        force: bool,
    },
    /// Export reviewed.json from approved projects
    ReviewExport {
        /// Path to config file (default: config.toml)
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to facts file (default: data/facts.json)
        #[arg(short, long, default_value = "data/facts.json")]
        facts: PathBuf,
        /// Path to enriched draft (default: data/enriched.draft.json)
        #[arg(short, long, default_value = "data/enriched.draft.json")]
        draft: PathBuf,
        /// Path to overrides file (default: overrides.toml)
        #[arg(short, long, default_value = "overrides.toml")]
        overrides: PathBuf,
        /// Output path (default: data/reviewed.json)
        #[arg(short, long, default_value = "data/reviewed.json")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { config, example } => {
            if example {
                init::write_example_config(&config)?;
            } else {
                init::run_init(&config)?;
            }
        }
        Commands::Scan {
            config,
            dry_run,
            validate,
        } => {
            let config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;

            let facts = scan::scan(&config, dry_run).context("Scan failed")?;

            if dry_run {
                dry_run::print_dry_run(&facts)?;
            } else {
                if validate {
                    let schema_path = PathBuf::from("schemas/facts.schema.json");
                    if schema_path.exists() {
                        facts
                            .validate_schema(&schema_path)
                            .context("Schema validation failed")?;
                    }
                }
                println!(
                    "Scan complete. {} projects written to data/facts.json",
                    facts.projects.len()
                );
            }
        }
        Commands::AiModels { config, role } => {
            let config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            if role == "review" {
                review_ai::run_ai_models_review(&config).context("AI models review failed")?;
            } else {
                println!("ai models --role enrich not yet implemented");
            }
        }
        Commands::AiSelect { config, facts: _ } => {
            let _config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            println!("ai select command not yet implemented");
        }
        Commands::Enrich {
            config,
            facts: _,
            dry_run,
            force: _,
        } => {
            let _config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            if dry_run {
                println!("enrich --dry-run command not yet implemented");
            } else {
                println!("enrich command not yet implemented");
            }
        }
        Commands::ReviewRun {
            config,
            facts: _,
            draft: _,
            overrides: _,
            project: _,
            no_ai,
            dry_run,
            force: _,
            allow_same_model: _,
        } => {
            let config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            review_checks::run_review(&config.review, no_ai, dry_run)
                .context("Review run failed")?;
        }
        Commands::Review { config, report: _ } => {
            let _config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            println!("review interactive command not yet implemented");
        }
        Commands::ReviewStatus {
            config,
            report: _,
            overrides: _,
            draft: _,
        } => {
            let config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            decisions::run_status(&config.review).context("Review status failed")?;
        }
        Commands::ReviewNarrative {
            config,
            facts: _,
            draft: _,
            overrides: _,
            dry_run,
            force: _,
        } => {
            let config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            narrative::run_narrative_review(&config.review, dry_run)
                .context("Narrative review failed")?;
        }
        Commands::ReviewExport {
            config,
            facts: _,
            draft: _,
            overrides: _,
            output,
        } => {
            let config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            export::run_export(&config.review, &output).context("Review export failed")?;
        }
    }

    Ok(())
}
