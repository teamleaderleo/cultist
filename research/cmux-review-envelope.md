# cmux review-envelope projection

## Status

Research bridge over the current review-receipt experiment.

This lane asks whether an orchestration-owned review receipt can feed Cultist's existing evidence semantics without turning Cultist into an agent runner or teaching cmux a second provenance vocabulary.

## Boundary

cmux owns execution concerns such as reviewer spawning, session isolation, verification commands, repair worktrees/checkpoints, and the developer-facing review workflow.

Cultist consumes the resulting bounded receipt as evidence.

The first projector therefore does three things only:

1. validates exact source identity and receipt consistency;
2. preserves the shared `PROVEN / DERIVED / OBSERVED / INFERRED / UNKNOWN` claim kinds;
3. projects human-attention findings, quiet findings, and an explicit review frontier.

It grants no repair, merge, or approval authority.

## Example

```bash
cargo run --example cmux_review_envelope < receipt.json
```

The output keeps:

- exact base/head and diff identity;
- separate review protocol/ruleset identity;
- task intent and requirement status;
- non-refuted/non-suppressed findings for attention;
- provenance-bearing claims and verification state;
- quiet refuted/suppressed findings;
- `UNKNOWN`, blocked verification, uncertain challenge, human-required, and unresolved states in the frontier;
- declared coverage gaps.

## Semantic controls

The projector fails closed on several inconsistent states:

- abbreviated or malformed source identities;
- summary counts that disagree with retained findings;
- a refuted final disposition without a refuting challenger;
- a surviving finding after explicit challenger refutation;
- reproduced/static-supported verification without at least one `PROVEN` or `DERIVED` claim;
- a repaired disposition without an attempted fixed repair;
- a repaired disposition without a distinct exact resulting source;
- a repaired disposition without passed, evidence-bearing post-repair verification;
- a human-required disposition with no retained uncertainty or blocked/human verification state.

Source applicability and review-policy applicability remain separate. The source object contains only code-state identity; `policy_version` and top-level `ruleset_sha256` describe the review process/rules that produced the receipt.

A repaired finding explicitly carries both states: the top-level receipt source is the pre-repair review coordinate, while `repair.after_source` and `repair.verification` bind the repair result and replay evidence to the resulting coordinate.

These checks are intentionally narrower than a universal review ontology. They test whether the current orchestration receipt preserves enough evidence for Cultist to reason about attention and later applicability.

## Next discriminator

The next useful experiment is revision applicability:

```text
review receipt at head A
-> candidate moves to head B
-> which claims remain applicable?
-> which findings need refresh?
-> which exact verification receipts survive?
```

That should reuse Cultist's existing applicability and review-memory primitives instead of adding a cmux-specific freshness model.

A second follow-up can use behavioral receipts to measure whether surfaced review items changed the next justified action while quiet findings stayed quiet.
