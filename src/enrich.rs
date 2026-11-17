//! Enrich module - orchestrates AI enrichment.
//!
//! RUST NOTE: This module handles:
//! 1. Building digests from facts
//! 2. Calling the AI API
//! 3. Validating responses
//! 4. Caching results
//! 5. Writing the draft output

use crate::model::Draft;

/// Enrichment result for a single project.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EnrichmentResult {
    pub draft: Option<Draft>,
    pub failed: Vec<String>, // Project names that failed
}

/// Run enrichment on opted-in projects.
#[allow(dead_code)]
pub fn enrich(
    _facts_path: &std::path::Path,
    _config: &crate::config::AiConfig,
) -> anyhow::Result<EnrichmentResult> {
    // Stub - implementation in M2
    Ok(EnrichmentResult {
        draft: None,
        failed: Vec::new(),
    })
}

/// Dry-run: print request bodies without sending.
#[allow(dead_code)]
pub fn dry_run(
    _facts_path: &std::path::Path,
    _config: &crate::config::AiConfig,
) -> anyhow::Result<()> {
    // Stub - implementation in M2
    println!("Dry run not implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub() {
        // Stub test
    }
}
