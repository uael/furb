"""Check that the title of a pull request is a conventional commit of a type that release-please reads here.

The types are those of `changelog-sections` in `release-please-config.json`, which is their one home. A title is a
type, a scope in parentheses if any, a ! for a breaking change if any, then a colon, a space and a sentence that
starts with a capital letter, as `fix: Reap the shell of a command that drops before its exit`.

Usage: python3 script/title.py "<title>"
"""

import json
import re
import sys
from pathlib import Path

CONFIG = Path(__file__).resolve().parent.parent / "release-please-config.json"


def types() -> list[str]:
  return [str(x["type"]) for x in json.loads(CONFIG.read_text())["changelog-sections"]]


def fits(title: str, kinds: list[str]) -> bool:
  return re.fullmatch(rf"(?:{'|'.join(map(re.escape, kinds))})(?:\([^()\s]+\))?!?: [A-Z][^\n]*", title) is not None


def main() -> int:
  title, kinds = " ".join(sys.argv[1:]), types()
  if fits(title, kinds):
    sys.stdout.write(f"The title is right: {title}\n")
    return 0
  sys.stderr.write(
    f"The title is wrong: {title!r}\n"
    f"Start it with a type of {', '.join(kinds)}, a scope in parentheses if any, a ! for a breaking change if any,"
    " then a colon, a space and a sentence that starts with a capital letter, as"
    " 'fix: Reap the shell of a command that drops before its exit'.\n"
  )
  return 1


if __name__ == "__main__":
  sys.exit(main())
