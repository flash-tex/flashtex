#!/usr/bin/env python3
"""Generate the runtime-v1 demo request and verify the compiler's response."""

import json
from pathlib import Path
import subprocess
import sys


COMPILER = Path(
    "/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/"
    "e30fd4a4-f46a-4c3f-a28c-cbb8617b4425/scratchpad/"
    "wt-compiler/crates/compiler/target/release/flashtex-compiler"
)


def main():
    samples = Path(__file__).resolve().parent
    request = {
        "protocol_version": 1,
        "id": "sample-demo-1",
        "type": "compile",
        "payload": {
            "project_id": "demo",
            "revision": 1,
            "entry_path": "main.tex",
            "documents": [
                {"path": "main.tex", "text": (samples / "demo.tex").read_text(encoding="utf-8")}
            ],
        },
    }
    request_line = json.dumps(request, ensure_ascii=False) + "\n"
    (samples / "demo-request.json").write_text(request_line, encoding="utf-8")
    completed = subprocess.run(
        [str(COMPILER)], input=request_line, capture_output=True,
        encoding="utf-8", timeout=30,
    )
    (samples / "demo-result.json").write_text(completed.stdout, encoding="utf-8")
    if completed.returncode:
        raise RuntimeError(
            f"Compiler exited with code {completed.returncode}: {completed.stderr.strip()}"
        )
    if len(completed.stdout.splitlines()) != 1:
        raise ValueError("Expected exactly one compiler output line")
    result = json.loads(completed.stdout)
    if (result.get("protocol_version") != 1
            or result.get("id") != request["id"]
            or result.get("type") != "compile_result"):
        raise ValueError("Expected a matching runtime-v1 compile_result envelope")
    payload = result["payload"]
    status = payload["status"]
    pages = len(payload["pages"])
    diagnostics = len(payload["diagnostics"])
    print(f"status: {status}; page count: {pages}; diagnostic count: {diagnostics}")
    if status != "ok" or diagnostics != 0 or pages < 2:
        raise ValueError("Demo requires status ok, zero diagnostics, and at least two pages")


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, RuntimeError) as error:
        print(f"Demo generation failed: {error}", file=sys.stderr)
        sys.exit(1)
