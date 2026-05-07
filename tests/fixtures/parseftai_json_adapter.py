#!/usr/bin/env python3
"""Adapter: parse a .ftai file with the upstream Python parser and emit
its tag/key/value structure as JSON.

This is a thin wrapper around `parseftai_linter.py` that reuses
`parse_ftai_with_lines()` to reach a structured representation, then
projects it into a stable JSON shape the Rust parity test can compare.

Output JSON shape:

    {
      "intent_fail": false,
      "tags": [
        {
          "tag": "@document",
          "line": 3,
          "body": [["title:", "Sodium Bicarbonate"], ...]
        },
        ...
      ]
    }

This shape is intentionally minimal — the upstream Python parser is a
linter, not a full AST builder, so we expose only what it actually
records.

Usage:  python3 parseftai_json_adapter.py <path/to/file.ftai>
"""

from __future__ import annotations

import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from parseftai_linter import parse_ftai_with_lines  # type: ignore


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: parseftai_json_adapter.py <file.ftai>", file=sys.stderr)
        return 2
    tag_data, _syntax_errors, intent_fail = parse_ftai_with_lines(argv[1])
    out = {
        "intent_fail": bool(intent_fail),
        "tags": [
            {
                "tag": tag.split()[0] if tag else "",
                "line": line,
                "body": [list(pair) for pair in body],
            }
            for tag, body, line in tag_data
        ],
    }
    json.dump(out, sys.stdout, sort_keys=True)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
