# cmux review applicability

## Question

Can an exact review receipt be reused safely after the candidate or review policy changes?

The first answer uses separate applicability axes.

### Source identity

The reviewed source reuse coordinate is fingerprinted from:

```text
repository_id
base_sha
tree_sha
```

The fingerprint is carried through Cultist's existing exact-revision applicability evaluator.

`tree_sha` is the canonical candidate-content snapshot. `head_sha` stays available as lineage evidence, `working_tree_dirty` remains useful presentation/provenance state, and optional `patch_sha256` can bind retained rendered patch bytes. Those fields do not force a fresh review when base + candidate tree are byte-identical.

Repository identity keeps content-addressed Git objects scoped to the project they came from. This prevents cross-repository reuse as well as a same-HEAD dirty-tree false reuse while also avoiding a redundant review after an identity-only amend/rebase that preserves the exact base and candidate tree.

### Review policy identity

Review protocol and repository rules remain separate coordinates:

```text
policy_version
ruleset_sha256
```

A code-identical candidate reviewed under changed policy/rules requires refresh instead of pretending the previous review covered the new decision inputs.

## Continuity disposition

```text
exact source + policy + ruleset
  -> reuse_exact_review

repository/base/tree source mismatch or policy/ruleset mismatch
  -> refresh_review

current source unavailable
  -> need_current_source / UNKNOWN
```

The historical receipt remains evidence even when its exact disposition is stale.

## Repair control

A repaired finding makes this distinction concrete.

The original receipt is bound to source A. Its repair carries `after_source=B` plus post-repair verification. If the current candidate is B, the original exact-review disposition is INVALID and the next action is a fresh review.

Passing the old discriminator on B is useful repair evidence. It is not a complete review of B.

## Example

```bash
cargo run --example cmux_review_applicability < request.json
```

The request carries the historical receipt plus the current source and current review-policy identity.

Controls require:

- different repository identity -> refresh;
- same HEAD + different candidate tree -> refresh;
- different HEAD + same base/tree -> reuse;
- different patch digest + same base/tree -> reuse of the semantic review disposition;
- policy/ruleset drift -> refresh.

## Next discriminator

A later composition can use review memory to decide which **individual prior-head concerns** deserve refresh, reuse as historical context, or quiet dismissal. This experiment only governs reuse of the exact whole-review disposition.
