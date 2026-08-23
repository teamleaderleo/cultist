# Selected refinement observation acquisition loop

Tracking: #234. Current-main composition uses #179, #231, #210, #216, and the repaired #339 edit-class source. Historical #221 established the source species; #339 repaired its current-repository applicability boundary.

## Question

Can one retained selected refinement drive its exact source investigation all the way from candidate requirement to current observation?

The retained carrier is Oxc:

```text
episode   history/oxc-edit-class-v1
candidate syntax-changing-current-cohort
D         edit_class
S         oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:
          crates/oxc_linter/src/rules.rs
```

## Current-main composition

```text
#179 selected candidate
-> #231 candidate/discriminator -> exact subject mapping
-> #210 exact ObservationRequirement
-> withhold exact observation
-> #210 MISSING frontier
-> #339 source bridge + read-only rust_edit_class probe
-> #216 existing planner SELECTED
-> #339 focused Rust source executes against explicitly bound current repository
-> v2 KNOWN syntax_changed + APPLIES observation
-> #210 exact frontier CURRENT
```

The selected Oxc subject comes from the retained #231 source mapping. The public carrier binds both source repository coordinates explicitly:

```text
current repository = oxc-project/oxc
subject repository = oxc-project/oxc
```

The source still observes the current checkout revision itself. The workflow checks the applicability receipt names `current=oxc-project/oxc@...`, so the #339 repository discriminator is exercised rather than satisfied only by argument count.

## Network-free control

`tests/selected_refinement_observation_loop.rs` compiles the selected Oxc D@S requirement and combines the broad current observation corpus with the dedicated focused fixture:

```text
research/refinement-observation-requirements/oxc-focused-edit-class-v1.json
```

The broad corpus is deliberately not required to retain the exact selected Oxc observation. The focused fixture supplies the stable exact-control species used to construct two other-subject controls:

```text
same focused revision, wrong Rust path
pinned repository head 8783524..., same path, UNKNOWN anchor-unchanged
```

With the exact focused observation withheld, the selected requirement must be:

```text
MISSING
current = 0
other_subject = 2
```

Adding the exact focused control changes only that frontier to:

```text
CURRENT
value_ref = syntax_changed
other_subject = 2
```

This protects the subject boundary in ordinary CI without network access.

## Public Oxc carrier

The GitHub-hosted workflow builds independent readers for requirement compilation, frontier evaluation, probe planning, and the #339 Rust edit-class source. It then:

1. compiles the retained #179 + #231 data and extracts the exact Oxc requirement;
2. removes broad-corpus edit-class rows and constructs wrong-path plus pinned-head controls from the focused fixture;
3. requires the exact frontier MISSING with two other-subject controls;
4. checks out pinned Oxc history `8783524015b1e6ff1c39ccf426df0bb07cbbc588`;
5. requires `228e8e0f85c0e7aeded02c5e27fd810004d3b41a` to remain the latest non-merge `rules.rs` change inside that pinned history;
6. runs the pinned-head #339 source control and requires value UNKNOWN `anchor-unchanged` with applicability APPLIES at explicitly bound current repository `oxc-project/oxc`;
7. checks out the exact focused commit and runs #339 again;
8. requires the live source observation to name the exact selected subject, produce KNOWN `syntax_changed`, and carry applicability APPLIES with the bound current repository in its receipt;
9. sends the still-MISSING frontier plus source bridge/probe through #216 and requires the read-only probe to be SELECTED;
10. appends the live produced observation and requires the exact #210 frontier CURRENT while both other-subject controls remain visible.

The planner receipt deliberately keeps:

```text
frontier_status = missing
```

Planning alone cannot promote the observation. Only the later source observation changes currentness.

## Current-main counterexample retained

The first #339-compatible current-main public replay was GitHub Actions run `32660178002`, job `97245015391`. Requirement compilation succeeded, then the workflow failed in its withheld-observation setup with Python `StopIteration` because `research/discriminator-observations/cultist-v1.json` no longer retained the exact focused Oxc observation.

That failure proved the historical carrier had an accidental corpus-retention dependency. The repaired carrier reads the exact control from the dedicated focused fixture instead. The live public transition to CURRENT still uses the observation produced by #339, not the fixture row.

The first repaired semantic head `4644cd475bbde24acffd044fd2b212db06aa979e` passed the dedicated public carrier in run `32660343188` and generated provenance in run `32660343233`.

After refreshing this durable note, exact head `57f7a1abf56b23a9dc8af426342cee3dfe29a0be` passed the complete current-main certification pair:

```text
ordinary CI
  run: 32660467513
  result: success

selected-refinement public Oxc carrier
  run: 32660467451
  result: success

generated provenance
  run: 32660467453
  result: success
```

Ordinary CI passed format, strict Clippy, the population-aware active-work advisory, full tests, and every repository/history/CI-filter/diff dogfood step. The public carrier independently proved the full MISSING -> selected read-only investigation -> live #339 KNOWN+APPLIES -> CURRENT transition against pinned Oxc history.

## Failure conditions

The carrier fails if:

- the selected candidate changes;
- the #231 mapping changes away from the exact Oxc subject;
- the focused fixture no longer names that exact subject;
- the pinned historical window no longer contains the focused commit as the latest `rules.rs` change;
- the current repository bound to #339 differs from the intended Oxc checkout;
- the source adapter bridge/observation names another subject;
- the focused classifier stops producing `syntax_changed`;
- the planner cannot select the exact read-only probe;
- another-subject observations satisfy the exact selected requirement;
- planning is mistaken for current evidence.

## Boundary

- composition/research carrier only;
- no new shared state model or second Rust syntax classifier;
- no refinement promotion or ranking;
- no effect authority, ownership, scheduling, merge, or mutation semantics;
- no score across candidates or sources;
- `syntax_changed` is only the observed edit-class partition, not a higher-level product-correctness claim;
- public network work remains pinned and GitHub-hosted.

Historical #221/#235 receipts remain useful lineage, while current certification authority belongs to the #339-compatible current-main head and its hosted runs.

North star:

> Start from the selected refinement's exact evidence requirement, perform only the admitted investigation, and make currentness depend on the resulting source observation.
