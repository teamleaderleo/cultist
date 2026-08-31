# Cultist agent playbook

Cultist is built by dogfooding the same repository-reasoning ideas it is trying to make useful for other projects.

Do the requested task first. At the same time, pay attention to evidence that the work itself exposes about how Cultist could improve.

## Feed useful friction back into the project

If a task reveals something that made the work harder, less reliable, more repetitive, or easier to misunderstand, ask whether Cultist could have surfaced that fact earlier or preserved the lesson for the next worker.

Useful signals include:

- two workers, PRs, or branches independently doing overlapping work;
- an important repository fact, convention, generated relationship, decision, or policy that was easy to miss;
- a false assumption that later evidence disproves;
- a stale or wrong evidence coordinate;
- explicit prose whose applicability no longer matches the exact current change;
- a historical reason that had to be reconstructed manually;
- a useful counterexample that weakens an existing heuristic;
- an analyzer finding that was noisy, overconfident, incomplete, or missing an important `UNKNOWN`;
- a surfaced finding that changed the next inspection, validation, coordination, or implementation step;
- a surfaced finding that consumed attention and changed nothing;
- repeated investigation that could become a deterministic probe;
- a reviewed lesson that future agents would benefit from seeing before making the same edit.

Do not turn every inconvenience into a feature. Preserve the distinction between:

- an observation from this task;
- a plausible reusable pattern;
- a tested discriminator with counterexamples;
- a product behavior that has earned promotion.

## Prefer evidence over retrospective stories

When proposing a Cultist improvement from dogfood:

1. identify the exact task, change, file, issue, PR, commit, or repository evidence that exposed it;
2. state what is **PROVEN**, **DERIVED**, **OBSERVED**, **INFERRED**, or **UNKNOWN**;
3. search for a counterexample or negative control before generalizing;
4. keep chronology separate from causality unless an explicit relationship establishes it;
5. bind remote prose or metadata to its exact work/head/freshness coordinate before treating it as current intent;
6. prefer a small deterministic probe or evidence packet before a broad heuristic;
7. preserve failed experiments and weakened hypotheses when they teach a boundary.

A successful task can still expose a product problem. A failed task can still produce useful evidence. Neither automatically proves a general rule.

## Promote concurrent work from evidence, not main-SHA churn

This repository often has several workers landing changes at once. Do not treat every new `main` SHA as an automatic invalidation of a successful branch receipt.

For an open pull request, the normal compatibility authority is the **current GitHub merge view plus its successful required checks**.

When `main` advances after a successful run:

1. inspect the intervening change;
2. rerun when it touches a branch path, a directly consumed contract/module/schema/fixture/generated relation, an applicable policy/CI gate, changes mergeability, or otherwise changes a compatibility input;
3. preserve the existing receipt when the branch semantic tree is unchanged and the intervening change is demonstrably irrelevant to its compatibility inputs;
4. use `UNKNOWN` for semantic independence when evidence is insufficient, then inspect further instead of automatically declaring either safety or failure.

Direct path disjointness is useful evidence, but it does not prove semantic independence. Producer/consumer contracts can collide across different files.

If a branch commit SHA changes while its repository tree remains byte-identical, compare tree identity before triggering another expensive run. Do not reanchor byte-identical work solely to chase the newest `main` coordinate.

For broad instructions such as “keep working,” proceed in coherent milestones. After one bug repair, research discriminator, promotion, external replay, or cleanup reaches a durable boundary, record the result and leave an exact next handoff. Avoid recursively promoting every adjacent descendant unless the requested task still requires it or the user explicitly wants a sustained sweep.

See `docs/concurrent-work-promotion.md` for the durable promotion/reanchoring policy and examples.

## Prevent third-party GitHub backlinks before writing

Cultist frequently studies public GitHub issues, pull requests, reviews, discussions, commits, and source. Internal research can churn aggressively. Human-facing third-party references require a stricter rule because an accidental GitHub cross-reference cannot be undone after the write has been processed.

For an automated worker, the rule is simple:

**Every third-party GitHub reference the worker creates in human-facing interaction text must already be backlink-safe before the GitHub write happens.**

Default to non-linking wording when click-through is unnecessary:

```text
OWNER/REPOSITORY issue 123
OWNER/REPOSITORY PR 456
OWNER/REPOSITORY discussion 789
```

When a link is useful, use the literal redirect host:

