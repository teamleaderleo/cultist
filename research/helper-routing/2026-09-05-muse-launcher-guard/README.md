# First Muse helper routing receipt

The selected launcher model-identity repair was accepted after independent review and a focused
test rerun. The owner reports zero cost in two helper wrapper receipts. End-to-end cost, repair
effort, time, and task tokens remain **UNKNOWN**. This is one accepted task, not evidence that a
routing strategy wins.

Source: [Cultist #383 first trial](https://github.com/teamleaderleo/cultist/issues/383#issuecomment-5549768574).
The [merged repair](https://github.com/teamleaderleo/compute-node-bootstrap/pull/35) was checked
against merge commit `56a07f63bed0ac50eeff2f1fa6c5ae460dea9c27` when retaining this receipt.
The provider/process fields remain unknown because the comment does not supply those events;
accepted work does not retroactively prove each intermediate process succeeded.

## Reuse

From the repository root:

```bash
python3 scripts/helper_routing_evidence.py research/helper-routing/2026-09-05-muse-launcher-guard/result.json
python3 scripts/helper_routing_evidence_test.py
```

The offline summarizer consumes `result.json` research receipts and prints descriptive JSON. It
does not dispatch, poll, change a ledger, fetch sources, or infer a routing policy. This receipt
is the capture example: retain exact task/evidence references; use JSON `null` for unknowns;
record the four classes already specified by #383; and distinguish classifications made before
dispatch from retrospective judgments. Future cohorts can place multiple task records in one
receipt. Each canonical task reference must occur once so repeated attempts do not inflate the
accepted-task denominator.

The four outcome fields are independent observations: explicit provider success, process
completion, independent verification, and acceptance. Record process failure even when a provider
emits success. A rejected or unverified proposal does not become accepted because another proposal
from the same worker did.

For each task, capture retry count, human/agent repair minutes, end-to-end wall seconds, and total
task tokens only when evidence establishes their scope. A follow-on implementation after review
is not necessarily a retry. Provider usage retains its own metric, unit, scope, and evidence;
an empty list means no observed usage, not free work. Unknown units stay null. Include reviewer
and repair consumption when measuring end-to-end efficiency, and avoid overlapping usage rows.

Groups separate route, classification timing, and all four difficulty classes. Metric totals
cover known observations only and include known/unknown task counts; an all-unknown total remains
null. Provider usage is retained per task rather than combined across currencies, units, or
partial scopes. Source records and limitations survive aggregation. Even fully populated groups
are descriptive, not matched experimental arms: comparisons still need predeclared cohorts and
review of confounding factors.

The next useful experiment is a small real-task cohort with pre-dispatch classification and
complete review/repair accounting. Preserve failures and missing data alongside accepted work.
