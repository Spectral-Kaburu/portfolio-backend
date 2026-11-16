//! Portfolio Engine - Scan local git repos and build a portfolio overview.

pub mod config;
pub mod dry_run;
pub mod facts;
pub mod git;
pub mod init;
pub mod languages;
pub mod scan;
pub mod state;

// M2 modules
pub mod ai_client;
pub mod cache;
pub mod digest;
pub mod enrich;
pub mod model;
pub mod pickers;
pub mod redact;
