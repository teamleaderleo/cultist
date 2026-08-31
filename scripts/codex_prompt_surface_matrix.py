#!/usr/bin/env python3
"""Measure current Codex prompt surfaces under task-scoped capability profiles."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from codex_prompt_token_report import arm, canonical, digest


PROFILES = {
    "default": [],
    "skills-muted": ["skills.include_instructions=false"],
    "plugins-off": ["features.plugins=false"],
    "apps-off": ["features.apps=false"],
    "skills-plugins-off": ["skills.include_instructions=false", "features.plugins=false"],
    "skills-apps-off": ["skills.include_instructions=false", "features.apps=false"],
    "multi-agent-off": ["features.multi_agent=false"],
    "all-off": [
        "skills.include_instructions=false",
        "features.plugins=false",
        "features.apps=false",
    ],
}


def private_write(path: Path, value: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as sink:
        sink.write(value)
        sink.flush()
        os.fsync(sink.fileno())


def component_presence(summary: dict[str, Any]) -> dict[str, bool]:
    labels = {row["label"] for row in summary["segments"]}
    return {
        "skillsCatalogue": "skills-catalogue" in labels,
        "recommendedPlugins": "recommended-plugins" in labels,
        "appsInstructions": "apps-instructions" in labels,
        "pluginsInstructions": "plugins-instructions" in labels,
    }


def comparison(default_tokens: int, current_tokens: int) -> dict[str, Any]:
    delta = current_tokens - default_tokens
    return {
        "textTokenDeltaFromDefault": delta,
        "textTokenReductionPercentFromDefault": round(
            -delta * 100 / default_tokens, 3
        ) if default_tokens else 0.0,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--cwd", type=Path, required=True)
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--encoding", default="o200k_base")
    args = parser.parse_args()
    try:
        import tiktoken
    except ImportError as error:
        raise SystemExit("tiktoken is required for an OpenAI-tokenizer report") from error
    output = args.output_dir.expanduser().resolve()
    if output.exists():
        raise SystemExit("refusing to overwrite prompt-surface matrix")
    output.mkdir(parents=True, mode=0o700)
    os.chmod(output, 0o700)
    encoder = tiktoken.get_encoding(args.encoding)
    rows: list[dict[str, Any]] = []
    for name, overrides in PROFILES.items():
        argv = [str(args.codex), "debug", "prompt-input"]
        for override in overrides:
            argv.extend(["-c", override])
        argv.append(args.prompt)
        process = subprocess.run(
            argv, cwd=args.cwd, check=False, capture_output=True, text=True
        )
        if process.returncode != 0 or process.stderr:
            raise SystemExit(f"prompt-input profile {name} failed clean execution")
        raw = process.stdout.encode("utf-8")
        private_write(output / f"{name}.raw.json", raw)
        summary = arm(json.loads(process.stdout), encoder)
        rows.append({
            "profile": name,
            "overrides": overrides,
            "promptInputSha256": hashlib.sha256(raw).hexdigest(),
            "summary": summary,
            "components": component_presence(summary),
        })
    default_tokens = rows[0]["summary"]["textTokens"]
    for row in rows:
        row["comparison"] = comparison(default_tokens, row["summary"]["textTokens"])
    result = {
        "schema": "cultist-codex-prompt-surface-matrix/v1",
        "recordedAt": datetime.now(timezone.utc).isoformat(),
        "codexVersion": subprocess.run(
            [str(args.codex), "--version"], check=True, capture_output=True, text=True
        ).stdout.strip(),
        "encoding": args.encoding,
        "promptSha256": digest(args.prompt.encode("utf-8")),
        "profiles": rows,
        "countsTextSegmentsOnly": True,
        "countsProtocolOverhead": False,
        "broadProfilesRetireCapabilities": True,
        "rawContentEmitted": False,
        "authorizesGlobalDefaultChange": False,
        "authorizesProductionPromotion": False,
    }
    payload = canonical(result) + b"\n"
    private_write(output / "result.json", payload)
    print(json.dumps({
        "schema": "cultist-codex-prompt-surface-matrix-receipt/v1",
        "resultSha256": digest(payload),
        "profileCount": len(rows),
        "defaultTextTokens": default_tokens,
        "allOffTextTokens": rows[-1]["summary"]["textTokens"],
        "allOffReductionPercent": rows[-1]["comparison"]["textTokenReductionPercentFromDefault"],
        "broadProfilesRetireCapabilities": True,
        "rawContentEmitted": False,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
