# Cultist agent hot path

Do the requested task first. Notice repository friction only when it changes a justified next action
or exposes a repeatable product problem; do not widen every inconvenience into Cultist work.

## Always

- Ground claims in an exact task, change, file, issue, PR, commit, or repository observation. Mark
  consequential evidence `PROVEN`, `DERIVED`, `OBSERVED`, `INFERRED`, or `UNKNOWN`; seek a negative
  control before generalizing and keep chronology separate from causality. Bind remote prose or
  metadata to its exact work, head, and freshness before treating it as current intent.
- Human-facing third-party GitHub references must be backlink-safe before writing. Use plain
  `OWNER/REPOSITORY issue 123` wording when click-through is unnecessary, or literal
  `https://redirect.github.com/...` links. Never emit direct third-party `github.com` links or
  `OWNER/REPOSITORY#123` shorthand in interaction text or commit messages. Owned
  `teamleaderleo/*` references are exempt.
- Preflight the exact proposed interaction text before a GitHub write:

  ```sh
  python3 scripts/external_github_reference_guard.py --repository teamleaderleo/cultist --stdin
  ```

  Pipe the exact text on standard input. On non-interaction machine/evidence surfaces only,
  provider/API coordinates and retained source evidence may remain canonical where identity
  requires them; this never exempts interaction preflight.

## Route by task

- For concurrent PR compatibility or reanchoring, open
  [`docs/concurrent-work-promotion.md`](docs/concurrent-work-promotion.md). A new `main` SHA alone
  does not invalidate a successful receipt; inspect changed compatibility inputs.
- For external references, open
  [`docs/external-reference-policy.md`](docs/external-reference-policy.md).
- For dogfood signals, evidence receipts, view taxonomy, promotion choices, and examples, search
  headings in [`docs/agent-playbook.md`](docs/agent-playbook.md) and open only the matching section.
  Issue references there are historical routing hints; query current issue/PR state when it decides
  the next action.
- Prefer shared evidence/provenance/freshness primitives plus a task-specific projection. Do not
  create competing truth vocabularies or query surfaces.

## Finish

Preserve the smallest useful consequence: a focused regression or negative control, research
receipt, existing owner update, or exact evidence-backed handoff. Record when surfaced evidence
changed the next inspection, validation, coordination, implementation, preservation, or stop
decision; otherwise keep it quiet.
