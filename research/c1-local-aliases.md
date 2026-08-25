# C1 local alias research

Issue: #155

This experiment keeps canonical C1 as the semantic source and applies a reversible report-local string table only after ordinary C1 encoding.

The retained external input is the raw `AnalysisReport` captured from the historical SmolRunner replay used by closed evidence carrier #143. Its old byte measurements are historical receipts only. This branch re-encodes that raw report with current `main` before measuring alias savings.

Candidate rule:

```text
repeated canonical JSON string literal
+ replacement bytes saved > alias declaration bytes
-> eligible local alias
```

Alias numbers are packet-local and carry no durable identity. Expansion must reproduce canonical C1 byte-for-byte before the existing C1 decoder runs. Inputs that do not earn positive total savings remain ordinary C1.

The executable controls live in `tests/c1_local_alias.rs` and cover:

- fresh current-C1 measurement over the retained external report;
- byte-identical expansion and ordinary C1 decode;
- deterministic alias selection;
- a representative current report;
- a tiny no-repetition report that stays plain C1;
- rejection of a frequent low-value one-character literal;
- malformed definitions, duplicate values, unknown references, and references inside JSON strings.

Exact current byte results are recorded on the PR after hosted CI executes the branch.
