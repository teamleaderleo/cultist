# Luna Max blind Stensibly guard-detail pair

## In simple words

Two fresh, subscription-backed Codex CLI sessions ran the frozen blind pair from issue #267. Both chose `inspect_accepted_guard_detail` as their first action. The pair was admitted with no confound reasons, so the selected accepted-guard detail did not change Luna Max's first action in this one pair.

This is a retained null result. It does not show that the detail is useless generally; the control packet already named the accepted guard, its discriminator, and its enforcement path, which was enough to make this Luna configuration inspect it before deciding.

## Exact run

- Organizer repository head: `55b0d027c9a711bd9423f829cc03d73c48a8d94a`
- Frozen artifact: workflow run `32271062286`, artifact `9372168346`
- Artifact digest: `sha256:d4de6fe63a7088a921cbd6ea1a3c386f22a5620c5c25a71f34edc5bb11fd23cd`
- Plan fingerprint: `cultist-behavioral-trial-plan-sha256-v1:1aca6332c77ed72b49cb20593f215c7eb2952121ad9bf3d5ae60bea0df5df024`
- Worker: `gpt-5.6-luna`, reasoning effort `max`
- Harness: Codex CLI `0.146.0`, authenticated with ChatGPT; no API key was configured
- Session mode: fresh `--ephemeral` processes, user config ignored, read-only sandbox, no repository context, no web search
- Input: one canonical packet file on stdin per session; organizer arm mapping was not supplied
- Output: the same closed JSON schema for both sessions

The fixed invocation shape was:

```sh
codex -a never -s read-only exec \
  --ephemeral --ignore-user-config --skip-git-repo-check --json \
  -m gpt-5.6-luna -c 'model_reasoning_effort="max"' \
  --output-schema observation.schema.json - < packet.json
```

`sampling-config.json` is the canonical fixed configuration. Its SHA-256 is `4fd03ccfd05f841af5acc11007b6bbd9f72def171a45c2cb365c9fb781438297`.

## Result

| Sequence | Packet fingerprint suffix | Session | First action | Input tokens | Output tokens | Reasoning tokens |
| --- | --- | --- | --- | ---: | ---: | ---: |
| 1 | `5fa460fe...52887d` | `01a03613-ab0c-7970-b38e-5c734e920e3b` | `inspect_accepted_guard_detail` | 17,672 | 391 | 219 |
| 2 | `a80303b5...38549` | `01a03613-ab37-7d82-8ac5-5b53b32de3cb` | `inspect_accepted_guard_detail` | 17,815 | 506 | 331 |

`admission.json` is the canonical organizer verdict:

- `verdict: admitted`
- `reasons: []`
- `fresh_uncontaminated_sessions: true`
- `distinct_arm_coverage: true`
- `same_first_action: true`
- no automatic effect or generalization claim

`pair.json` retains both canonical packet byte strings, run metadata, exact raw worker-output byte strings, and their computed hashes. `events-1.jsonl` and `events-2.jsonl` retain the fresh Codex thread IDs, exact structured outputs, and usage. Their hashes are the freshness receipts carried by the run metadata.

## Harness observations

The first launch command placed the global approval flag after `codex exec`; CLI argument parsing rejected both commands before a session started. The corrected command moved global flags before `exec`. No worker evidence was produced by the rejected commands, so they are a harness setup failure rather than a confounded behavioral pair.

Both successful sessions also emitted the same non-fatal harness warning that installed skill descriptions had been shortened to fit the skills context budget. User configuration was ignored, but installed skills remained visible to the CLI runtime. The workers had no repository or network affordance and returned valid observations. A next blind run should test whether disabling irrelevant skills reduces the roughly 17.7K input-token overhead without changing the first-action distribution.

The CLI also logged a local model-cache decode error before connecting, then completed both ChatGPT-backed runs successfully. This is a harness maintenance issue, not a worker-result defect.

## Decision and boundary

Retain the pair as the first real Luna calibration receipt and close the missing-external-execution gap in #267. Do not use it alone to retire the accepted-guard detail or claim treatment equivalence. A later pair can vary the worker tier or remove the control packet's explicit guard pointer if the product question needs a stronger discriminator.

The next Sol/Luna engineering run should keep the closed worker receipt but use a leaner output contract: exact identity and checks are high-signal; repeating objective prose in the receipt is not.
