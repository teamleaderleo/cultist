# cmux review applicability

## Question

Can an exact-source review receipt be reused safely after the candidate changes?

The first answer stays deliberately narrow:

```text
current revision == reviewed revision
  -> reuse_exact_review

current revision != reviewed revision
  -> refresh_review

current revision missing
  -> need_current_revision / UNKNOWN
```

This uses Cultist's existing applicability evaluator. It does not claim that every historical claim becomes useless when the patch moves. It says the **review disposition for the exact reviewed patch** is no longer current.

That distinction leaves room for later per-claim reuse:

- prior-head evidence remains historical evidence;
- deterministic facts may be reacquired or revalidated cheaply;
- review-memory can recover the prior concern/outcome;
- a new exact-head review decides which concerns survive.

## Repair control

A repaired finding makes this distinction concrete.

The original receipt is bound to source A. Its repair carries `after_source=B` plus post-repair verification. If the current candidate is B, the original review disposition is INVALID for B and the next action is a fresh review.

Passing the old discriminator on B is useful repair evidence. It is not a complete review of B.

## Example

```bash
cargo run --example cmux_review_applicability < request.json
```

The request wraps one review receipt plus a Cultist `EvaluationContext`. The first experiment requires only `current.revision`; richer repository/work/scope applicability belongs to later evidence once the orchestration receipt carries stable canonical coordinates for them.
