# Roadmap (M2-M5)

**Do not build any of this until the human activates the milestone.** Each gets its own detailed spec after the previous milestone's lesson is done. Agents should not read this file during M1.

## M2: Enrich
- Calls an OpenAI-style chat API on NVIDIA-hosted models. Base URL, model list endpoint, auth and limits below were reported by third-party sources in November 2026. **Re-verify against build.nvidia.com before coding.**
  - Base URL `https://integrate.api.nvidia.com/v1`, key from build.nvidia.com via env var.
  - `GET /v1/models` lists models. A model appearing in the list does not prove chat works with your key (404s reported). Probe before trusting.
  - Free-tier limits are roughly 40 requests per minute and vary by model. Throttle and back off on 429.
  - Model names change often: the model is a config value, never hard-coded.
- `init --models` fetches the live model list and offers a **clickable multi-select** to pick the model.
- Input: redacted digest only. Output: `data/enriched.draft.json` with overview prose per project, a "what I've been up to" narrative, links between new and existing work, and `evidence` on every claim. Request structured output matching a schema and validate it in Rust.
- Redaction pass (key-like strings, emails, URLs with tokens) runs before any request. Test it with sentinel secrets.
- First async-free HTTP lesson. Async is a later, separate lesson.

## M3: Review
- Review agent reads the draft and evidence, verifies each claim, flags jargon, private-looking items and unsupported statements, writes `data/review.report.md`.
- Human decisions go to `overrides.toml` keyed by `repo_id`: include/exclude, featured, summary override, ordering, `built` status. Regeneration must never overwrite them.

## M4: Emit and frontend contract
- `frontends/<name>/plan.md`, `contract.schema.json`, `mapping`. Adapter command proposes a contract from the plan. Human approves and freezes it.
- `emit` validates final JSON against the frozen contract, strips evidence and hashes, fails loudly on mismatch.
- Includes a plain placeholder page that renders the JSON.

## M5: Publish
- Writes reviewed output to the separate Pages repo, commits and pushes only after explicit human approval.
- Then the real frontend.
