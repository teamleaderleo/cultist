# Agent experience episodes: learning without retrospective doctrine

Status: research carrier for #41, #74, #106, #137, and #148.

## Question

Can yesterday's agent work make a later worker measurably cheaper, more reliable, or less likely to repeat the same wrong turn while preserving:

```text
what happened
when the lesson applies
which counterexamples weaken it
what an operator changed
what earned deterministic persistence
what remains rejected / provisional / behaviorally null
```

The v1 carrier records agent-work experience. Existing Cultist lanes continue to own selection, reviewed memory, promotion, and behavioral evaluation.

## Existing owners

```text
#41
  real agentic corpus and failure / learning species

#106
  task-conditioned JEI selection and explicit applicability

#74
  brief -> diff -> teach lifecycle

#10 / #11
  reviewed decision memory and explicit deterministic promotion

#148
  keep / weaken / split / reject refinement transitions

#217 / #237
  smallest actionable prior-episode front and selected detail

#137
  receiver-side behavioral evaluation
```

The missing question is narrower:

> Which exact agent-work episode should those systems be able to reference later?

## V1 contract

`src/agent_experience_episode.rs` admits a bounded batch with:

```text
identity
  episode id
  repository
  exact revision when one episode has one useful revision
  work / run identity

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

lessons
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
  exact observed token / CI / repair quantities

behavior
  links to executed behavioral evaluation

cross-repository evidence
  repositories where a technique was observed

automatic_policy_authority = false
```

The last field fails closed. The episode can record promotion while the promoted code/test/decision remains the authoritative artifact.

## Roles

The first real corpus needs these distinctions:

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

`failure_class` stays source-supplied. The corpus has evidence for these roles; it has yet to earn a universal agent-failure ontology.

## Applicability before generalization

Every discriminator is either:

```text
applicability
outcome
```

Every lesson must name at least one applicability discriminator.

Example:

```text
Run 05A

applicability
  planned_validation_command = cargo
  worker_path_contains_command = false

observed cost
  input_tokens = 632,503

persistence
  --require-command preflight
```

That is enough to test the longitudinal claim directly:

```text
yesterday
  impossible worker launches
  missing command discovered later
  632,503 input tokens consumed

tomorrow
  exact required-command preflight fails before model launch
  model input tokens spent on the impossible launch = 0
```

## Lesson status and persistence stay separate

Lesson status:

```text
candidate
retained
weakened
rejected
promoted
```

Persistence artifact:

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

A promoted lesson must name an explicit persistence artifact. A `promoted_deterministic_check` role additionally requires a real deterministic-check artifact.

This lets the corpus retain all of these at once:

```text
successful reusable technique
rejected proposal
weakened routing heuristic
promoted executable check
behavioral null
```

## Real dogfood corpus

The retained fixture is:

```text
research/agent-experience-episodes/sol-luna-dogfood-v1.json
```

It contains eight executed episodes.

### Run 01: unsafe read boundary

```text
context_brief_defect
operator_intervention
promoted_deterministic_check
```

A Luna attempt crossed an information boundary. Sol stopped it, discarded the contaminated transcript, tightened the brief, and later runner work compiled read scope into a mechanical permission profile.

The promoted object is the permission profile. The stopped trajectory stays as the source episode.

### Run 01: focused review versus integration

```text
environment_defect
review_miss
integration_only_defect
operator_intervention
```

The focused exact-head reviewer accepted the candidate. Strict TypeScript checking was unavailable in the worker/reviewer checkout. Repository-wide CI later found a real TypeScript defect cluster and caused one repair turn.

The reusable lesson is conditional:

```text
promised gate unavailable locally
+ gate still pending
-> focused acceptance remains provisional
```

Semantic review and repository integration remain different defect surfaces.

### Run 03: Git metadata capability

```text
worker_capability_defect
rejected_lesson
promoted_deterministic_check
```

A review hypothesis treated `workspace-write` as evidence of `.git` write authority. A physical probe falsified it.

The episode keeps both transitions:

```text
rejected
  workspace-write implies Git metadata write authority

promoted
  Git metadata write authority requires its own capability preflight
```

### Run 03H: rejected combined-failure proposal

```text
rejected_lesson
operator_intervention
```

A verification worker proposed replacing a simultaneous non-zero child outcome with a retention failure. Sol rejected that proposal.

A combined-failure regression now preserves both:

```text
worker_failed
harnessError
```

The rejected candidate can later surface when the same semantics recur, while unrelated work stays quiet.

### Run 05A: missing required executable

```text
environment_defect
promoted_deterministic_check
```

A Luna High SmolRunner review was allowed to run a focused Cargo test while the confined worker PATH lacked Cargo.

Exact observed cost:

```text
input_tokens = 632,503
```

Stensibly #1667 promoted the lesson into `--require-command`, which fails before authentication/model launch when the declared executable is absent.

### Run 05: review-effort routing counterexample

```text
counterexample_to_routing_heuristic
```

The same Run 05 wave supplies this narrow counterexample:

```text
High review
  632,503 input tokens
  no finding

later bounded Low review on the smaller preflight follow-up
  found executable-directory false positive
```

The heads differ, so the earned conclusion is:

```text
risk-alone-routes-high = weakened
```

The replacement remains a candidate:

```text
ambiguity and expected discrimination value belong beside consequence
```

A same-head replay is the next discriminator.

