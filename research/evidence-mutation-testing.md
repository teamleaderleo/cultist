# Evidence mutation testing

Tracking: #146. Builds on the merged #125 evidence-role projection fixture, #123/#124 applicability semantics, #131 exact report fingerprint, #157 behavioral receipts, and #165 behavioral episode identity.

## Question

When Cultist intentionally omits, misbinds, or changes evidence-bearing state in a terse/JEI/review projection, can adversarial mutation tests show which owning semantic contract changed?

The carrier reuses independent deterministic evaluators instead of inventing one universal decision language.

## Oracle 1: evidence-role next action

The merged role-projection fixture models:

```text
support
counterexample
limit
clearing
```

and derives one test-local `NextAction`:

```text
proceed
hold
restrict_scope
reconcile_exception
execute_clearing_step
```

V0 mutations remove one evidence role and compare the canonical and mutated actions.

Observed controls:

```text
support-only omission
  Proceed -> Proceed
  survived this fixture/action oracle

limit omission
  RestrictScope -> Proceed
  killed

counterexample omission
  ReconcileException -> Proceed
  killed

clearing omission
  ExecuteClearingStep -> Hold
  killed
```

One adversarial fixture carries both counterexample and limit evidence. The action oracle prioritizes the limit, so dropping the counterexample leaves `RestrictScope -> RestrictScope`. That mutation survives the immediate-action oracle while still losing a material exception receipt.

This keeps the key boundary explicit:

> `survived` describes one fixture under one oracle. It never grants canonical deletion of the omitted evidence.

## Oracle 2: exact applicability

The second control reuses the shared #123/#124 applicability evaluator directly. Evidence in the fixture requires exact revision `head-a`; repository/work coordinates are present but unrequired.

```text
move required revision head-a -> head-b
  applies -> invalid
  killed

drop required revision from current context
  applies -> unknown
  killed

move repository coordinate the evidence did not require
  applies -> applies
  survived this applicability oracle
```

A mutation is evaluated against the coordinates the evidence actually declared. Extra ambient coordinates do not become hidden applicability requirements.

## Oracle 3: exact report snapshot identity

The third control reuses merged `src/report_fingerprint.rs`, whose production-shaped contract is:

```text
AnalysisReport
-> validated deterministic C1 bytes
-> SHA-256
-> cultist-report-c1-sha256-v1:<digest>
```

The mutation harness deliberately excludes the old positional-delta application fixture from #128. It asks only properties owned by the reusable exact-snapshot fingerprint.

Controls:

```text
pure finding reorder
  fingerprint A -> fingerprint B
  killed

claim semantic mutation
  fingerprint A -> fingerprint B
  killed

pretty-JSON serialize + typed parse
  fingerprint A -> fingerprint A
  survived
```

The representation-only control is important. JSON whitespace/layout can change while the typed `AnalysisReport` and its C1 fingerprint remain identical. Conversely, finding order belongs to exact snapshot identity even when the finding semantics themselves are individually unchanged.

This oracle proves exact snapshot identity only. It does not establish semantic lineage across changed reports or apply a positional delta to another base.

## Composing oracles

Three independent species now make one aggregate mutation score even less useful:

```text
next action preserved?
applicability preserved?
exact snapshot identity preserved?
material evidence role preserved?
semantic lineage preserved?
reopen/clearing condition preserved?
```

A mutation may survive one oracle and fail another. Every mutation receipt stays tied to the evaluator that owns its verdict.

## Relationship to behavioral receipts

Merged #157 records observed worker outcomes such as `changed_next_action`, `needed_stronger_evidence`, `stale_or_wrong_coordinate`, and `correct_quiet_negative`. Merged #165 wraps those receipts in stable episode identity.

Keep deterministic mutation identity and behavioral episode identity separate:

```text
mutation kind/id
  semantic edit applied by the research harness

episode_id
  real receiver observation
```

A future A/B replay may join the two by explicit episode ID. Deterministic fixtures never fabricate behavioral receipts.

## Earlier executed receipts

The first two-oracle carrier passed on rebased head:

```text
9b1fcde5ac278ca87c64a32d9e2f0e8cb53614e0
```

CI run `32245648698` / #1163 and generated-provenance run `32245648545` / #198 succeeded.

Main later advanced through behavioral corpus, project-memory collectors/admission hardening, JEI budget pilots, explicit scope-history research, and project-memory lineage controls. The original #125 projection fixture remained byte-identical, so the two-oracle carrier was replayed as one commit on main `3db534cfee58da530978c032666f0c1b4f149dfd`.

Replay head:

```text
905c538634cb8d287fc43f1cef94d051755825fc
```

CI run `32249802071` / #1317 and provenance run `32249802194` / #240 passed, including the newer project-memory lineage control.

## Latest-main three-oracle replay

Main then advanced again with #191's explicit scoped agent-context envelope. That landed on:

```text
fe285be0aa9e1a476430b154ed6ec84f77011d32
```

Its landed file set is disjoint from the mutation/fingerprint paths. The mutation carrier was rebuilt as one semantic commit on that exact main and gained Oracle 3 through the reusable report fingerprint module.

Exact compacted semantic head:

```text
6b015047d7651a85aa8d02906fde667a31a64f08
```

GitHub Actions CI run `32250509751` / run number `1349` completed successfully. It passed:

- `cargo fmt --check`;
- strict Clippy;
- project-memory lineage controls;
- active-work preflight;
- full tests including all three mutation oracles;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

Generated provenance review dogfood run `32250509645` / run number `250` also completed successfully on the same head.

The first identity-oracle attempt stopped at rustfmt before Clippy/tests. After the exact formatter delta, the formatted intermediate head passed CI #1346 + provenance #249; the branch was then compacted back to the one semantic commit above and revalidated through CI #1349 + provenance #250.

## Next mutations

Two high-value independent species remain:

1. ambient context binding (#134/#136), once its owning evaluator is reusable on current main;
2. durable clearing/reopen semantics (#144/#159), preferably after the owning research types land or can be composed without copying them into this carrier.

The old positional delta application model remains fixture-local in `tests/delta_identity.rs`. Keep it there until a reusable owner-backed delta/base evaluator exists. Exact report fingerprinting already gives the mutation lane the identity property it can legitimately test today.

## Boundary

- research/test-only;
- no `AnalysisReport` schema change;
- no public terse format change;
- no aggregate mutation score;
- survived mutation is fixture-and-oracle-local evidence;
- exact snapshot identity is separate from semantic lineage;
- behavioral episodes remain optional observations, never the deterministic oracle itself;
- no model dependency.

North star:

> Mutate one semantic contract at a time and make the owning evaluator prove whether the edit preserves the property that consumer actually relies on.
