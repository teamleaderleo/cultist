# External repository dogfood

Cultist's larger evaluation corpus should live outside ordinary CI. The normal product loop stays small; external repositories are an on-demand fieldwork lane.

## Goal

Make another repository cheap to inspect before Cultist has any installation or configuration inside that repository.

The first harness supports:

```text
current Rust syntax/convention scan
current CI test-filter inventory
optional repeated scan for warm-cache measurement
optional bounded history for one explicitly named file
optional diff against one explicitly named base revision
```

Every Cultist invocation enables `CARGO_CULTIST_PERF=1` and preserves the JSON output, stderr, and performance receipt in one artifact directory.

## Safety boundary

The runner treats the target checkout as evidence.

It does not invoke target `cargo`, build scripts, tests, generators, package managers, or repository-provided commands. It writes Cultist caches and receipts outside the target checkout when the caller provides `--cache-dir` and `--output-dir` outside that checkout.

The GitHub workflow has read-only `contents` permission and is `workflow_dispatch` only. It therefore adds no network-heavy external corpus work to ordinary push or pull-request CI.

The durable workflow currently targets public repositories. A future private-repository adapter should use an explicit credential boundary instead of silently widening token authority.

## Progressive history cost

Repository history is a reservoir of evidence, not a mandatory startup cost.

The manual workflow defaults to a target checkout depth of 256 commits. A shallow checkout is recorded in `summary.json`; history results from that checkout should be read with that boundary in mind. Set checkout depth to `0` only when the replay actually needs complete history.

The history probe is additionally gated by:

- an explicitly supplied repository-relative file;
- a maximum commit count between 1 and 1000;
- Cultist's existing non-merge and broad-commit cohort rules.

This gives us a useful progression:

```text
new repository
  -> current scan + CI inventory

interesting target / current task
  -> bounded file history

specific change replay
  -> explicit base + diff

history question survives those gates
  -> deeper/full checkout deliberately
```

A bounded diff replay admits both sides explicitly. If a base is supplied, the workflow fetches the same bounded ancestry for that base before Cultist runs. The first Cloud Hypervisor carrier exposed why: a shallow exact-head checkout can have ample head history while lacking a newer base tip, and `git merge-base` should refuse that incomplete object set.

The aim is to learn from large histories without making every edit-loop invocation pay for them.

## Local use

Build Cultist once, then point the harness at any existing checkout:

```text
cargo build --release
python scripts/external_dogfood.py \
  --cultist target/release/cargo-cultist \
  --repo /path/to/other/repo \
  --output-dir /tmp/cultist-dogfood \
  --cache-dir /tmp/cultist-cache \
  --repeat-scan
```

Add one bounded history target when there is a reason:

```text
python scripts/external_dogfood.py \
  --cultist target/release/cargo-cultist \
  --repo /path/to/other/repo \
  --output-dir /tmp/cultist-dogfood \
  --history-file src/example.rs \
  --history-max 100
```

A specific change can add `--base REV` and run the existing diff analyzer against the checked-out target. The local caller is responsible for making that base object and enough ancestry available in the checkout.

## Pinned corpus registry

`research/external-dogfood-cases.json` is the durable queue of external questions.

Each case has one of three statuses:

```text
replayable
  exact repository/revision coordinates + today's Cultist evidence can run the question

adapter_gap
  exact case is pinned, but today's product does not ingest the evidence needed for the interesting question

needs_pin
  useful historical story still needs an exact revision/evidence coordinate before replay
```

Registered replay cases use exact 40-hex revisions. PR and issue numbers remain provenance labels rather than executable identity.

`scripts/external_dogfood_case.py` validates the registry, refuses non-replayable cases, and resolves a selected case into the manual workflow's normalized checkout/probe inputs. The same resolver validates ad hoc public-repository inputs when no case ID is supplied.

The registry is intentionally small metadata. Adding fifty historical questions should add roughly fifty descriptors, not fifty default CI jobs or fifty repository clones.

## First external carrier: Glaeda historical corpus

Glaeda is the current product name. This section keeps SmolRunner where the pinned repository/corpus identity belongs to the historical evidence.

Issue #62 already established a useful pinned history replay from Glaeda's SmolRunner-era corpus at:

```text
teamleaderleo/smolrunner@ed3b70e375a57eabce26f2311f798f75b33bdeb0
src/disposable_clone_runtime.rs
```

That target is a good first carrier because it has known earned-history discriminators and counterexamples. An exact SHA should be used whenever a reproducible historical replay is desired.

### Executed receipt

PR #129 ran the temporary carrier against that exact SmolRunner-era coordinate:

```text
workflow run: 32240366281
job:          96029365880
artifact:     9360596386
sha256:       9794b9958627cb1b88d5f347496a1fc76b720d789ab8f66f1a48067989f2bf2b
checkout:     shallow, depth 256
```

All four probes and the safety discriminator passed.

Observed Cultist work receipts:

