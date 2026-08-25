# Roadmap

Cultist explores repository reasoning between two familiar extremes:

- deterministic tools that enforce rules we already know; and
- unconstrained AI review that is asked to infer everything from a codebase.

The project goal is to make the middle useful: recover bounded repository evidence, preserve what that evidence does and does not establish, and make the next human or agent less likely to repeat avoidable investigation.

Rust currently provides the deepest semantic adapter and the `cargo-cultist` distribution, but Cultist's evidence model is repository-oriented. Git history, provenance, active-change coordination, decision memory, and explicit project guidance can apply independently of source language.

Umbrella tracking issue: #19.

## Core evidence loop

```text
deterministic facts
  -> scoped observations
  -> counterexample search
  -> questions worth asking
  -> optional explanation
  -> human/project decision
  -> preserved rationale
  -> promote stable consensus into deterministic policy
```

The primitive is a **finding**, not an error. A good finding says what the repository evidence shows, what it does not establish, and why a worker may want to look.

## Product pressure test

Issue #137 adds a behavioral lens across the existing research program:

> Does selected Cultist evidence change the next justified action, prevent an expensive wrong turn, or reduce important repository knowledge that a later worker has to rediscover manually?

This complements provenance and precision rather than replacing them. A finding can be epistemically careful and still cost attention without changing a decision. Another finding can justify its interruption repeatedly by sending workers toward a missing validation step, active collaborator, reviewed decision, generated output, or decisive counterexample.

Useful behavioral receipts include:

```text
surfaced
-> consulted
-> next inspection / validation / coordination / implementation changed
-> wrong turn prevented or reversed
```

and quiet/negative outcomes such as:

```text
surfaced -> ignored
surfaced -> irrelevant
surfaced -> stale before action
surfaced -> needed stronger evidence
candidate evidence suppressed -> quiet case stayed quiet
```

These receipts should remain inspectable. Cultist does not need one universal actionability score.

## Agent work loop

The newer agent-facing work composes those primitives across time:

```text
BEFORE CODE
  brief / JEI
  recover the evidence most likely to matter before the edit

DURING CODE
  check / diff / preflight / evidence queries
  reconcile live work with repository precedent, guidance,
  active changes, decisions, counterexamples, and unknowns

AFTER CODE
  teach / reviewed decision / promoted rule
  preserve an earned lesson when there is one

NEXT WORKER
  retrieves the reviewed repository memory
```

Compact form:

```text
retrieve -> work -> reconcile -> preserve -> retrieve
```

Lifecycle composition is tracked in #74. Bounded pre-edit context work is tracked in #62. Behavioral evaluation is tracked in #137.

## Different views, shared evidence

Several research directions answer different questions. They should compose rather than create parallel truth systems.

### Lifecycle: when?

`brief -> check/diff -> teach` asks **when** evidence should be recovered, reconciled, and preserved.

Tracking: #74, #62, #10.

### Just-enough information: what?

JEI work asks **what** evidence is worth spending the worker's attention/context budget on for the current task.

Selection should favor explicit authority/decisions, exact action facts, counterevidence, active/freshness information, local precedent, and useful unknowns before broad context volume.

Tracking: #106.

### Review intelligence: where?

Review envelopes ask **where** scarce reviewer attention should go and what evidence would change the review decision. A review view should reuse the same facts rather than invent an opaque risk score.

Tracking: #109.

### Compact representation: how?

C1 and compact-IR research ask **how** evidence should be transmitted efficiently and interoperably.

Merged C1 work is a lossless structural encoding of the current `AnalysisReport`; it deliberately does not perform JEI selection or semantic abbreviation. The broader compact-IR work can add explicit validity, omission, transition, invalidation, supersession, and reopen semantics as those concepts earn a stable representation.

Tracking: #113, #115.

### Decision memory: what survives?

Decision-memory work asks **what reviewed rationale should remain recoverable** after the original task and conversation are gone.

Tracking: #10 plus decision-memory research under #75.

### Behavioral product pressure: did it help?

