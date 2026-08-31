# Stensibly worker-brief density

## Result

Current issue-1616 fixture, `o200k_base`:

| presentation | tokens | vs explicit |
|---|---:|---:|
| explicit | 1,000 | — |
| terse before duplicate-digest cleanup | 653 | -34.7% |
| terse | 608 | -39.2% |
| terse, lazy policy | 397 | -60.3% |

The first change was pure duplicate removal: one semantic digest already printed at
the top of the brief. It saved 45 tokens without changing the presentation invariant.

The policy pointer is conditional, not a new default. Fresh provider sessions gave:

- policy irrelevant: same JSON, no tools, exactly 183 fewer input tokens;
- policy required: same JSON, one policy read, 12,591 more input tokens.

So: use the pointer only when the dispatch is already typed as policy-independent.
Keep policy eager for authority, validation, evidence, and escalation work. Do not
guess from task prose.

Stensibly implementation: [PR #1768](https://github.com/teamleaderleo/stensibly/pull/1768)
and [PR #1770](https://github.com/teamleaderleo/stensibly/pull/1770).

## Controls

- Codex CLI `0.151.0`
- model `gpt-5.6-sol`, reasoning `low`
- fresh ephemeral sessions
- closed-task prompt-surface profile
- same schema and task within each pair
- only the operating-policy block changed
- raw prompts/events remain in the private workstation lane
- quiet losing result retained

Private receipt SHA-256:
`f9d4f5b34021b05b79c118723fcaf5af648a0e71c7c7f646f6bf768db7eba89e`.

This result does not authorize automatic policy deferral or a global presentation
change.
