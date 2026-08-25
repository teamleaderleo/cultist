# Agent experience episodes: learning without retrospective doctrine

Status: research carrier for #41, #74, #106, #137, and #148.

## Question

Can yesterday's agent work make a later worker measurably cheaper, more reliable, or less likely to repeat the same wrong turn while preserving the exact boundary between:

```text
what happened
what probably caused it
when the lesson applies
which counterexamples weaken it
what an operator changed
what earned deterministic persistence
what remains rejected / provisional / behaviorally null
```

The first carrier answers only the evidence-recording layer.

It does not create an agent score, a worker reputation, a universal failure taxonomy, or automatic operating policy.

## Existing owners already cover most of the loop

This experiment extends existing lanes instead of opening a parallel memory system.

```text
#41
  real agentic corpus and failure/learning species

#106
  task-conditioned JEI selection and explicit applicability/unknowns

#74
  brief -> diff -> teach lifecycle

#10 / #11
  reviewed decision memory and explicit deterministic promotion

#148
  keep / weaken / split / reject refinement transitions

#217 / #237
  smallest actionable prior-episode front and post-selection detail

#137
  whether surfaced evidence changes justified worker behavior
```

The missing carrier is narrower:

> What exact work episode should those systems be able to refer to when the thing being learned is about agent work itself?

## V1 contract

`src/agent_experience_episode.rs` admits a bounded batch of typed episodes.

Each episode retains:

```text
identity
  stable episode id
  repository
  exact revision when available
  work/run identity

classification
  caller-supplied failure_class
  typed evidence roles

evidence
  exact source refs
  counterexample refs

discriminators
  applicability facts
  observed outcome facts

intervention
  actor
  action
  source
  observed outcome

candidate lessons
  candidate
  retained
  weakened
  rejected
  promoted

persistence
  research receipt
  brief contract
  review cue
  deterministic check
  counterexample test
  decision record
  operating guidance
  reusable technique

cost
  exact token / CI / repair quantities when observed

behavior
  links to already-executed behavioral evaluation

cross-repository evidence
  exact repositories where a technique was observed

automatic_policy_authority = false
```

The last field is fail-closed. A retained experience episode cannot grant automatic project policy authority.

## Evidence roles

V1 carries the concrete distinctions exposed by the current dogfood:

```text
context_brief_defect
environment_defect
worker_capability_defect
review_miss
integration_only_defect
rejected_lesson
promoted_deterministic_check
counterexample_to_routing_heuristic
cross_repository_reusable_technique
operator_intervention
behavioral_null_result
```

These roles classify evidence carried by the episode. `failure_class` remains source-supplied because the repository has not earned a universal ontology of agent failures.

A later corpus can add a role only when an executed case needs a distinction the current set cannot express.

## Applicability is separate from outcome evidence

Every discriminator is explicitly one of:

```text
applicability
outcome
```

Example:

```text
Run 05A

applicability
  planned_validation_command = cargo
  worker_path_contains_command = false

outcome
  632,503 input tokens consumed before the missing executable stopped validation
```

A lesson must name at least one applicability discriminator.

This prevents a seductive retrospective pattern such as:

```text
High review found nothing
Low review found a bug
```

from becoming a context-free routing rule.

The retained routing episode instead says:

```text
applicability
  review_candidate = small_high_consequence_diff

outcomes
  one expensive High review produced no finding
  one bounded Low review found an executable-resolution bug on a smaller follow-up

lesson
  risk-alone-routes-high = weakened

candidate replacement
  ambiguity/discrimination value also belongs in routing evidence
```

The candidate replacement stays a candidate.

## Lesson status is separate from persistence

V1 keeps two axes distinct.

### Lesson status

```text
candidate
retained
weakened
rejected
promoted
```

### Persistence artifact

```text
research_receipt
brief_contract
review_cue
deterministic_check
counterexample_test
decision_record
operating_guidance
reusable_technique
```

A `promoted` lesson must point at an explicit persistence artifact.

A `promoted_deterministic_check` role additionally requires a real `deterministic_check` artifact.

The experience record still has zero automatic policy authority. The code/test/decision that earned review and merge is the authoritative artifact for the behavior it implements.

This yields the desired distinction:

```text
retrospective prose
  -> evidence

reviewed reusable lesson
  -> retained evidence + applicability

repeated lesson that earns code/test/CI
  -> promoted lesson + exact deterministic artifact

rejected proposal
  -> rejected lesson + counterexample / test

behavioral null
  -> retained null receipt
```

## Real dogfood corpus

The retained v1 corpus is:

```text
research/agent-experience-episodes/sol-luna-dogfood-v1.json
```

It contains eight executed cases.

### 1. Run 01 unsafe read boundary

Observed classification:

```text
context_brief_defect
operator_intervention
promoted_deterministic_check
```

