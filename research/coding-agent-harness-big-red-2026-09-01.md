# Big Red coding-agent harness evidence: Codex and Pi

## Decision

**DERIVED:** Keep Codex as the operator-visible control plane and admit Pi as a thinner internal worker backend behind a typed, receipt-bearing local boundary. The completed comparisons do not justify replacing Codex's product surface, and they do justify using Pi for bounded external work: the same model/provider reached the same correctness class with materially less provider input and faster completion in the admitted tasks.

This is not a harness leaderboard. The user stopped the remaining matched comparison after the useful discriminator was clear. The long minute-60 retention and kill/resume treatments therefore remain **UNKNOWN**, and their unfinished protocol defects are preserved below rather than converted into claims.

## Frozen treatment

**PROVEN:** The inspected harnesses were Codex CLI 0.151.0 (`rust-v0.151.0`, source `78c290807ce710180111df227df3b7a4fe845452`, installed binary SHA-256 `9739cbc9…2aaf0c9a`) and `@earendil-works/pi-coding-agent` 0.84.4 (source `b79e4cc834970cca69daebffab7df1da7d1e52c4`, bundled CLI SHA-256 `5406c369…77fd4bf`). Both used the OpenAI Codex Responses provider through ChatGPT subscription OAuth and the same named model/effort within each matched arm.

**PROVEN:** The node was Big Red generation 1: Ubuntu 26.04.1, Linux 7.0.0-30, x86-64, Intel Core Ultra 7 255H, 16 logical CPUs. Canonical context was read from pinned Cultist, Glaeda, Stensibly, and Leo Workspace heads before treatment construction. Experimental installs, sessions, evidence, and worktrees were isolated from canonical configuration.

The full instantiated behavior map is retained privately at `~/.local/state/big-red-harness-eval-20260831/phase1/instantiated-harnesses.md`, SHA-256 `f4cd351dca90f42b3cb9bad93b0566722f5d17fc9909e45cb1c3018ddad1ec28`.

## Measurements

### Fixed tax

**PROVEN:** On one tiny closed edit with the same Sol/high model and OpenAI Codex OAuth provider, both arms produced the same passing result.

| Measure | Codex | Pi |
|---|---:|---:|
| First-request total input | 11,025 | 1,171 |
| Total provider input | 69,009 | 8,087 |
| Completion wall time | 44.5 s | 26.2 s |

**DERIVED:** Pi removed most fixed prompt/tool tax without losing behavior on this closed task. This says nothing about long-session state or containment. Evidence receipt SHA-256: `817c56b91aed0f16175346d802f0e219241ab26e92a19a8e7559cc45c03bf519`.

### Bounded real coding task

**PROVEN:** Both arms independently passed the hidden semantic oracle, current tests, formatter, and Clippy after source/history orientation, a nontrivial edit, an initially misleading hypothesis, and recovery.

| Measure | Codex | Pi |
|---|---:|---:|
| Total provider input | 874,647 | 401,165 |
| Model requests | 20 | 16 |
| Tool calls | 22 | 35 |
| Completion wall time | 491 s | 395 s |

**OBSERVED:** Neither arm compacted, required human help, or cognitively rediscovered unchanged evidence. Pi used less provider context and time but more fragmented tool/source interaction. The original evaluator falsely rejected both correct candidates because it coupled to exact error text; that negative receipt is retained. Evidence SHA-256: `277d1a564cabfba8559b5bcb09e0d88d025cd8b7cd8daefb9b080b9f3fbf422a`.

### Physical campaign

**PROVEN:** Both arms produced the same Rust release binary SHA-256 `16f8f048…d65a126` and passed independent semantic verification on a two-scenario physical peak-RSS campaign.

| Measure | Codex | Pi |
|---|---:|---:|
| Wall time | 1,358.0 s | 822.2 s |
| Provider input | 3,798,796 | 1,771,449 |
| Cached input | 3,648,768 | 1,645,056 |
| Uncached input | 150,028 | 126,393 |
| Output | 33,650 | 19,573 |
| Reasoning output | 13,135 | 5,788 |
| Model requests | 44 | 31 |
| Tool calls | 31 | 54 |

**PROVEN:** Pi delivered a clean four-commit worktree with measured source/evidence reachable in Git. Codex edited the checkout but its linked-worktree sandbox could not create the shared Git index lock outside the writable root, leaving the valid candidate/evidence untracked and the temporary measured source not Git-reachable.

**OBSERVED:** Pi was 39.5% faster, used 53.4% less total input and 15.8% less uncached input, and produced the stronger retained Git artifact. It also used substantially more source/tool operations and had weaker containment: plain Pi has no built-in sandbox, the benchmark build was not explicitly offline, and its output path was too permissive. Evidence SHA-256: `905504f8547254901ef31fb7be824c724ae05485de5585371b2d3520a23ad62b`.

### External Luna review wave

**PROVEN:** Four matched read-only review shards ran as eight independent Luna/max CLI processes. All exited successfully.

| Harness | Workers | Input | Cached | Uncached | Output | Reasoning | Tool calls |
|---|---:|---:|---:|---:|---:|---:|---:|
| Codex | 4 | 6,851,640 | 6,426,368 | 425,272 | 119,293 | 97,297 | 141 |
| Pi | 4 | 3,093,189 | 2,813,440 | 279,749 | 111,997 | 95,160 | 107 |

