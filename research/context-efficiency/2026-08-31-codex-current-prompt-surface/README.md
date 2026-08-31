# Current Codex prompt-surface ablation

This lane moved from the historical 17.7K Luna run to the current model-visible
Codex surface. The historical pair remains provenance; it is not relabelled as a
current baseline.

## Result

On Big Red with Codex CLI 0.151.0, a ten-character closed prompt carried 4,152
`o200k_base` text tokens before protocol overhead. The two largest eager segments
were the skills catalogue (1,655) and recommended-plugin list (1,389).

The current client produced these deterministic prompt projections:

| profile | text tokens | delta | reduction |
| --- | ---: | ---: | ---: |
| default | 4,152 | 0 | 0.0% |
| skills catalogue muted | 2,497 | -1,655 | 39.9% |
| plugins off | 1,598 | -2,554 | 61.5% |
| apps off | 2,617 | -1,535 | 37.0% |
| skills + plugins off | 899 | -3,253 | 78.3% |
| skills + apps off | 962 | -3,190 | 76.8% |
| multi-agent feature off | 4,152 | 0 | 0.0% |
| skills + plugins + apps off | 753 | -3,399 | 81.9% |

`features.plugins=false` and `features.apps=false` are capability changes, not pure
context retirement. They belong in an explicitly selected closed-task runner
profile, never an inferred or global default.

`features.multi_agent=false` did not remove the host-injected team-orchestration
or multi-agent-mode segments. That zero-saving control is retained; the CLI feature
flag is not the owner of those prompt bytes.

A live provider replay on a capability-free current task reduced reported input from
19,471 to 10,712 tokens (-8,759; 45.0%). Both arms returned the same exact `OK`
output. The run used the Codex client's built-in default model under
`--ignore-user-config`; the event stream did not identify that model, so the result
does not invent a model name. This microtask validates the real request-envelope
saving, not broad engineering capability preservation.

The explicit-skill gate also passed on a live `gpt-5.6-sol:xhigh` run. Both the
ordinary and catalogue-muted arms issued two command executions referencing the
named `lazy-commander` skill and `SKILL.md`, read the file, and returned the same
structured observation with the exact `# Lazy Commander` heading. Muting the eager
catalogue reduced provider-reported input from 43,062 to 30,759 tokens (-12,303;
28.6%). This establishes progressive disclosure for this named installed skill on
the pinned client/machine path; it does not establish every skill or broad plugin/app
retirement.

Three fresh behavioral replays with only the skills catalogue muted preserved the
same first justified action. Provider input-token savings were -698, -698, and -490,
so the useful conclusion is a non-zero saving with a retained first-action null—not
an exact-token constant or a general capability claim.

## Deterministic output

- `codex_prompt_input_probe.py` proves effective model-visible catalogue removal.
- `codex_prompt_token_report.py` counts each prompt segment with the OpenAI tokenizer
  without emitting segment text.
- `codex_prompt_surface_matrix.py` compares exact current client profiles and keeps
  raw prompt projections private.
- `codex_context_ablation_run.py` and its aggregator retain fresh usage evidence and
  quiet/null behavior.

Stensibly consumes the finding through a typed prompt-surface profile compiler. The
compiler requires an explicit profile choice and receipts both context and capability
retirements; task text is never used to guess missing capabilities.

## Limits

Tokenizer counts cover model-visible text segments, not provider protocol framing,
tool schemas, or other server-side overhead. A debug renderer probe using a literal
`$lazy-commander` did not expose the skill body after catalogue suppression. That is
a retained negative result for the probe method, not evidence that real runtime
explicit-skill invocation works or fails.
The live command-event replay supersedes that method limitation for the one pinned
`lazy-commander` case while retaining the renderer null as evidence about the probe.
