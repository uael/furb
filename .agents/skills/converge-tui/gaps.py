"""Print, as JSON, the gaps that count among the answers of the judges of an audit of the TUI: each gap names a capture
that its judge read, and quotes the rulings exactly, in twelve characters at least, once.

Usage: python3 -I gaps.py <rulings file> <capture>... < answers, a JSON list of the answer of each judge.
"""

import json
import sys
from pathlib import Path

FIELDS = 3
SHORTEST = 12


def plain(text: str) -> str:
  for mark in ">*_`\"'\u201c\u201d":
    text = text.replace(mark, " ")
  return " ".join(text.split()).lower()


def counted(rulings: str, names: set[str], answers: list) -> list[list[str]]:
  said, found, seen = plain(rulings), [], set()
  for answer in answers:
    for one in answer if isinstance(answer, list) else []:
      if not (isinstance(one, list) and len(one) == FIELDS and all(isinstance(part, str) for part in one)):
        continue
      shot, gap, quote = (part.strip() for part in one)
      key = (shot, plain(gap))
      if shot in names and gap and len(plain(quote)) >= SHORTEST and plain(quote) in said and key not in seen:
        seen.add(key)
        found.append([shot, gap, quote])
  return found


if __name__ == "__main__":
  rulings = Path(sys.argv[1]).read_text(encoding="utf-8")
  json.dump(counted(rulings, set(sys.argv[2:]), json.load(sys.stdin)), sys.stdout)