**OBSERVED:** Independent agreement found real pre-merge defects in the model-accounting and repeated-prefix patches, selected Pi's Phase 4 delivery with explicit reservations, and blocked the unsafe kill protocol. Pi used 54.9% less total and 34.2% less uncached input across the review wave. Raw event streams were far too verbose for commander ingestion, especially Pi token-update JSONL; only deterministic bounded projections should enter hot context. Fleet evidence SHA-256 after preserving the later timeout receipts: `34fc0e929a0a2e4ce8b13549ccae06dfc188dfb230ecf1dff5d16326a73fd34c`.

### First operational Pi worker use

**PROVEN:** Codex used Pi as an internal worker—not a benchmark arm—to make a bounded 24-line Leo Workspace documentation edit through six typed tools with no generic shell. Attempt 1 completed in 83.385 seconds using 57,595 input (41,472 cached), 3,972 output, and 2,990 reasoning tokens. Its edit and focused tests were correct, but linked-worktree Git metadata was not mounted inside bubblewrap, so `git diff --check` failed and Pi truthfully returned `partial`.

**PROVEN:** Codex restarted Pi through the runner's intended session path with the same session ID, added only the missing read-only main-checkout `.git` mount, and asked it to continue. Attempt 2 completed in 11.085 seconds using 24,313 input (22,528 cached), 231 output, and 118 reasoning tokens; `git diff --check` passed and Pi returned `complete`. No conversation, failed approach, edit, or next action was reconstructed by hand. Host-side review then passed all 14 focused tests and the diff check before merge.

**PROVEN:** The canonical runner subsequently completed a separate read-only self-review with no edits and provider-visible usage of 36,429 input (30,208 cached), 4,944 output, and 3,944 reasoning tokens. The compact review projection carries only backend/state, paths, usage, result status, verification count, and blocker. It deliberately marks textual worker verification claims `ambiguous` until the commander checks real evidence; stdout, stderr, transcript, and summary prose remain cold.

## What belongs where

**DERIVED:** Codex buys the human-facing app/desktop/mobile/CLI surface, an integrated permission and sandbox model, owned process/background lifecycle, structured planning and collaboration, Git/diff awareness, compaction, and first-party thread recovery. Its costs in these treatments were a much larger fixed prompt/tool surface, greater provider input, and a concrete linked-worktree sandbox mismatch.

**DERIVED:** Pi buys a small prompt/tool loop, explicit extension/RPC/session seams, lower observed context tax, and easy external process fan-out. Its costs are equally concrete: no built-in sandbox, no native Git/process/planning authority, more fragmented observations in the physical task, and verbose raw streaming events. A generic Pi `bash` tool shares the host user's authority and can reach Pi OAuth state; it is not admissible for unattended canonical work.

The intended boundary is:

- Codex: visible commander and integration/adjudication surface.
- Pi: internal long-running or bounded worker process.
- Git/JJ: source and edit identity.
- Glaeda: deterministic execution/verification.
- Cultist: evidence identity, negative results, and failed-approach memory.
- Stensibly: typed launch, authority, timeout, receipt, and compact worker-glance boundary.

**INFERRED:** Pi can be an equally good or better worker when the typed environment supplies repository orientation, scoped patching, fixed verification, exact receipts, and compact recovery state. It should not be made to imitate Codex with another giant hot tool catalogue. Provider OAuth stays in the host Pi process; model-visible operations must be worktree-scoped, argv-only where they execute, environment-scrubbed, bounded, and unable to expose Pi state.

## Preserved negative results and unknowns

- **UNKNOWN:** At minute 60–75, whether either harness retains failed approaches and evidence identity better. Both physical arms completed before minute 23.
- **UNKNOWN:** Matched forced-kill recovery fidelity. The proposed kill controller failed admission review because it did not enforce a clean frozen base, exact successful landmark, empty process group, or original/resumed session lineage. The treatment was canceled, not passed. The ordinary Pi process restart above proves intended same-session continuation after a correctable environment failure, not forced-kill equivalence.
- **PROVEN:** A first Pi retry failed before provider contact because it pointed at an empty isolated profile. The valid run used the admitted OAuth state.
- **PROVEN:** A first long Codex controller buffered provider output until normal exit and lost it on interruption; incremental durable capture was subsequently added.
- **OBSERVED:** Three later broad Codex/Luna implementation workers hit their 30-minute timeout after producing work, but their settlement-only receipts retained `codex.tokenUsage: null`. Killed ephemeral Codex CLI turns therefore remain usage-accounting blind unless provider usage is captured incrementally.
- **OBSERVED:** The first Pi-backend implementation dispatch was rejected by Stensibly before launch because `bun` was missing from the worker's minimal effective PATH. The harness failed closed and the retry used the exact installed executable path. The operator mistakenly reused the same run directory for the retry, whose lifecycle clearing removed the first receipt; preserve this as a fleet-controller defect, not as a durable-receipt claim.

## Consequence

**DERIVED:** Stop expanding the harness survey. Operationalize Pi behind Stensibly, retain Codex as the visible commander, separate Sol/Luna cached/uncached/output/reasoning accounting, and supervise fleets through bounded glances rather than worker narration. Reopen a comparison only for a concrete discriminator such as demonstrated long-session rediscovery or resume loss.
