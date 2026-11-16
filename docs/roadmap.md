# Roadmap (M4-M5)

**Do not build any of this until the human activates the milestone.** Each gets its own detailed spec after the previous milestone's lesson is done. Agents should not read this file during M1.

## M2: Enrich
- Written. See `docs/spec/m2-enrich.md`.

## M3: Review
- Written. See `docs/spec/m3-review.md`.

## M4: Emit and frontend contract
- `frontends/<name>/plan.md`, `contract.schema.json`, `mapping`. Adapter command proposes a contract from the plan. Human approves and freezes it.
- `emit` validates final JSON against the frozen contract, strips evidence and hashes, fails loudly on mismatch.
- Includes a plain placeholder page that renders the JSON.

## M5: Publish
- Writes reviewed output to the separate Pages repo, commits and pushes only after explicit human approval.
- Then the real frontend.
