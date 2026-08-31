# Cultist

**Find out why before you copy it.**

Cultist is an experiment in repository-aware evidence for software work: recover deterministic facts, keep provenance and counterexamples visible, and ask useful questions before inventing project rules.

The product test is empirical: **does selected evidence change the next justified action, prevent an expensive wrong turn, or spare a later worker from repeating manual investigation?** Issue #137 owns that pressure test; [ROADMAP.md](ROADMAP.md) owns the broader research map. Agents should follow [AGENTS.md](AGENTS.md).

> Status: active research prototype in sustained dogfood. The Rust distribution is `cargo-cultist`; public analyzer commands are deterministic, local, and read-only. Remote/project adapters can build evidence inventories around the core analyzer boundary.

Rust is the first deep semantic adapter, not the product boundary. Repository-generic primitives already include Git history, claim provenance, concurrent-change preflight, active-work inventories, bounded evidence packets, and repo-local decision-memory research.

Traditional linters are strongest after a rule is known. Cultist starts earlier: gather what a repository actually does, keep contradictions and uncertainty visible, and help a worker decide what to inspect, validate, coordinate, or preserve next.

The claim vocabulary is:

- **PROVEN** — exact machine facts or guarantees;
- **DERIVED** — deterministic conclusions from explicit facts;
- **OBSERVED** — empirical repository patterns or supplied observations;
- **INFERRED** — plausible interpretations;
- **UNKNOWN** — evidence is insufficient to recover the answer.

Repository guidance/authority stays separate from observed precedent. Frequency never silently becomes policy. Human-readable and JSON output use the same provenance-bearing finding model where a command produces findings.

## Current public commands

The package/binary is `cargo-cultist`, so installed use is `cargo cultist ...`.

### Repository test-module conventions

The default command inspects Rust `#[cfg(test)]` modules and reports the names a repository actually uses without promoting majority spelling into policy.

```bash
cargo cultist
cargo cultist --format json
```

### Change-time evidence

`cargo cultist check` and `cargo cultist diff` run the same change analyzer. `check` is the task-oriented alias; `diff` remains available for compatibility.

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

Direct shared paths are deterministic collision evidence. Different paths remain semantically `UNKNOWN` until independent generated, historical, policy, or coordination evidence establishes a relationship.

Inventory mode accepts a bounded provider/orchestrator-supplied active-change snapshot:

```bash
cargo cultist preflight --inventory active-work.json
cargo cultist preflight --inventory active-work.json --format json
```

The inventory contract carries exact work identity, head, freshness, changed-path observations, and optional explicit coordination edges such as `depends_on`, `blocks`, `hold_merge_while`, and `supersedes`. The core command does not fetch GitHub itself.

Cultist's PR CI dogfoods a GitHub adapter. Provider-backed evidence is bound to an exact snapshot identity and revalidated at the observed frontier when consumed. Partial or paginated observations fail closed instead of manufacturing complete file coverage. Cross-request provider snapshot consistency remains `UNKNOWN` unless the provider or a controlled replay proves it, so bounded completeness is asserted only from evidence that establishes it.

### Historical companions

`cargo cultist history FILE` explores which paths repeatedly changed with one current file in recent non-merge history.

```bash
cargo cultist history src/protocol.rs
cargo cultist history --max-commits 200 src/protocol.rs
cargo cultist history --format json src/protocol.rs
```

The explorer preserves directional support/opportunity counts, examples, absence counterexamples, exclusions, and cohort limits. Historical co-change remains association evidence, never required-update policy.

### CI test-filter inventory

`cargo cultist ci-tests` analyzes a deliberately narrow GitHub Actions Cargo/libtest selector family and compares literal selectors with conservative source inventories.

```bash
cargo cultist ci-tests
cargo cultist ci-tests --format json
```

Unsupported shell forms, ambiguous targets, unknown flags, generated tests, and parse gaps are skipped or surfaced conservatively instead of guessed through.

## Agent-facing research views

Current views are projections over shared repository evidence:

```text
edit lifecycle (#74)        WHEN recover, reconcile, or preserve evidence?
JEI (#106)                  WHAT evidence is worth selecting now?
review intelligence (#109)  WHERE should scarce reviewer attention go?
C1 / compact IR (#113/#115) HOW should selected evidence travel efficiently?
decision memory (#10)       WHAT reviewed rationale should survive?
behavioral pressure (#137)  DID evidence change justified work or reduce rediscovery?
```

A new view reuses authority, provenance, freshness, counterexample, `UNKNOWN`, and omission semantics instead of creating a competing truth vocabulary.

### Bounded context packets

Research under #62 asks:

> What repository evidence would I regret missing before I modify this target?

Packets use bounded defaults and explicit truncation/omission receipts while preserving guidance, history, companions, counterexamples, decisions, provenance, freshness, and useful `UNKNOWN`s. Selected evidence must survive byte pressure when removing it would change the justified next action.

### Compact C1 evidence grammar

Merged research provides a lossless C1 encoding of the current `AnalysisReport` model:

```bash
cargo run --example cultist_c1 < report.json
cargo run --example cultist_c1 -- --decode < report.c1
```

C1 is representation compression only. It does not select JEI, rank evidence, change authority, or abbreviate meaning. Unsupported future semantics fail closed during down-conversion.

### Decision memory

Repo-local decision-memory research explores how intentional exceptions and earned rationale can become version-controlled evidence. Decision records are evidence, not implicit suppressions, and model-authored prose gains no project authority merely because it was recorded.

## Work loop

```text
BEFORE  recover bounded target evidence
DURING  reconcile the live change with evidence, guidance, counterexamples, active work, and UNKNOWNs
AFTER   preserve an intentional decision or earned lesson when useful
NEXT    let a later worker recover that repository memory
```

Or:

```text
retrieve -> work -> reconcile -> preserve -> retrieve
```

The prominent automatic evidence should earn its interruption by changing inspection, validation, coordination, or preservation behavior often enough to justify the attention cost.

## Research discipline

Standalone examples and durable receipts hold experiments that are outside the public product surface. The research loop is:

```text
hypothesis
-> deterministic probe
-> real repository discriminator
-> counterexample / negative control
-> durable receipt
-> keep, weaken, split, reject, or promote
```

A successful experiment does not automatically become a lint or public feature. Failed experiments stay useful when they expose a boundary. For dogfood signals, receipts, view taxonomy, promotion choices, and worked examples, use [docs/agent-playbook.md](docs/agent-playbook.md).

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

## Dogfooding and current direction

CI runs formatting, Clippy, tests, and the public analyzers against Cultist itself. Pull-request CI also runs the non-blocking active-work heads-up.

Dogfood is product input. Preserve exact evidence when work exposes duplicate effort, missed repository facts, stale evidence, misleading metadata, false assumptions, useful counterexamples, or repeated manual investigation. Record the downstream consequence when visible: what changed next, what stayed quiet, and which evidence proved too weak or stale. Generalization requires a discriminator and negative control.

Near-term work focuses on behavioral evaluation, bounded selected-evidence delivery, evidence applicability/reuse, active-work freshness and completeness, decision memory, counterexamples, explicit guidance, and promotion/demotion based on inspectable outcomes. [ROADMAP.md](ROADMAP.md) and the linked owner issues carry the evolving research chronology.

Optional model-assisted explanation can sit on top of bounded evidence later. The deterministic evidence packet must remain useful without a model.
