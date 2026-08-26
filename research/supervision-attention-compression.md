# Supervision attention compression

## Status

Research note.

This note connects Cultist's existing just-enough-information, review-intelligence, compact-representation, decision-memory, and behavioral-evaluation work to a broader operational question:

> How much autonomous software work can one human supervise before reviewer attention becomes the limiting resource?

The immediate Cultist question is narrower:

> Which repository evidence earns interruption because it changes a consequential human decision, and which evidence should stay available on demand?

## Working hypothesis

A many-agent workflow can produce more candidate work than one human can inspect line by line. The useful product target therefore includes **attention compression**: reduce the human minutes required to reach a justified decision while preserving provenance, uncertainty, counterexamples, and recovery evidence.

This is compatible with Cultist's existing product test. Evidence earns prominent delivery when it changes inspection, validation, coordination, implementation, preservation, or another justified next action often enough to justify the interruption.

Attention compression adds one further observation: several useful evidence items may still deserve a single reconciled decision surface rather than several separate interruptions.

## Claims and current evidence level

### OBSERVED

Cultist already distinguishes several projections over shared repository evidence:

- lifecycle work asks when evidence should appear;
- JEI asks what evidence should be selected;
- review intelligence asks where reviewer attention should go;
- compact IR asks how evidence should be represented efficiently;
- decision memory asks what reviewed rationale should survive;
- behavioral receipts ask whether surfaced evidence changed worker behavior.

Cultist also retains both action-changing episodes and quiet/negative cases.

### INFERRED

These lanes can support a common operational objective: reduce repeated human investigation and preserve human attention for residual uncertainty and consequential judgment.

A useful reviewer experience may therefore be a bounded decision packet assembled from existing evidence primitives instead of a new evidence vocabulary.

### UNKNOWN

Current evidence does not establish a stable numeric relationship between packet size, reviewer time, decision quality, and downstream defect rate.

Current evidence also does not establish that agent-generated summaries reliably preserve every discriminator required for human review across repositories and task classes.

## Candidate supervision metric

A simple behavioral quantity is:

```text
supervision ratio
  = human review minutes / trustworthy autonomous work completed
```

This is intentionally task-relative. A reversible refactor, public API change, authentication change, research conclusion, and irreversible migration deserve different review burdens.

Cultist should avoid collapsing these into one opaque score. A useful experiment records the task class, evidence delivered, evidence consulted, decision reached, time or interaction burden when measurable, and the eventual result.

## Candidate decision packet

A reviewer-facing packet could reuse Cultist's existing provenance-bearing evidence while selecting only facts that change the decision:

```text
Decision requested:
Recommended action:
Why this reached human review:
Candidate revision / exact work identity:
Relevant deterministic checks:
Behavior or contract changes:
Repository precedent / explicit guidance:
Counterexamples:
Residual UNKNOWNs:
Recovery path:
Blocked downstream work:
Evidence omitted by budget:
```

The packet should link back to exact canonical sources and preserve omission/truncation receipts.

A packet is successful when the human can make the same justified decision with less investigation, or when it causes the human to inspect a discriminator that would otherwise have been missed.

## Review fan-in

Many workers may produce many individually valid receipts. A human-facing system gains little if every receipt becomes an interruption.

A useful intermediate projection can reconcile several child results into one parent review surface when they share one decision boundary.

Example:

```text
27 autonomous changes completed
  22 fully covered by deterministic checks and standing policy
   3 require semantic awareness but no decision
   2 share one public-behavior decision

human interruption count: 1
```

The research question is whether Cultist can identify the evidence that justifies those buckets without erasing a meaningful exception.

This belongs beside review intelligence: the question extends from **where should reviewer attention go?** to **which findings can safely fan in before that attention is requested?**

## Premise-change evidence

High execution throughput creates a second review problem: a worker can efficiently pursue a weak premise.

Cultist already values counterexamples, stale guidance detection, contradiction, provenance, and `UNKNOWN`. Those primitives may help surface premise-changing evidence separately from ordinary implementation findings.

Candidate signals include:

- a counterexample that invalidates the assumption shared by several active changes;
- repository history showing the target metric is a proxy rather than the actual product requirement;
- new evidence that makes an explicit decision record inapplicable;
- repeated worker disagreement around the same unstated premise;
- a failed deterministic discriminator that reveals the current question was framed incorrectly.

A premise-change finding deserves prominent review when it can redirect substantial downstream work.

## Experiments

### 1. Blind packet versus full-review replay

For a completed change with known evidence:

1. prepare a bounded decision packet from exact repository evidence;
2. give one fresh reviewer only that packet plus focused drill-down access;
3. give another fresh reviewer the ordinary repository/PR surface;
4. record decision, requested follow-up evidence, review time or interaction count, and eventual agreement with the accepted outcome;
5. preserve disagreements and missing discriminators.

A useful result is a clean reduction in review burden with equivalent or better decision quality.

### 2. Fan-in replay

Take a set of completed child changes sharing one parent outcome.

Compare:

- one interruption per child;
- one reconciled parent packet with exception drill-down.

Record whether the compressed view hides any decision-changing child evidence.

### 3. Quiet evidence negative control

Select evidence that is true and available but has no consequence for the current decision.

Verify that the attention selector keeps it out of the prominent packet while preserving explicit-query access.

### 4. Premise challenge replay

Use a historical change where later evidence redirected the work.

Ask whether a bounded contradiction/counterexample packet would have caused an earlier justified pivot.

### 5. Omission pressure

Reduce the packet byte or item budget until a decision changes or a reviewer asks for missing evidence.

The first lost discriminator is more informative than a generic compression percentage.

## Counterexamples and failure modes

Attention compression can fail in several ways:

- a concise summary hides the only important counterexample;
- several child changes appear similar while one has a distinct authority or recovery boundary;
- deterministic checks pass while product meaning changed;
- historical precedent is stale for the exact candidate;
- a reviewer accepts a recommendation because the packet framed alternatives poorly;
- aggressive fan-in delays a small but urgent exception;
- a low-interruption workflow merely shifts investigation cost into later failures.

These are first-class negative controls. A compression technique that reduces review time while increasing expensive wrong turns has failed the product test.

## Relationship to existing Cultist work

This note should reuse existing lanes instead of becoming a parallel product family:

- **JEI** supplies task-relative evidence selection;
- **review intelligence** supplies reviewer-attention selection;
- **C1 / compact IR** may reduce transmission cost after selection;
- **decision memory** preserves reviewed rationale that future packets may retrieve;
- **behavioral receipts** measure whether the interruption changed justified behavior;
- **active-work evidence** supplies fan-out, overlap, and blocked-work context where it is proven;
- **applicability/freshness work** protects against stale evidence presented as current truth.

The open research contribution here is the combined objective: minimize human review burden while preserving every discriminator needed for a justified consequential decision.

## Promotion test

A supervision-attention behavior should earn promotion only after real replays show that it can:

- reduce human review time or interaction count;
- preserve or improve decision quality;
- keep consequential `UNKNOWN`s visible;
- preserve provenance and exact work identity;
- surface premise-changing counterexamples early enough to redirect work;
- keep quiet evidence queryable without forcing interruption;
- avoid hiding distinct authority, recovery, or semantic boundaries during fan-in.

The desirable end state is a repository where a large amount of autonomous work can occur while human attention concentrates on a small number of evidence-backed decisions.
