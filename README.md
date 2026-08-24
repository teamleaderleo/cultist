# Cultist

**Find out why before you copy it.**

Cultist is an experiment in repository-aware evidence for software work: recover deterministic facts, keep provenance and counterexamples visible, and ask useful questions before inventing project rules.

A central product test is now empirical: **does selected evidence change the next justified action, prevent an expensive wrong turn, or spare a later worker from repeating manual investigation?** That question is already producing retained behavioral evidence rather than sitting only as a future test. Issue #137 remains the broader pressure-test programme while blind paired trials, evidence selection, applicability, representation, and analyzer research continue.

> Status: active research prototype in sustained dogfood. The current Rust distribution is named `cargo-cultist`; its public analyzer commands are deterministic, local, and read-only. Research and integration lanes also retain real-repository discriminators, behavioral receipts, external replays, and promotion/demotion evidence. Remote/project adapters that build evidence inventories live outside the core analyzer boundary.

Rust is the first deep semantic adapter, not the product boundary. Several useful primitives are repository-generic already: Git history, claim provenance, concurrent-change preflight, active-work inventories, scoped evidence packets, and repo-local decision-memory research.

See [ROADMAP.md](ROADMAP.md) for the thesis and current research map. Agents working on Cultist should also follow [AGENTS.md](AGENTS.md).

Traditional linters are strongest after a rule is known. Cultist starts earlier. It gathers what a repository actually does, keeps contradictions and uncertainty visible, and helps a worker answer questions such as:

- What evidence would I regret missing before I edit this target?
- Does my live change disagree with local precedent or explicit project guidance?
- Is another current change touching the same repository surface?
- Why does this exception or guard exist?
- What lesson from this completed work should be recoverable by the next worker?
- Which current evidence changes what I should inspect, validate, coordinate, or preserve next?

The core claim vocabulary is:

- **PROVEN** — exact machine facts or guarantees;
- **DERIVED** — deterministic conclusions from explicit facts;
- **OBSERVED** — empirical repository patterns or supplied observations;
- **INFERRED** — plausible interpretations;
- **UNKNOWN** — evidence is insufficient to recover the answer.

Human-readable and JSON output are rendered from the same provenance-bearing finding model where the command produces findings.

## Current public commands

The Rust package/binary remains `cargo-cultist`, so installed use is `cargo cultist ...`.

### Repository test-module conventions

The default command inspects Rust `#[cfg(test)]` modules and reports the names a repository actually uses without promoting majority spelling into policy.

```bash
cargo cultist
cargo cultist --format json
```

### Change-time evidence

`cargo cultist check` and `cargo cultist diff` run the same existing change analyzer. `check` is a task-oriented alias; `diff` remains available for compatibility and for callers that prefer the exact underlying concept.

The analyzer applies supported change-time evidence such as Rust test-module precedent and generated-companion evidence.

```bash
cargo cultist check
cargo cultist check --base origin/main
cargo cultist check --base origin/main --format json

cargo cultist diff
cargo cultist diff --base origin/main
```

With `--base REV`, Cultist uses the merge base while still including local staged and unstaged work. Changed-file parse failures remain explicit uncertainty instead of becoming false absence claims.

### Concurrent-change preflight

Local ref mode compares two concurrent Git change sets from their merge base:

```bash
cargo cultist preflight --against other-agent
cargo cultist preflight --against origin/main --format json
```

Direct shared paths are deterministic collision evidence. Different paths remain semantically unknown until an independent evidence source establishes a generated, historical, policy, or explicit coordination relationship.

Inventory mode accepts a bounded provider/orchestrator-supplied active-change snapshot:

```bash
cargo cultist preflight --inventory active-work.json
cargo cultist preflight --inventory active-work.json --format json
```

The landed inventory contract can carry exact work identity/head/freshness/path observations plus explicit coordination edges such as `depends_on`, `blocks`, `hold_merge_while`, and `supersedes`. The core command does not fetch GitHub itself.