Behavioral evaluation asks **whether the selected evidence changed the worker's justified behavior enough to earn its attention cost**.

The first important comparison is held-out work with and without selected Cultist evidence while repository/task state stays equivalent. Record first relevant inspection, irrelevant expansion, known failed approaches, validation choices, coordination choices, and completion outcome.

Tracking: #137, #16.

A new projection should not invent a second provenance, authority, freshness, counterexample, unknown, or omission vocabulary merely because its layout differs.

## Principles

### Ask questions before inventing rules

Repository statistics are evidence, not policy. Popularity alone does not make a convention correct for every scope.

### Keep scope visible

Precedent can differ across a file, package/crate, repository, recent history, explicit project guidance, and current work. When those scopes disagree, Cultist should expose the tension rather than flatten it into one score.

Tracking: #3.

### Search counterexamples first

Before promoting precedent or association into a stronger claim, look for exceptions and ask whether they reveal a narrower cohort, scope, or reason.

Tracking: #6.

### Preserve provenance

Cultist distinguishes:

- **PROVEN** — exact machine facts or guarantees;
- **DERIVED** — deterministic conclusions from explicit facts;
- **OBSERVED** — empirical or provider-supplied observations;
- **INFERRED** — plausible interpretations;
- **UNKNOWN** — evidence is insufficient to recover the answer.

Tracking: #15.

### Applicability is separate from source authority

An explicit source can still be stale, copied, or attached to the wrong current work coordinate. Remote prose and metadata should remain bound to exact work identity/head/freshness evidence. Contradictory exact-head evidence can make the source's current applicability unknown without declaring the source itself untrustworthy.