The separate 2,247,548-token repair-review receipt belongs to the earlier Run 01 sequence. It remains source evidence for review-cost calibration and is deliberately excluded from this Run 05 episode.

### Portfolio: exact-head independent review

```text
cross_repository_reusable_technique
operator_intervention
```

The portfolio journal retains useful exact-head independent review outcomes across:

```text
teamleaderleo/stensibly
teamleaderleo/cultist
teamleaderleo/smolrunner
teamleaderleo/elatura
```

The technique carries an explicit limit from Run 01: independent semantic review can be useful while repository integration later finds a different defect class.

### Cultist #352: Luna Max null pair

```text
behavioral_null_result
rejected_lesson
```

Both blind arms chose the same first action. The control already named the accepted guard and enforcement path, so adding the explicit selected detail produced no first-action change in this pair.

The corpus rejects this broader claim:

```text
explicit accepted-guard detail always changes Luna Max's first action
```

The next experiment stays open:

```text
remove the explicit guard pointer from control
or vary the worker tier
```

## Selection discipline

V1 stops before relevance discovery.

A useful composition is:

```text
current task
-> #106 / source adapter identifies candidate episodes
-> existing repository/revision/work/path applicability applies
-> episode applicability discriminators are checked
-> rejected / weakened / promoted status remains visible
-> #217-style front projects the smallest action-changing item
-> #137 observes receiver behavior
```

Three rules guide the next experiment.

### Deterministic artifact first

When a lesson earned a deterministic check, run the check first.

Healthy result:

```text
stay quiet
```

Failing result:

```text
current check result
exact applicability
promoted artifact
minimal source episode ref
```

The disposable worker can skip the retrospective prose.

### Rejected and weakened lessons are conditional anti-repetition evidence

Surface a rejected/weakened lesson when:

```text
current applicability matches
or
current worker proposes the same candidate
```

Keep it queryable elsewhere.

This makes operator rejection useful without converting one historical decision into universal doctrine.

### Behavioral evidence changes delivery pressure

Behavioral receipts can justify:

```text
foreground this family
weaken this treatment
change the control packet
run a stronger discriminator
```

Source evidence, applicability, and repository authority remain separate.

The #352 null pair is the current negative control.

## Small next experiments

### A. Required-command longitudinal proof

Freeze a task that requires Cargo and a worker PATH without Cargo.

Record:

```text
model_spawned
input_tokens
first_failure_surface
operator_intervention_required
```

Expected promoted-arm result:

```text
model_spawned = false
input_tokens = 0
first_failure_surface = deterministic preflight
```

This is the clearest current cost-reduction experiment.

### B. Same-head review-effort pair

Replay the original buggy #1667 preflight head under identical blind review packets at two effort levels.

Record:

```text
finding class
first relevant inspection
input/output/reasoning tokens
wall-clock
false concerns
completion
```

This can strengthen, split, or reject the current routing candidate.

### C. Provisional-integration behavioral pair

Freeze:

```text
focused review = clean
promised local gate = unavailable
repository integration = pending
```

Treatment surfaces:

```text
keep acceptance provisional until promised integration settles
```

Measure premature final acceptance.

### D. Rejected-proposal recurrence control

One task recreates the Run 03H combined-failure semantics. One task is unrelated harness work.

Expected behavior:

```text
same semantics -> rejected episode + counterexample test surfaces
unrelated work -> quiet
```

### E. Cross-repository review replay

Use held-out candidates from at least two repositories and keep integration-only outcomes separate from review outcomes.

### F. Leaner Luna control

Repeat #352 with a leaner control or another worker tier.

Also test the harness observation from #352: remove irrelevant skill context and measure whether the roughly 17.7K input-token startup burden falls while first-action behavior stays stable.

## Hierarchy behavior

Disposable worker receives:

```text
current deterministic checks
small selected prior-episode front
exact detail when needed
explicit unknown / stop condition
```

Long-running coordinator can inspect:

```text
episode lineage
counterexamples
rejected candidates
exact costs
behavioral receipts
cross-repository recurrence
operator interventions
```

Repository retains:

```text
tests / CI / preflights
reviewed decisions
versioned guidance
exact evidence receipts
```

The hierarchy improves when the long-running layer converts experience into better evidence and the disposable layer receives only the earned, applicable slice.

## Reader and validation

Replay:

```text
cargo run --example agent_experience_episodes \
  < research/agent-experience-episodes/sol-luna-dogfood-v1.json
```

The validator rejects:

- oversized batches;
- malformed repository/revision identity;
- duplicate role/discriminator/lesson/persistence identity;
- lessons without an applicability discriminator;
- promoted lessons without persistence;
- promoted-check roles without a deterministic artifact;
- rejected-lesson roles without a rejected candidate;
- routing counterexamples without a weakened/rejected lesson;
- cross-repository techniques with fewer than two validated repositories;
- intervention receipts without the matching role;
- empty exact-cost objects;
- `automatic_policy_authority = true`.

## Boundary

- research-only carrier;
- no similarity search or relevance rank;
- no agent reputation or scalar learning score;
- no chronology-derived causality;
- no automatic brief/guidance rewrite;
- no automatic promotion/demotion;
- no general behavioral claim from one positive or null pair;
- source deterministic checks, review, CI, and reviewed decisions remain authoritative.

North star:

> Preserve enough exact experience that a later worker can skip a known dead end, run the earned check first, or ask the sharper question while rejected lessons, counterexamples, and behavioral nulls remain available to challenge yesterday's conclusion.