Cultist's own PR CI dogfoods a GitHub adapter for this contract. Common disjoint runs use a cheap exact-path prefilter and stay quiet; possible overlaps pay for the fuller deterministic analyzer. Research adapters also explore unpublished branches, but bare-branch activity is not enabled by default because divergence alone does not prove somebody is still working there.

Current main now binds that provider-backed dogfood to an explicit provider snapshot identity instead of treating one fetched inventory as timeless truth. The consumer revalidates the observed frontier when it uses the snapshot, preserves `UNKNOWN` when coordination/provider evidence is unresolved, and refuses to manufacture complete file coverage from paginated or partial observations. Complete bounded file coverage is asserted only when one provider response actually proves the selected union. These are landed correctness rules in the active-change path, not future research notes.

### Historical companions

`cargo cultist history FILE` explores which paths repeatedly changed with one current file in recent non-merge history.

```bash
cargo cultist history src/protocol.rs
cargo cultist history --max-commits 200 src/protocol.rs
cargo cultist history --format json src/protocol.rs
```

The explorer preserves directional support/opportunity counts, examples, absence counterexamples, exclusions, and known cohort limitations. Historical co-change remains association evidence rather than required-update policy.

### CI test-filter inventory

`cargo cultist ci-tests` analyzes a deliberately narrow GitHub Actions Cargo/libtest selector family and compares literal selectors with conservative source inventories.

```bash
cargo cultist ci-tests
cargo cultist ci-tests --format json
```

Unsupported shell forms, ambiguous targets, unknown flags, generated tests, and parse gaps are skipped or surfaced conservatively rather than guessed through.

## Agent-facing research views

Several current ideas are intentionally **different projections over shared repository evidence**, not competing sources of truth:

```text
edit lifecycle (#74)
  -> WHEN should evidence be recovered, reconciled, or preserved?

just-enough information / JEI (#106)
  -> WHAT evidence is worth selecting for this task now?

review intelligence (#109)
  -> WHERE should scarce reviewer attention go?

C1 / compact IR (#113, #115)
  -> HOW should evidence be represented and transmitted efficiently?

decision memory (#10 and research)
  -> WHAT reviewed rationale should survive for later workers?

behavioral product pressure (#137)
  -> DID the selected evidence change the next justified action or reduce rediscovery?
```

A new view should reuse authority, provenance, freshness, counterexample, unknown, and omission semantics rather than inventing a parallel vocabulary merely because its output layout is different.

### Bounded context packets

Research under #62 asks the pre-edit question:

> What repository evidence would I regret missing before I modify this target?

The packet work emphasizes bounded defaults, truncation receipts, explicit guidance, history, companions/counterexamples, decisions, and useful `UNKNOWN`s rather than giant repository summaries.

That work has already produced real discriminators: scoped evidence recovered decision-changing Stensibly history that file-local packets missed, selected-evidence protection preserved those lessons under severe byte pressure, and retained behavioral episodes record cases where surfaced evidence changed the next action as well as a quiet negative. The next stronger gate is cleaner blind paired execution across fresh sessions, plus broader replication before promotion claims become general product policy.

### Compact C1 evidence grammar

Merged research provides a lossless C1 encoding of the current `AnalysisReport` model. The converter remains an example rather than a second product binary:

```bash
cargo run --example cultist_c1 < report.json
cargo run --example cultist_c1 -- --decode < report.c1
```

C1 is structural compression only. It does not select JEI, rank evidence, change authority, or abbreviate meaning. Current machine-report deserialization fails on unknown fields so unsupported future semantics cannot be silently erased during down-conversion.

### Decision memory

Repo-local decision-memory research explores how intentional exceptions and earned project rationale can become version-controlled evidence for future work. Decision records are evidence, not implicit suppressions, and a model-generated sentence does not become project truth merely because it was written down.

## The work loop

The larger agent lifecycle being explored is:

```text
BEFORE
  recover bounded evidence for the target

DURING
  reconcile the live change with check/diff, precedent, guidance, active work,
  counterexamples, decisions, trust boundaries, and unknowns

AFTER
  preserve an intentional decision or earned lesson when appropriate

NEXT WORKER
  retrieves that repository memory before repeating the same investigation
```

