# Rust edit-class observation acquisition loop

Tracking: #220, building on #54, #185, #210, and #216.

## Question

Can one retained analyzer family complete the full deterministic loop:

```text
noncurrent source discriminator
-> explicit source bridge
-> existing evidence planner
-> source probe execution
-> v2 current observation
-> current frontier
```

without copying source semantics into the generic bridge or planner?

The first carrier uses the Oxc `edit_class` discriminator because #54 already earned a deterministic producer: select a focused non-merge commit that changes one Rust anchor, compare that anchor with its first parent, tokenize both versions with `proc_macro2`, recursively remove doc attributes, and classify whether lexical Rust syntax changed.

## Reuse boundary

`examples/rust_syntax_cohort.rs` remains the owning classifier. This carrier changes only visibility of:

```text
RustEditClass
classify_rust_edit(...)
```

so the source adapter can call that exact implementation. The token/fingerprint/classification logic is unchanged.

## Focused-commit admission

The first public carrier exposed an important precondition that had been implicit in #54.

#54 never called the classifier on arbitrary repository heads. It first selected commits from:

```text
git log --no-merges -- anchor.rs
```

then classified each selected commit against its first parent.

Calling the raw token comparator at pinned Oxc repository head:

```text
8783524015b1e6ff1c39ccf426df0bb07cbbc588
```

produced equal `rules.rs` token streams because that repository commit does not change `crates/oxc_linter/src/rules.rs`. Without an explicit focus check, unchanged-anchor equality was indistinguishable from a comment/docs/whitespace-only edit.

The source adapter now admits a value only when:

```text
revision has exactly one parent
+ exact anchor path changed in that revision
```

Otherwise it emits value UNKNOWN with one of:

```text
rust-edit-class:not-single-parent:...
rust-edit-class:anchor-unchanged:...
```

This keeps repository/head applicability separate from whether an `edit_class` observation exists for the anchor at that commit.

## Source contract

`src/rust_edit_class_source.rs` owns one source-specific record:

```text
RustEditClassSubject
  repository
  exact 40-hex revision
  normalized repository-relative .rs path
```

Collection emits:

```text
RustEditClassSourceResult
  subject
  current_head
  bridge
  probe
  observation
```

### Bridge

The adapter explicitly maps:

```text
edit_class @ repository@revision:path
-> probe kind rust_edit_class
-> target same exact subject ref
-> repository + revision + exact-path clearing requirements
```

The bridge carries a source receipt. No generic string equality convention is introduced.

### Probe

The admitted probe is read-only and forecasts the source work performed by the focused first-parent classifier:

```text
git subprocesses: 5
Rust files parsed: 2
remote requests: 0
effectful executions: 0
```

The existing #145 planner remains responsible for capability, clearing-coordinate applicability, cost, and effect authority.

### Observation

For an admitted focused commit, the adapter maps only the existing classifier result:

```text
SyntaxChanged
  -> KNOWN syntax_changed

CommentsOrWhitespaceOnly
  -> KNOWN comments_or_docs_only

Unclassified
  -> UNKNOWN with exact source reason receipt
```

A root/merge/unrelated-anchor revision stays UNKNOWN before this mapping.

Current applicability is evaluated through the shared #123 evaluator against:

```text
required
  repository
  exact revision
  exact file path

current
  repository label
  git rev-parse HEAD
  exact file path
```

A moved checkout can therefore preserve an old known edit class while applicability becomes INVALID. It cannot claim the old classification is current.

Every emitted observation is revalidated through the v2 #210/#185 observation batch contract before return.

## Local deterministic controls

`tests/rust_edit_class_source.rs` creates temporary local Git repositories and exercises the actual retained classifier:

1. lexical Rust code change -> KNOWN `syntax_changed` + APPLIES;
2. comment-only change -> KNOWN `comments_or_docs_only` + APPLIES;
3. root commit with no single parent -> value UNKNOWN + APPLIES;
4. unrelated repository commit that leaves the anchor unchanged -> value UNKNOWN `anchor-unchanged` + APPLIES;
5. previously known syntax edit after HEAD advances -> KNOWN old value + INVALID;
6. exact source bridge/probe is planned by #216/#145;
7. inserting the produced observation changes the exact v2 frontier from MISSING to CURRENT;
8. emitted observation round-trips through v2 validation;
9. traversing/non-Rust paths and non-exact revisions fail closed.

## Retained public Oxc carrier

The workflow pins the historical repository window at:

```text
oxc-project/oxc@8783524015b1e6ff1c39ccf426df0bb07cbbc588
anchor: crates/oxc_linter/src/rules.rs
```

It keeps that repository head as the negative control:

```text
8783524...
anchor unchanged
-> value UNKNOWN / anchor-unchanged
-> applicability APPLIES to the exact repo/head/path coordinate
```

Within that pinned history, the workflow resolves the latest actual focused anchor commit through the same admission species as #54:

```text
228e8e0f85c0e7aeded02c5e27fd810004d3b41a
fix(linter): resolve inactive React compiler rules (#25830)
```

The workflow then executes three independent readers in sequence on that exact focused commit:

```text
rust_edit_class_observation
  -> source bridge + probe + KNOWN syntax_changed observation

observation_probe_plan
  -> MISSING frontier + exact source bridge
  -> selected read-only probe

observation_frontiers
  -> produced observation
  -> CURRENT edit_class frontier
```

The resulting source subject is:

```text
oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs
```

This corrects the inherited #185 retained observation coordinate. The pinned repository head remains the historical cohort window and explicit unchanged-anchor negative control; the actual `edit_class=syntax_changed` observation belongs to the focused file-touching commit.

## Boundary

- research only;
- Rust lexical syntax class is not behavioral equivalence;
- edit class does not prove generator ownership or regeneration intent;
- source observation values grant no evidence strength or action authority;
- the generic #216 bridge/planner/frontier remain unchanged;
- source execution produces evidence, not a claim that any higher-level refinement should be promoted;
- the public carrier preserves both exact repository-window and exact focused-commit coordinates.

## Execution receipt

The first public carrier run `32257281636` / run number `1` compiled all readers, collected a source result, selected the bridge probe, and fed the result to a CURRENT frontier. Its final assertion failed because arbitrary pinned repository head `8783524...` emitted `comments_or_docs_only`; inspection showed the anchor did not change in that commit. This failure produced the focused-commit admission repair above.

After adding explicit single-parent + anchor-changed admission, public carrier run:

```text
GitHub Actions run: 32257998359
workflow run number: 7
result: success
```

proved both controls:

```text
pinned repository head 8783524...
  -> UNKNOWN anchor-unchanged

focused rules.rs commit 228e8e0f85c0...
  -> KNOWN syntax_changed
  -> applicability APPLIES
  -> exact read-only probe selected
  -> final frontier CURRENT
```

The emitted focused observation, bridge, selected probe, and final frontier all use the exact `228e8e0f...:rules.rs` subject coordinate.

After correcting the inherited #185 observation corpus and recording the receipt, the branch's current semantic state passed:

```text
ordinary CI
  run: 32258258665
  run number: 1533
  result: success

public Oxc carrier
  run: 32258258680
  run number: 9
  result: success
```

Ordinary CI passed format, strict Clippy, active-work preflight, the full local Git/source/bridge/frontier suite, and repository/history/CI-filter/diff dogfood. The public carrier again reproduced the pinned-head UNKNOWN control and focused-commit `syntax_changed -> selected probe -> CURRENT frontier` loop.

North star:

> Let one real source discriminator complete the investigation loop while every handoff remains typed, exact-coordinate, and replayable.
