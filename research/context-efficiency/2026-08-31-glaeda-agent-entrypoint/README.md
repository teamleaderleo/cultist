# Glaeda agent-entrypoint compression

## Result

Move cold rules. Keep the router and universal invariants hot. Replay behavior.

Glaeda candidate `d3f0f67` reduced its automatically loaded `AGENTS.md` from
4,246 to 1,218 `o200k_base` text tokens: **-3,028 (-71.314%)**. A host-mutation
task also loaded the 1,001-token safety reference, for 2,219 tokens total:
**-2,027 (-47.739%)** versus the current-main root.

The change did three things:

- deleted mutable priorities from the automatic entrypoint; current issues own
  them;
- stopped restating the eight commands already owned by
  `./scripts/verify required`;
- moved ownership, persistence, repair, subprocess, and physical-experiment
  detail to one explicit cold reference.

## Controlled replay

Fresh ephemeral `gpt-5.6-sol` low-reasoning sessions used the same read-only
task, schema, host, closed-task prompt-surface profile, and source tree except
for the documentation treatment.

### Ordinary documentation task

Both arms selected bootstrap, scoped README/workflow inspection,
documentation-only verification, and self-review. Command count was noisy across
fresh runs (3 control, 7 treatment). An earlier treatment opened the coordination
reference only because the first router wrongly mapped generic review work to it.
The router was narrowed and replayed. Retain command-count parity as
**inconclusive**, not a compression claim.

### Safety-critical automatic repair

The treatment explicitly opened `docs/AGENT_EXECUTION_SAFETY.md`. Both arms
preserved blockers for exact ownership, atomic durable state, root separation,
empty/allowlisted child environments, no implicit shell, repair budget and
circuit breaker, pre/post observation, journal recovery, rollback/compensation,
physical verification, and independent exact-head acceptance.

| Measure | Control | Treatment | Delta |
| --- | ---: | ---: | ---: |
| provider input tokens | 107,229 | 95,871 | -11,358 (-10.592%) |
| completed commands | 4 | 3 | -1 |
| mixed raw event-log bytes | 921,686 | 30,510 | -891,176 (-96.690%) |

One pair proves this task instance, not universal behavioral equivalence.

## Event projection

The replay reproduced another hotspot: one final structured decision sat inside
a 922 KB mixed Codex event log. `scripts/codex_exec_event_view.py` now
projects that log to:

- 1,227 bytes content-free; or
- 5,059 bytes when the exact final structured result is requested.

It omits warning text, command strings, command output, and intermediate agent
messages while retaining counts, byte totals, digests, event classes, and usage.
Raw logs stay private.

## Research boundary

This treatment follows evidence that natural-language prompts contain large
removable regions, but it does not assume all prose is equally disposable:

- [Yin et al.](https://aclanthology.org/2023.acl-long.172/) found task
  definitions could lose 60% of tokens without worse performance, while output
  and label information mattered most.
- [LLMLingua](https://aclanthology.org/2023.emnlp-main.825/) demonstrated high
  prompt compression with a budgeted, coarse-to-fine selector rather than blind
  truncation.
- [Gist Tokens](https://arxiv.org/abs/2304.08467) targeted repeated prompt
  encoding directly, motivating a small reusable router plus cold expansion.
- [Deng et al.](https://arxiv.org/abs/2412.17483) reported compression failures
  around boundaries and surprising information. That supports task-specific
  replay and explicit cold-reference routing.
- [Hakim](https://arxiv.org/abs/2604.00025) found brevity constraints improved
  accuracy on a particular inverse-scaling subset. Treat that as evidence
  against reflexive verbosity, not a universal instruction to hide reasoning.

Rule: compress repeated agent-facing transmission. Preserve decisions,
invariants, authority, output contracts, unknowns, and recovery. Human-facing
prose keeps the project's normal voice.