```text
https://redirect.github.com/OWNER/REPOSITORY/issues/123
https://redirect.github.com/OWNER/REPOSITORY/pull/456
https://redirect.github.com/OWNER/REPOSITORY/discussions/789
https://redirect.github.com/OWNER/REPOSITORY/commit/SHA
```

Automated human-facing text must not emit direct third-party `github.com` URLs or third-party `OWNER/REPOSITORY#123` shorthand. Do not rely on a post-write workflow to prevent a backlink; at that point the interaction already exists.

Repositories under `teamleaderleo/*` are first-party coordination surfaces for this rule. Normal same-repository references such as `#109` and ordinary owned-repository links are allowed.

Before creating or editing an issue, pull request, comment, review, inline review comment, or discussion that mentions third-party GitHub work, preflight the **exact text that will be written**:

```sh
python3 scripts/external_github_reference_guard.py \
  --repository teamleaderleo/cultist \
  --stdin < proposed-body.md
```

The write happens only after that preflight succeeds. Interaction preflight has no automated evidence-marker exception.

Do not put clickable third-party issue/PR/discussion references, direct URLs, or cross-repository shorthand in commit messages. Prefer a plain project name and item number without GitHub shorthand, or omit the external coordinate from the commit message entirely.

Canonical `github.com` / `api.github.com` provider coordinates may remain exact on machine/evidence surfaces where identity requires them: API inputs, retained JSON receipts, parser fixtures, workflow inputs, copied source payloads, and other non-interaction evidence. In tracked prose written by an agent, prefer non-linking wording or `redirect.github.com` unless exact canonical source text is itself the retained evidence.

The CI reference guard is a detector and cleanup aid. It is not the prevention boundary.

See `docs/external-reference-policy.md` for the durable policy and examples.

## Dogfood the interruption

When Cultist or repository evidence surfaces something during real work, preserve the consequence when it is observable. A small receipt can classify the episode without inventing a universal score:

```text
surfaced -> consulted -> changed next action
surfaced -> consulted -> prevented/reversed a wrong turn
surfaced -> useful context, same action
surfaced -> ignored
surfaced -> irrelevant
surfaced -> stale / wrong coordinate
surfaced -> needed stronger evidence
candidate evidence stayed quiet -> correct negative
```

Record the concrete next action when one changed: another file opened, a test run, a generator invoked, a collaborator contacted, a patch changed, an assumption dropped, a decision preserved, or an investigation stopped because the missing discriminator became explicit.

Issue #137 owns the behavioral product pressure test. It complements the existing research lanes. Continue useful JEI, review, applicability, decision-memory, representation, analyzer, and performance research; use these receipts to decide which evidence deserves prominent automatic delivery and which belongs in quieter or explicit-query views.

## Compose views instead of inventing competing truths

Cultist has several agent-facing research views over shared repository evidence. Treat them as different jobs unless evidence proves otherwise:

- lifecycle work (`brief -> check/diff -> teach`) asks **when** evidence should be recovered or preserved;
- just-enough-information work asks **what** evidence is worth selecting for the current task;
- review intelligence asks **where** reviewer attention should go;
- compact IR / C1 research asks **how** evidence should be represented or transmitted;
- decision memory asks **what reviewed rationale should survive** for later work;
- behavioral evaluation asks **whether surfaced evidence changed justified worker behavior enough to earn the interruption**.

Do not create a new authority/provenance/unknown/freshness vocabulary merely because a new projection needs those facts. Prefer shared evidence primitives plus a task-specific projection.

## Close the loop when it is useful

When evidence is strong enough and the task scope permits it, incorporate the lesson through the smallest appropriate durable surface:

- improve an existing analyzer or evidence contract;
- add a focused regression or negative control;
- record a research receipt;
- update the relevant roadmap/research issue;
- preserve reviewed rationale for future context/decision memory;
- open a focused follow-up issue when the lesson is real but outside the current task.

Avoid unrelated implementation expansion merely because an idea occurred during the task. If the lesson belongs elsewhere, leave an exact handoff with its evidence instead of silently widening scope.

## Agent-facing product test

Recurring questions for work in this repository are:

> What did this task force the worker to discover manually that Cultist could have surfaced, bounded, or preserved for the next worker?

> Which surfaced evidence changed the next justified inspection, validation, coordination, implementation, or preservation step?

If either answer is meaningful, treat the task as dogfood evidence and carry the lesson forward with provenance.

The goal is a repository that becomes easier to work in because prior workers leave behind earned, inspectable knowledge and because current workers receive the evidence that earns their attention at the moment it can still change the work.
