use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod dry_run;
mod facts;
mod git;
mod init;
mod languages;
mod scan;
mod state;

// M2 modules
mod ai_client;
mod cache;
mod digest;
mod enrich;
mod model;
mod pickers;
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
        Commands::AiModels { config } => {
            let _config = config::Config::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;
            println!("ai models command not yet implemented");
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
    }

    Ok(())
}