The first Luna attempt crossed an information boundary. Sol stopped it, discarded the contaminated transcript, tightened the brief, and the later runner work compiled read scope into a mechanical permission profile.

The useful lesson is the mechanical profile. The discarded trajectory remains evidence for why it exists.

### 2. Run 01 focused review vs repository integration

Observed classification:

```text
environment_defect
review_miss
integration_only_defect
operator_intervention
```

The exact-head focused reviewer accepted the candidate. Full strict typecheck was unavailable in the worker/reviewer checkout. Repository-wide CI later found a real TypeScript defect cluster and triggered one repair turn.

The promoted contract is:

```text
focused evidence can be provisional
promised unavailable integration evidence stays pending
final acceptance waits for that gate
```

This keeps semantic review and integration evidence as different defect surfaces.

### 3. Run 03 Git metadata capability

Observed classification:

```text
worker_capability_defect
rejected_lesson
promoted_deterministic_check
```

A review hypothesis said `workspace-write` permitted `.git` mutation. A physical probe falsified it and exposed the inverse defect: the harness could promise Git metadata write authority that the confinement did not grant.

The corpus keeps both:

```text
rejected
  workspace-write implies Git write

promoted
  separate Git metadata capability preflight
```

### 4. Run 03H rejected combined-failure proposal

Observed classification:

```text
rejected_lesson
operator_intervention
```

A Low verification worker proposed letting an evidence-retention failure override a simultaneous non-zero child result. Sol rejected the proposal.

The durable artifact is a combined-failure regression preserving both causes:

```text
worker_failed
harnessError
```

The rejected proposal remains useful historical evidence. It never becomes a hidden negative rule for unrelated tasks.

### 5. Run 05A missing required executable

Observed classification:

```text
environment_defect
promoted_deterministic_check
```

A Luna High SmolRunner review had permission to run a focused Cargo test, while the confined worker `PATH` lacked Cargo. The run consumed exactly:

```text
input tokens = 632,503
```

before returning without that validation.

Stensibly #1667 promoted the lesson into `--require-command`.

Its key property is measurable:

```text
historical failure
  missing required command discovered after model launch
  632,503 input tokens consumed

promoted path
  missing required command discovered before authentication/model launch
  model token spend for that failed launch = 0
```

This is the clearest current example of yesterday's work making tomorrow's worker cheaper.

### 6. Run 05 review-effort routing counterexample

Observed classification:

```text
counterexample_to_routing_heuristic
```

The corpus retains the exact expensive High-review quantities:

```text
input      2,247,548
output        23,039
reasoning     11,287
```

and the later bounded Low review that found the executable-directory false positive in the required-command preflight.

The retained conclusion is deliberately narrow:

```text
risk-alone-routes-high = weakened
```

The corpus does not claim `Low is better`. The two reviews covered different heads. A same-head replay is still required.

### 7. Cross-repository exact-head independent review

Observed classification:

```text
cross_repository_reusable_technique
operator_intervention
```

The portfolio journal records useful independent exact-head findings across:

```text
teamleaderleo/stensibly
teamleaderleo/cultist
teamleaderleo/smolrunner
teamleaderleo/elatura
```

The retained technique keeps a counterexample beside it: Stensibly #1661 had a correct focused semantic review while later integration found a different defect class.

So the reusable technique is:

```text
independent exact-head review can discover distinct defects
integration remains a separate evidence surface
```

### 8. Cultist Luna Max guard-detail null pair

Observed classification:

```text
behavioral_null_result
rejected_lesson
```

Cultist #352 retained a clean blind pair where both arms chose the same first action. The treatment's extra accepted-guard detail did not change the first action under that control because the control already named the guard and enforcement path.

The corpus therefore rejects:

```text
explicit guard detail always changes Luna Max's first action
```

and keeps the next experiment as a candidate:

```text
use a leaner control or another worker tier
```

The null result survives. It can prevent future researchers from silently counting the pair as a positive.

## Selection discipline

V1 deliberately stops before relevance discovery.

A useful composition with current Cultist work is:

```text
current task
-> #106 / source adapter identifies candidate prior episodes
-> existing repository/revision/work/path applicability applies
-> episode applicability discriminators are checked
-> rejected / weakened / promoted status remains visible
-> #217-style front projects the smallest action-changing item
-> #137 observes what the receiver actually does
```

Three selection rules should guide the next experiment.

### 1. Deterministic artifact first

When a lesson already earned a deterministic check, execute the check.

For a healthy case, stay quiet.

For a failing case, surface:

```text
current check result
exact applicability
the promoted artifact
minimal source episode reference
```

A worker does not need the retrospective essay before every launch.

### 2. Rejected and weakened lessons are conditional anti-repetition evidence

A rejected proposal deserves front-of-context delivery when the current task matches its applicability discriminators or when a worker proposes the same candidate.

