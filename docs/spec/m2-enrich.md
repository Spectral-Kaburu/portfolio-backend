# M2: Enrichment - Redaction and Digest Building

## Goal
Build the privacy core: redaction module and digest building - pure functions that never leak sensitive data.

## Contract
- `redact(text: &str) -> Redacted` - returns cleaned text and count
- `build_project_digest(project, active_days) -> ProjectDigest` - builds digest for one project
- `build_digest(facts, active_days, max_bytes) -> String` - builds complete digest
- `build_evidence_map(project) -> EvidenceMap` - maps E1, E2... to real refs

## 39. The digest
The digest is the ONLY thing that leaves the machine. It contains:
- Project name (not path)
- Status (active/dormant based on last commit)
- Commit count
- Date-only timestamps (no sub-date times)
- Recent commit subjects (shortened)
- Aggregate stats

The digest excludes:
- `remote_url` - may contain credentials
- Commit hashes - replaced with evidence references
- Author data - dates only for status determination
- Sub-date timestamps - only date strings like "2024-01-15"

## 40. Privacy model
- Only allowlisted file contents are read
- Only a redacted digest of facts is sent to the model
- Redaction removes key-like strings and emails before anything leaves the machine
- Public output contains an overview in plain language only

## 50. Redaction patterns
All patterns must match and replace with `[REDACTED]`. Patterns:

1. **API keys** - Various formats:
   - AWS: `AKIA[0-9A-Z]{16}`
   - Generic key: `key["']?\s*[:=]\s*["']?[a-zA-Z0-9_\-]{16,}`
   - OpenAI-style: `sk-[a-zA-Z0-9]{20,}`
   - Generic API key env var: `[A-Z_]+_API_KEY=["']?[a-zA-Z0-9_\-]{8,}`

2. **Emails** - `[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+`

3. **URLs with credentials** - `https://[^/]+:[^@]+@`

4. **Private keys** - `-----BEGIN (RSA | DSA | EC | OPENSSH) PRIVATE KEY-----` through `-----END ... PRIVATE KEY-----`

5. **Long hex strings** - 32+ consecutive hex characters (potential hashes, keys)

6. **Bearer tokens** - `Bearer\s+[a-zA-Z0-9_\-\.]+`

## 60. Language mix
- Top 3 languages only
- Excluded: Other, Markdown, Text, JSON, TOML, YAML
- Renormalized to 100% (shares sum to 1.0)
- If fewer than 3 valid languages, include what's available

## 84. Language filtering
Only count programming languages. Filter out:
- Other
- Markdown
- Text
- JSON
- TOML
- YAML

## 85. Size cap enforcement
Digest must fit within `max_bytes`. Strategy:
1. Build initial digest
2. If too large:
   - First: drop oldest commit subjects (keep stats)
   - Then: shorten README excerpt
   - If still too large: fail with error

## 86. Evidence map
Maps E1, E2, E3... to actual references:
- `E1` → `commit:abc123...` (recent commits)
- `E2` → `readme` (README excerpt)
- `E3` → `manifest:package.json` (manifest files)
- `stats` → aggregate statistics

## 87. Date formatting
- Dates formatted as `YYYY-MM-DD` only
- No time component, no timezone

## 88. Commit subjects
- Truncated to 60 characters max
- If longer, truncate and add `...`
- Used as evidence references

## 89. Validation
Before sending to AI:
- Verify no API keys, emails, or URLs with credentials in digest
- Verify language percentages sum to 1.0 (within floating point tolerance)
- Verify all required fields present