# Helper routing comparative evidence and negative control

This research receipt records a bounded four-task cohort under [Cultist #383](https://github.com/teamleaderleo/cultist/issues/383) evaluating cheap-worker routing against verification oracle strength, false provider success, and physical execution receipt projection from [Stensibly #1841](https://github.com/teamleaderleo/stensibly/pull/1841).

## Hypothesis and Evaluation

Under Cultist #383, the routing discriminator for cheap workers (Muse/Luna/Gemini class) is not benchmark score alone, but the cost and reliability of detecting a wrong answer:
* **Strong oracle, low ambiguity, local coupling, low failure cost**: observed 2/2 acceptance in this single-arm retrospective cell when bounded deterministic tests verified behavior; no matched counter-route, so no causal routing claim.
* **Negative control**: cheap worker self-reports (`provider_success: true`) are uncoupled from real success; without independent machine verification, undetected failures require human/commander escalation.
* **Execution vs Acceptance separation**: physical test execution receipts (from Stensibly #1841) establish command verification, but full task acceptance remains distinct.

## Cohort Breakdown

1. **`compute-node-bootstrap#35`** ([PR #35](https://github.com/teamleaderleo/compute-node-bootstrap/pull/35), commit `56a07f63bed0ac50eeff2f1fa6c5ae460dea9c27`):
   * Class: strong oracle, low ambiguity, local coupling, low failure cost (`cheap-first`, retrospective).
   * Result: Muse implemented model-identity guard; bash regression suite verified behavior; Codex independently reviewed diff; merged and accepted (cost: 0).
2. **`compute-node-bootstrap#36`** ([PR #36](https://github.com/teamleaderleo/compute-node-bootstrap/pull/36), commit `6c52c6710a7d824229d9e89a6ea840a5581df40c`):
   * Class: strong oracle, low ambiguity, local coupling, low failure cost (`cheap-first`, retrospective).
   * Result: Muse implemented peer deadline kill-after grace; verified by positive tests (exit 137) and a negative control omitting grace (exit 124); merged and accepted (cost: 0).
3. **`stensibly#1821` Negative Control** ([PR #1821](https://github.com/teamleaderleo/stensibly/pull/1821)):
   * Class: strong oracle, low ambiguity, subsystem coupling, medium failure cost (`cheap-first`, retrospective).
   * Result: Defective attempt (task_ref attempt-scoped) emitted structured `SUCCESS` event followed by exit code 7; `provider_success: true`, `process_completed: false`, `verified: false`, `accepted: false`. Required follow-on repair in PR #1840/#1841. This attempt-level rejection is distinct from the PR's final merge state. Proves worker self-report cannot substitute for machine oracle.
4. **`stensibly-1841-adapter-verify`** ([Stensibly PR #1841](https://github.com/teamleaderleo/stensibly/pull/1841)):
   * Class: unrouted workstation execution projection (`route: null`, `classification: null`).
   * Result: Named `verify_focused` check succeeded (`verified: true`); parent task acceptance and process completion remain `unknown`.

## Mechanical Improvements

The offline summarizer (`scripts/helper_routing_evidence.py`) now addresses three mechanical bottlenecks:
1. **Stdin & Multi-Source Streaming**: Directly consumes piped projections (`bun scripts/helper-routing-evidence.ts < results.json | python3 scripts/helper_routing_evidence.py -`) or multiple files.
2. **Evidence Reconciliation (`--reconcile`)**: Binds Stensibly workstation execution receipts with outcome/classification annotations by exact `task_ref`, failing closed on conflicting values.
3. **Cohort Comparison (`--compare`)**: Computes exact acceptance/verification rates and metric burdens per accepted task over known denominators only, without imputing zero for missing data.

## Reproduction

```bash
python3 scripts/helper_routing_evidence.py research/helper-routing/2026-09-05-peer-helper-comparison/result.json --compare
python3 scripts/helper_routing_evidence_test.py
```
