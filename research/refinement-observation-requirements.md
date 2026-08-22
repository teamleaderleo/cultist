# Exact refinement observation requirements

Tracking: #227. Builds on #179, #185, #210, #221, and the green wrong-subject control in #228.

## Trigger

#228 proved an executable mismatch between two already-earned semantics.

The Oxc selected refinement says only:

```text
discriminator_refs = ["edit_class"]
```

When its exact current observation is replaced by a KNOWN+APPLIES `edit_class` for another subject, the existing #187-style ID-only coverage still says the selected candidate has current `edit_class` evidence.

The exact #210 frontier for the earned Oxc subject simultaneously says:

```text
MISSING
other_subject = [wrong edit_class observation]
```

The refinement candidate therefore needs an explicit way to say which source observation requirement applies to this candidate/application.

## Why the subject stays outside the discriminator definition

The Oxc `edit_class` discriminator is a reusable classifier, not a fact with one intrinsic subject. The same discriminator appears in both the selected forward cohort candidate and the rejected reverse control, and the #54 source producer classifies many different commits.

Adding one subject directly to the admitted discriminator would conflate:

```text
what classifier/refinement dimension exists
```

with:

```text
which exact observation subject this candidate currently needs
```

V0 keeps those separate.

## Source-owned mapping

`src/refinement_observation_requirement.rs` adds one bounded receipt:

```text
RefinementObservationRequirementMapping
  id
  episode_id
  candidate_id
  discriminator_id
  subject_ref
  source_receipt
```

The tuple:

```text
(episode_id, candidate_id, discriminator_id)
```

may have at most one v0 subject mapping. The source receipt owns the claim that this exact candidate/discriminator needs this exact observation subject.

The mapping must reference:

- an existing refinement episode;
- an existing candidate in that episode;
- a discriminator actually named by that candidate.

No subject is inferred from the current observation corpus.

## Selected requirement compilation

For each episode with a selected transition, the evaluator resolves the selected candidate's discriminator refs through the source-owned mappings and emits ordinary #210:

```text
ObservationRequirement
  discriminator_id
  subject_ref
```

It also preserves the exact mapping receipts used.

Missing mapping stays explicit:

```text
missing_discriminator_refs[]
```

An empty mapping set therefore produces zero resolved requirements and exposes every selected discriminator ref as missing mapping evidence. The evaluator never searches the observation corpus for a convenient current subject.

## Retained three-family mappings

The first corpus covers every selected #179 candidate.

### Justification

```text
justification/open-obligation-v1
allow-open-zero-edge
clearing_evidence_presence
-> refinement:justification/open-obligation-v1
```

### Oxc

```text
history/oxc-edit-class-v1
syntax-changing-current-cohort
edit_class
-> oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs
```

This is the exact focused source observation earned by #221.

### Project memory

```text
project-memory/primary-case-contract-collision-v1
split-primary-case-admission
primary_case_evidence_form
same_repository_issue_target
-> refinement:project-memory/primary-case-contract-collision-v1
```

The two discriminator requirements share the same episode-local subject while remaining separate exact requirements.

## Controls

The standard test carrier requires:

1. all three selected candidates compile with zero missing mappings;
2. every compiled requirement is CURRENT in the retained v2 observation corpus;
3. wrong-subject Oxc `edit_class` leaves the compiled exact requirement MISSING;
4. removing the Oxc mapping exposes `edit_class` under `missing_discriminator_refs` even when observations exist elsewhere;
5. a mapping for a discriminator absent from the candidate rejects;
6. multiple subject mappings for one exact episode/candidate/discriminator reject;
7. the same reusable discriminator may have a different mapping for another candidate without changing the selected candidate mapping;
8. request round-trip and a 512 KiB input bound are explicit;
9. an empty mapping batch is valid and exposes every selected requirement as unresolved mapping evidence.

## Reader

```text
cargo run --example refinement_observation_requirements < request.json
```

The request carries the validated refinement episode batch plus the source-owned mapping batch. The reader revalidates both before compiling selected exact requirements.

## Boundary

- research only;
- #179 replay ledger schema remains unchanged;
- mapping subject identity grants no observation value, evidence strength, or authority;
- #210 frontier currentness remains unchanged;
- #216 probe mapping remains a later acquisition step after an exact observation requirement exists;
- no implicit first/current observation selection;
- no automatic refinement promotion.

## Execution receipt

The first CI attempt stopped at `cargo fmt --check`; the formatter delta changed wrapping/order only. The next run reached Clippy and exposed a standalone-reader dependency: `observation_frontier.rs` imports `discriminator_observation`, so the research example needed that sibling module in its crate root. Adding that import changed no mapping semantics.

Formatted semantic head:

```text
95f614c079de09358f1865bab34399235a512844
```

GitHub Actions CI run `32259965516` / run number `1597` completed successfully. It passed:

- `cargo fmt --check`;
- strict Clippy;
- active-work preflight;
- full tests, including all three retained selected-family requirement mappings;
- exact retained D@S requirements CURRENT;
- wrong-subject Oxc edit class MISSING with `other_subject` receipt;
- missing mapping explicit despite other current observations;
- duplicate/invalid mapping controls;
- deterministic round-trip and input bound;
- repository/history/CI-filter/diff dogfood.

The result preserves the chosen boundary: refinement candidates keep reusable discriminator IDs, while source-owned candidate/application receipts bind those IDs to exact observation subjects before currentness or acquisition is evaluated.

North star:

> Let the refinement ledger say which discriminator a candidate uses, and let explicit source evidence say which exact observation subject this candidate currently requires.