```text
scan
  findings:          4
  git subprocesses:  4
  Rust files parsed: 259
  cache hits:         0
  wall time:          591307 us

scan-warm
  findings:          4
  git subprocesses:  4
  Rust files parsed: 0
  cache hits:         259
  wall time:          31640 us

ci-tests
  findings:          0
  git subprocesses:  1
  Rust files parsed: 0
  cache hits:         0
  wall time:          1197 us

history
  discovered:         14 commits
  considered:         14 commits
  git subprocesses:   2
  Rust files parsed:  0
  wall time:           9842 us
```

The history replay recovered the same strongest raw companion pattern recorded in #62:

```text
docs/DISPOSABLE_AUTOSCALING_CI.md                 7/14
src/disposable_lima_worker.rs                     6/14
src/disposable_template_runtime.rs                 4/14
src/disposable_worker_coordinator.rs               4/14
src/unix_personal_worker_store/disposable_clone_transaction.rs  4/14
```

The cold repository scan also surfaced four existing naming findings. Those are useful signal-quality corpus candidates; the carrier records them before deciding whether they are useful or noisy.

## Three-repository coverage carrier

PR #138 expanded the external lane with one exact Rust diff plus two adapter-gap controls.

Successful carrier receipt:

```text
workflow run: 32241829802
artifact:     9361136959
sha256:       c7ccd98334e58c7100c5c725642f71e9b2daf805a801fdb8600b093fce26e29b
```

### Cloud Hypervisor #8734 — canonical precedent tension

Exact coordinate:

```text
teamleaderleo/cloud-hypervisor@439ff52249e819f41570a5f3f3bf535d4bfb3e6e
base 1b004a7459ac752e1d7ad5a48237a1cb8608003b
pci/src/vfio.rs
```

The patch adds `#[cfg(test)] mod unit_tests` before an existing `#[cfg(test)] mod tests`.

The repository scan found:

```text
128 test-gated modules
unit_tests=89
tests=33
test_util=4
external_fds_tests=1
mock_vmm=1
```

The exact diff emitted one `test-module-precedent-tension` finding:

```text
changed declaration: unit_tests
repository precedent excluding changed declaration: unit_tests=88 of 127
same-file existing precedent: tests
observation: repository-wide and file-local precedent disagree
alignment: change follows repository-wide precedent and differs from file-local precedent
claim boundary: repository evidence alone does not establish which scope should govern
```

Work receipts:

```text
cold scan:  4 Git, 294 parses,   3 hits, 659355 us, 8 findings
warm scan:  4 Git,   0 parses, 297 hits,  16206 us, 8 findings
ci-tests:   1 Git,   0 parses,   0 hits,   1496 us, 0 findings
history:    2 Git,   0 parses,              7791 us, 5 discovered / 4 considered commits
diff:       9 Git,   1 parse,  296 hits,   38510 us, 1 finding
```

This is the canonical scope-tension discriminator from #16 in executable form. A preliminary carrier failed because the shallow checkout lacked the explicit base tip; that negative receipt led to the bounded-base-fetch rule above.

### Stensibly — current product coverage control

Pinned current coordinate:

```text
teamleaderleo/stensibly@1cc5e00040f0267705c6e9328dde1088d65cd880
```

Current scan:

```text
4 Git processes
0 Rust files parsed
0 findings
53937 us
```

CI-test inventory:

```text
1 Git process
0 Rust files parsed
0 findings
1666 us
```

This is a clean adapter-gap discriminator. Stensibly's useful organizational history lives in TypeScript, provider state, PR/issue relationships, policy files, and longitudinal event evidence. Today's Rust/local analyzer surface sees essentially none of it.

### Linux Fieldwork — research-memory coverage control

Pinned current coordinate:

```text
teamleaderleo/linux-fieldwork@b835ed842299f7654afc00f4988f7586e0be63bc
```

Current scan:

```text
4 Git processes
2 Rust files parsed
0 findings
34482 us
```

CI-test inventory:

```text
1 Git process
0 Rust files parsed
0 findings
1620 us
```

The useful Fieldwork corpus spans issues, investigations, notes, bug-species synthesis, and counterexamples. The quiet source scan therefore confirms the product boundary instead of disproving the value of the corpus.

## Corpus direction

The current registry starts with:

- Glaeda, using the pinned SmolRunner-era corpus for earned local history and agent-context work;
- Cloud Hypervisor for repository-vs-file precedent tension;
- Stensibly for longitudinal agentic churn and handoff/recovery questions;
- Linux Fieldwork for bug-species and research-memory controls.

The next evidence frontier is project memory and non-Rust repository evidence: issues, PRs, explicit references, policy evolution, and selected textual artifacts. The pinned `adapter_gap` cases define what that work must recover before it deserves product promotion.

History can now accumulate as descriptors first. A case only pays checkout/history cost when selected for an analyzer, a regression, or a current task.

## Receipts

Each run writes:

- `summary.json` — exact target HEAD, shallow/full-history boundary, probe list, counts, and performance receipts;
- `<probe>.json` — raw machine report;
- `<probe>.stderr.txt` — non-performance stderr;
- the GitHub job summary — a compact table of findings and work units.

Findings do not make a dogfood run fail. Process failures, malformed JSON, or a missing/invalid performance receipt do, because those make the evaluation itself unreliable.

Refs #16 #29 #41 #48 #49 #62 #129.