Or more compactly:

```text
retrieve -> work -> reconcile -> preserve -> retrieve
```

The product goal is to make Cultist fade into the work itself. A worker should be able to ask for the evidence it needs, do the work, and leave useful reviewed knowledge behind. The most prominent automatic evidence should earn its interruption by changing inspection, validation, coordination, or preservation behavior often enough to justify the attention cost.

## Research discipline

The repository contains standalone examples and durable receipts for experiments that are not public product features. The preferred research lifecycle is:

```text
hypothesis
-> deterministic probe
-> real repository discriminator
-> counterexample / negative control
-> durable receipt
-> keep, weaken, split, reject, or promote
```

Some research examples intentionally execute repository tooling. Those effectful experiments carry explicit boundaries and are not silently invoked by ordinary analyzer commands.

A successful experiment does not automatically become a lint or public feature. Failed experiments are retained when they expose a useful boundary.

Behavioral evidence adds another useful promotion question:

```text
surfaced
-> consulted?
-> changed the next action?
-> prevented or reversed a wrong turn?
-> ignored / irrelevant / stale / needed stronger evidence?
```

The retained collaboration corpus already contains both action-changing and quiet cases. This remains an inspectable scorecard, not a single opaque risk or quality number.

## Usage while developing

```bash
cargo run -- /path/to/a/rust/repository
cargo run -- check --base origin/main /path/to/a/rust/repository
cargo run -- diff --base origin/main /path/to/a/rust/repository
cargo run -- preflight --against some-ref /path/to/a/repository
cargo run -- preflight --inventory /path/to/active-work.json /path/to/a/repository
cargo run -- history /path/to/a/repository/src/file.rs
cargo run -- ci-tests /path/to/a/rust/repository
```

After installing locally:

```bash
cargo install --path .
cd /path/to/a/repository
cargo cultist
cargo cultist check
cargo cultist diff
cargo cultist preflight --against other-ref
cargo cultist history src/file.rs
cargo cultist ci-tests
```

The binary can also be invoked directly as `cargo-cultist`.

## Dogfooding

CI runs formatting, Clippy, tests, and the public analyzers against Cultist itself. Pull-request CI also runs the non-blocking active-work heads-up.

Dogfood is product input. When work exposes duplicate effort, a missed repository fact, stale evidence, misleading metadata, a false assumption, a useful counterexample, or repeated manual investigation, preserve the exact evidence and ask whether the smallest useful Cultist improvement can surface it earlier next time.

Also record the downstream consequence when it is visible: did the evidence change the next inspection, validation step, coordination decision, or implementation? Did the worker ignore it? Did a natural quiet case remain quiet? Those receipts help decide which evidence families deserve automatic delivery.

Do not turn task friction into a universal rule without a discriminator and negative control.

## Current direction

Near-term work is increasingly about promoting, weakening, and composing evidence from actual dogfood rather than merely adding new analyzers:

- expand the retained behavioral corpus and run cleaner blind paired fresh-session trials (#137, #267);
- preserve explicitly selected evidence through scoped packet budgeting and fail closed below the selected evidence floor;
- reuse prior CI/promotion receipts when exact compatibility evidence supports reuse, while keeping semantic independence `UNKNOWN` when it has not been earned;
- refine evidence acquisition so replay-rejected or unselected alternatives do not create needless investigation work;
- continue bounded pre-edit JEI and lifecycle integration;
- keep review-attention projections over the same evidence;
- maintain active-change coordination with explicit provider snapshot identity, current-frontier revalidation, and bounded-coverage uncertainty;
- continue decision-memory authority/applicability research;
- enrich scoped and temporal precedent with counterexamples;
- keep explicit repository guidance and instruction freshness separate from inferred precedent;
- promote, demote, or quiet evidence families based on inspectable interruption outcomes;
- move repeated, well-understood lessons into deterministic policy only after the evidence earns that promotion.

Optional model-assisted explanation can sit on top of bounded evidence later. The deterministic evidence packet must remain useful without a model.
