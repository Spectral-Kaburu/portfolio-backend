use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{Config, EXAMPLE_CONFIG};

/// Run the interactive init command.
pub fn run_init(config_path: &Path) -> Result<()> {
    // Check if config already exists
    if config_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt(format!(
                "Config already exists at {}. Overwrite?",
                config_path.display()
            ))
            .default(false)
            .interact()
            .context("Failed to read confirmation")?;
        if !overwrite {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Find candidate directories containing git repos
    let candidates = find_candidate_dirs()?;

    if candidates.is_empty() {
        println!("No git repositories found under your home directory.");
        return Ok(());
    }

    // Show multi-select TUI
    let selected = select_directories(&candidates)?;

    // Allow manual path entry
    let mut roots = selected;
    loop {
        let add_more = Confirm::new()
            .with_prompt("Add another directory manually?")
            .default(false)
            .interact()
            .context("Failed to read confirmation")?;
        if !add_more {
            break;
        }

        let path: String = Input::new()
            .with_prompt("Enter absolute path to directory")
            .interact_text()
            .context("Failed to read path")?;

        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            println!("Path does not exist: {}", path);
            continue;
        }
        if !path_buf.is_dir() {
            println!("Path is not a directory: {}", path);
            continue;
        }

        // Check if it contains git repos
        if !has_git_repos(&path_buf) {
            println!("No git repositories found in: {}", path);
            let proceed = Confirm::new()
                .with_prompt("Add anyway?")
                .default(false)
                .interact()
                .context("Failed to read confirmation")?;
            if !proceed {
                continue;
            }
        }

        roots.push(path_buf);
    }

    if roots.is_empty() {
        println!("No directories selected. Aborted.");
        return Ok(());
    }

    // Create and save config
    let config = Config {
        roots,
        max_depth: 4,
        active_days: 30,
        exclude_dirs: vec![
            "node_modules".to_string(),
            "target".to_string(),
            ".venv".to_string(),
            "vendor".to_string(),
        ],
        ai: crate::config::AiConfig::default(),
        review: crate::config::ReviewConfig::default(),
    };

    config.save(config_path).context("Failed to save config")?;

    println!("Configuration written to {}", config_path.display());
    Ok(())
}

/// Find directories under home that contain git repositories.
fn find_candidate_dirs() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let mut candidates = Vec::new();

    // Search up to 3 levels deep for directories containing .git
    for entry in WalkDir::new(&home)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only consider directories
        if !path.is_dir() {
            continue;
        }

        // Check if it contains a .git directory/file
        if path.join(".git").exists() {
            candidates.push(path.to_path_buf());
        }
    }

    // Sort for consistent display
    candidates.sort();
    candidates.dedup();

    Ok(candidates)
}

/// Check if a directory contains any git repositories.
fn has_git_repos(dir: &Path) -> bool {
    for entry in WalkDir::new(dir)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().join(".git").exists() {
            return true;
        }
    }
    false
}

/// Present multi-select TUI for directory selection.
fn select_directories(candidates: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let items: Vec<String> = candidates.iter().map(|p| p.display().to_string()).collect();

    let defaults = vec![false; items.len()];

    let selections = MultiSelect::new()
        .with_prompt("Select directories to scan (space to toggle, enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .context("Failed to run multi-select")?;

    Ok(selections
        .into_iter()
        .map(|i| candidates[i].clone())
        .collect())
}

/// Write example config file.
pub fn write_example_config(path: &Path) -> Result<()> {
    std::fs::write(path, EXAMPLE_CONFIG)
        .with_context(|| format!("Failed to write example config to {}", path.display()))?;
    println!("Example configuration written to {}", path.display());
    Ok(())
}