Otherwise it stays queryable and quiet.

This is how Run 03H can prevent repeated reasoning churn without turning one operator rejection into global doctrine.

### 3. Behavioral evidence can promote or demote attention, not truth

A behavioral receipt says what happened after delivery.

It can justify:

```text
foreground this episode family
weaken this treatment
change the control packet
run a stronger discriminator
```

It cannot rewrite the source episode or manufacture authority.

The #352 null pair is the first concrete negative control for this rule.

## Small next experiments

### A. Required-command longitudinal check

Use #1667 as the first exact longitudinal proof.

Protocol:

```text
candidate task requires cargo
worker PATH omits cargo

historical arm
  worker launches
  command absence discovered later
  observed input cost = 632,503

promoted arm
  required-command preflight runs first
  preflight rejects
  worker process never launches
```

Retain:

```text
model_spawned
input_tokens
first_failure_surface
operator_intervention_required
```

Success is mechanical: the promoted path spends zero model tokens on the impossible worker launch.

### B. Same-head review-effort replay

Replay the original buggy #1667 preflight head under the same blind review packet at two effort levels.

Record:

```text
finding class
first relevant inspection
input/output/reasoning tokens
wall-clock
false concerns
completion
```

The existing corpus only weakens `risk -> High`. A same-head pair can test whether the executable-resolution bug is reliably discoverable at lower effort.

### C. Provisional-integration behavioral pair

Freeze a task packet where:

```text
focused review = clean
promised strict typecheck = unavailable locally
repository integration = pending
```

Control omits the prior episode.

Treatment surfaces the compact prior-episode action:

```text
keep acceptance provisional until promised integration settles
```

The first behavioral discriminator is whether the worker/coordinator declares final acceptance before the pending gate.

### D. Rejected-proposal recurrence control

Construct one mutation that proposes the Run 03H override again and one unrelated harness change.

Expected behavior:

```text
same combined-failure semantics
  -> rejected episode surfaces with counterexample test

unrelated harness task
  -> episode stays quiet
```

This tests anti-repetition value and interruption cost together.

### E. Cross-repository review technique replay

Use held-out exact-head candidates from at least two repositories with different defect classes.

Compare:

```text
ordinary green checks
vs
green checks + compact independent-review cue
```

Keep integration-only defects as a separate outcome so review success cannot absorb the integration gate.

### F. Leaner Luna behavioral control

Repeat the #352 guard-detail pair with a control that omits the explicit guard pointer, or with another worker tier.

Also test the harness observation already recorded by #352: remove irrelevant skill context and measure whether the roughly 17.7K input-token startup burden falls without changing the first-action distribution.

## Hierarchy behavior

The intended learning hierarchy is simple.

### Disposable worker

Receives only:

```text
current deterministic checks
small selected prior-episode front
exact operational detail when needed
explicit unknown / stop condition
```

It does not inherit the whole retrospective corpus.

### Long-running coordinator

Can inspect:

```text
episode lineage
counterexamples
rejected candidates
exact costs
behavioral receipts
cross-repository recurrence
operator interventions
```

It decides which candidate deserves another replay, a weaker cue, a brief change, or an explicit promotion proposal.

### Repository

Keeps the durable source of truth:

```text
tests / CI / preflights
reviewed decision records
versioned guidance
exact evidence receipts
```

The hierarchy improves because the long-running layer compresses experience into better evidence and the disposable layer consumes only the earned, applicable slice.

## Reader and validation

Replay the retained corpus with:

```text
cargo run --example agent_experience_episodes \
  < research/agent-experience-episodes/sol-luna-dogfood-v1.json
```

The validator fails closed on:

- oversized batches;
- malformed repository/revision identity;
- duplicate episode, role, discriminator, lesson, or persistence identity;
- lessons without an applicability discriminator;
- promoted lessons without explicit persistence;
- promoted deterministic-check roles without a deterministic artifact;
- rejected-lesson roles without a rejected candidate;
- routing counterexamples without a weakened/rejected lesson;
- cross-repository techniques with fewer than two validated repositories;
- intervention receipts without the matching role;
- empty exact-cost objects;
- any `automatic_policy_authority = true`.

## Boundary

- research-only carrier;
- no similarity search;
- no relevance rank;
- no agent reputation;
- no scalar learning score;
- no chronology-derived causality;
- no model-brand capability doctrine;
- no automatic brief mutation;
- no automatic operating-guidance rewrite;
- no automatic promotion/demotion;
- no final behavioral claim from one null or positive pair;
- no replacement for the source deterministic check, review, CI, or decision record.

North star:

> Preserve enough exact experience that a later worker can skip a known dead end, run the earned check first, or ask the sharper question—while every rejected lesson, counterexample, and behavioral null remains available to stop yesterday's conclusion from becoming tomorrow's dogma.
