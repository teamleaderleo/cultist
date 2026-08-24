# Refinement candidate evidence readiness

Tracking: #240. Builds on #179 replay bookkeeping, #231 exact candidate observation requirements, #210 frontier currentness, and the complete selected-candidate acquisition replay in #235.

## Question

Can one descriptive view show both:

```text
replay verdict
current exact evidence readiness
```

without letting either axis overwrite the other?

The trigger is now concrete. #235 proved that the retained selected Oxc candidate can acquire a missing exact `edit_class` observation and move its exact frontier from MISSING to CURRENT. #179 simultaneously preserves competing Oxc candidates that were rejected by replay for different reasons.

Current evidence availability therefore answers a different question from replay quality.

## V0 receipt

`src/refinement_candidate_readiness.rs` evaluates every retained candidate and emits:

```text
RefinementCandidateReadiness
  episode_id
  candidate_id
  is_selected_transition
  replay_status
  replay_result
  evidence_status
    current
    blocked
  requirements[]
  requirement_mappings[]
  requirement_frontiers[]
  missing_requirement_mappings[]
```

`evidence_status` is deliberately evidence-only. It says whether every exact mapped observation requirement is currently KNOWN+APPLIES. It grants no candidate selection, replay revision, promotion, policy, or execution authority.

The full replay status/result from #179 remains present beside it.

## Reuse boundary

The readiness evaluator does not invent a second mapping validator. Before reading candidate mappings it constructs the existing #231 request and calls the existing selected-requirement evaluator, which validates:

```text
mapping schema
mapping ID uniqueness
(episode, candidate, discriminator) uniqueness
existing episode/candidate references
candidate discriminator membership
```

After that validation, readiness reads the same exact tuple:

```text
(episode_id, candidate_id, discriminator_id)
-> subject_ref + source receipt
```

for each candidate.

Every resolved requirement is passed unchanged to #210 `evaluate_observation_frontiers`. Current evidence means every required frontier is `CURRENT` and no requirement mapping is missing.

## Retained Oxc asymmetric controls

### Selected replay survivor + current evidence

```text
candidate  syntax-changing-current-cohort
replay     weakened
evidence   current
selected   true
```

This uses the exact #221/#231 focused subject:

```text
edit_class @ oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:
             crates/oxc_linter/src/rules.rs
```

### Replay survivor + missing exact evidence

The test removes that exact observation and inserts a current `edit_class=syntax_changed` observation for the same revision but `other.rs`.

Expected and executed:

```text
replay     weakened
evidence   blocked
frontier   missing
other_subject = 1
```

The replay survivor remains a survivor. Current use is blocked by evidence identity/currentness.

### Replay rejected + current evidence

The test supplies explicit candidate-specific mappings and current observations for:

```text
reverse-edit-class-control
  replay = rejected_no_improvement
  evidence = current

singleton-commit-partition
  replay = rejected_overfit
  evidence = current
```

These are intentionally synthetic readiness controls. The exact mapping/observation facts are supplied explicitly; no source claim is inferred from the candidate name.

The executed result protects the key direction:

> Perfect current evidence cannot rescue a candidate that deterministic replay already rejected.

### Missing mapping

Removing the selected Oxc candidate's #231 mapping yields:

```text
replay     weakened
evidence   blocked
requirements = []
missing_requirement_mappings = [edit_class]
```

The evaluator does not search the observation corpus for a convenient subject.

## Default retained corpus

The retained #231 mapping corpus maps only selected transitions. As a result, the two rejected Oxc candidates are evidence-blocked by missing candidate-specific mappings in the unmodified retained request.

That is descriptive: their replay rejection already explains why no acquisition path was needed. A caller can still supply an exact mapping/current observation for a rejected candidate when testing the independence of the two axes.

## Reader

```text
cargo run --example refinement_candidate_readiness < request.json
```

The request carries:

```text
schema_version = 1
refinements     #179 batch
mappings        #231 batch
observations    #185/#210 v2 batch
```

Input is bounded to 1 MiB before parsing. The underlying refinement, mapping, and observation validators remain authoritative for their own records.

## Boundary

- research only;
- no automatic candidate selection;
- no ranking or confidence score;
- no promotion authority;
- evidence currentness never changes `replay_status`;
- replay status never fabricates evidence;
- rejected candidates stay rejected with current evidence;
- replay survivors may stay evidence-blocked;
- wrong-subject observations stay `other_subject` through #210;
- source acquisition remains #216/#221/#235 and is outside this view;
- no product CLI/report-schema change.

## Execution receipt

The first three CI attempts exercised only carrier hygiene:

```text
#1689 run 32265386776
  rustfmt only

#1708 run 32265609939
  rustfmt passed
  Clippy found one unused test import

#1720 run 32265840673
  final import cleanup needed one rustfmt wrap
```

No semantic assertion ran before those issues were corrected.

Formatted semantic head:

```text
9b37d4d54f895442b26947151331f2e7ac2de459
```

passed full ordinary CI:

```text
run:        32266090296
run number: 1727
result:     success
```

The branch was then compacted to one semantic commit on #235:

```text
7fe8e00dbe080e85b896442a2ce3dfec189c4dfb
```

That exact compacted head passed full ordinary CI again:

```text
run:        32266336188
run number: 1741
result:     success
```

Both green runs passed format, strict Clippy, active-work preflight, the full candidate-readiness test suite, and repository/history/CI-filter/diff dogfood.

The executable result keeps both axes independent:

```text
selected syntax-changing-current-cohort
  replay   weakened
  evidence current

same candidate, exact observation withheld + wrong-path current control
  replay   weakened
  evidence blocked
  frontier missing

reverse-edit-class-control with exact synthetic current evidence
  replay   rejected_no_improvement
  evidence current

singleton-commit-partition with exact synthetic current evidence
  replay   rejected_overfit
  evidence current

selected candidate with exact mapping removed
  replay   weakened
  evidence blocked
  missing mapping edit_class
```

Evidence currentness therefore neither rescues a replay-rejected candidate nor supplies missing evidence to a replay-surviving candidate.

North star:

> Keep “did this refinement survive replay?” separate from “is the exact evidence it needs usable right now?”