This boundary is especially important for project-memory and coordination adapters (#18, #105).

### `UNKNOWN` is useful

If the repository cannot recover why an important-looking workaround, exception, stale branch, or metadata claim exists, say so instead of fabricating intent.

### Earn the interruption

Automatic evidence should increasingly justify the attention it consumes. High-value families repeatedly change inspection, validation, coordination, or preservation behavior. Lower-value observations can remain available through explicit queries, research views, or quieter projections.

The action relevance of a finding is separate from its epistemic claim kind. A `PROVEN` fact can still be irrelevant to the current decision; an `UNKNOWN` can be highly actionable when it identifies the missing discriminator blocking safe progress.

Tracking: #137, #109.

### Keep the core deterministic and bounded

Model-assisted explanation may help interpret selected evidence, but the deterministic evidence packet must remain useful without a model. Remote/provider integrations should produce explicit bounded artifacts that local analyzers can validate.

Tracking: #17.

### Teach the project why

When maintainers accept an intentional exception or project decision, the reason should be preservable in version control so later work can distinguish reviewed rationale from forgotten folklore.

Tracking: #10.

### Promote mature questions into rules

Repeated, stable human consensus can become deterministic policy, but promotion is an explicit project decision rather than automatic statistical enforcement.

Tracking: #11.

### Learn from the work itself

Cultist development is part of the evaluation corpus. Duplicate work, stale evidence, failed experiments, noisy findings, metadata mismatch, repeated manual archaeology, and action-changing findings are candidate product evidence.

Workers should preserve the exact episode, test the generalization, record the downstream consequence when observable, and use the smallest appropriate durable follow-up. See `AGENTS.md`.

## Workstreams

### 1. Precedent and repository relationships

- #3 — scope-aware precedent and tension
- #4 — temporal precedent and convention drift
- #6 — counterexample-first findings
- #20 — locally expanded idioms and helper precedent
- #21 — package/dependency intent
- historical companion and generated-relationship work

Historical co-change remains association evidence until stronger repository evidence establishes a stronger relation.

### 2. Archaeology and project memory

- #8 — exception archaeology
- #9 — historical fossils / expired workarounds
- #12 — `why` mode and evidence packets
- #18 — Git, PRs, issues, and reviews as project memory
- #38 — explicit repository guidance as higher-authority precedent

The question is not only “what changed?” but “what did the repository believe, what happened, and what lesson became durable?”

### 3. Institutional memory and policy

- #10 — explicit decision records / `teach`
- #11 — lint incubation and promotion
- #75 — decision-memory research

A model may propose rationale; reviewed repository state is what makes rationale durable project evidence.

### 4. Concurrent work and coordination

- #96 — preflight collision analysis
- #101 — explicit coordination edges in active-change inventories
- #105 — producer-side project metadata adapters

Current product supports local ref comparison and bounded active-work inventories. Cultist dogfoods an always-on PR heads-up that stays quiet for disjoint exact paths. Unpublished branch awareness remains research-only until branch activity/intent has a better discriminator than mere divergence.

### 5. Agent context, JEI, and review

- #62 — bounded pre-edit agent context packets
- #74 — before/during/after lifecycle
- #106 — JEI work envelopes
- #109 — review intelligence envelopes
- #137 — behavioral decision-changing evidence evaluation

The goal is evidence selected for the current decision, not giant repository summaries. Behavioral replays should test whether the selection changes investigation or action usefully.

### 6. Machine protocols and interoperability

- #22 — stable JSON findings
- #113 — C1 compact evidence grammar
- #115 — compact Cultist IR / context-relative protocol research

Machine formats should preserve meaning, provenance, unknowns, and omissions. Unsupported future semantics should fail explicitly rather than disappear during down-conversion.

### 7. Engine and performance

- #13 — local evidence index
- #14 — progressive semantic adapters
- #48 / #49 — performance measurement and demand-driven execution
- #50 — reusable repository snapshots / summaries

Work should be proportional to the evidence actually needed. Cheap irrelevant paths should stay cheap; expensive semantic/history layers should activate on demand.

### 8. Evaluation corpora

- #16 — dogfood/evaluation corpus
- #41 — Stensibly agentic organizational-history corpus
- #137 — behavioral A/B and interruption outcomes
- Glaeda replay research over the pinned SmolRunner-era corpus

Evaluation should include positive controls, quiet negatives, false assumptions, failed proofs, duplicate lanes, second-order regressions, and whether surfaced evidence actually changed behavior.

## Current product evidence

Cultist already has executable slices of several parts of this roadmap:

- provenance-bearing shared text/JSON findings;
- changed-first diff analysis, also exposed through the task-oriented `check` alias;
- historical companion evidence with examples/counterexamples;
- CI test-selector analysis;
- generated-companion evidence;
- direct concurrent-change preflight;
- bounded provider-supplied active-work preflight with explicit coordination edges;
- an always-on, non-blocking PR active-work heads-up in Cultist's own CI;
- research decision memory and pre-edit agent context packets;
- lossless compact C1 report encoding;
- performance work counters and demand-driven execution research.

These are evidence primitives and experiments, not a claim that the full agent lifecycle is finished.

## Near-term sequence

The most useful next work is composition and discrimination rather than adding broad new feature families:

1. run the #137 behavioral A/B gate on held-out tasks while #62/#106 packet work continues;
2. make pre-edit JEI consume the strongest already-earned evidence without becoming a context dump;
3. connect live check/diff/preflight evidence to the same task envelope during work;
4. keep review-attention output a projection over shared evidence rather than a second analyzer universe;
5. improve explicit project-memory applicability/freshness before trusting remote prose as current intent;
6. continue decision-memory authority research from current main after failed/stale carrier experiments;
7. evolve compact representation as new semantic primitives earn stable contracts;
8. measure interruption/context economics and promote, demote, or quiet evidence families from receipts;
9. keep performance work proportional to evidence demand.

This is not a commitment to build everything in order. Small experiments that falsify an assumption are valuable, and failed carriers should be retired instead of remaining fake active work.

## Product test

Useful recurring questions are:

> What did this worker have to discover manually that the repository could have surfaced, bounded, or preserved before the same mistake or investigation happens again?

> Which surfaced evidence changed the next justified inspection, validation, coordination, implementation, or preservation step?

Cultist earns its place as those manual rediscovery and avoidable wrong-turn burdens become progressively smaller without replacing project judgment with opaque automation.
