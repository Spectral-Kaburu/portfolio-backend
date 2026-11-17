//! Project selection UI for ai select command.
//!
//! RUST NOTE: Uses dialoguer for interactive selection.
//! Writes to overrides.toml preserving comments.

use crate::facts::Facts;

/// Selected projects for AI enrichment.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProjectSelection {
    pub selected: Vec<String>, // repo_ids
}

/// Interactive project selection.
#[allow(dead_code)]
pub fn pick_projects(_facts: &Facts) -> anyhow::Result<ProjectSelection> {
    // Stub - implementation in M2
    Ok(ProjectSelection {
        selected: Vec::new(),
    })
}

/// Load current selection from overrides.toml.
#[allow(dead_code)]
pub fn load_selection(_overrides_path: &std::path::Path) -> anyhow::Result<ProjectSelection> {
    // Stub - implementation in M2
    Ok(ProjectSelection {
        selected: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub() {
        // Stub test
    }
}
