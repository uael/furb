"""Check that no comment line that a change adds to a Rust file goes past 100 characters.

rustfmt sets the width of the code at 100 and wraps no comment, so a comment goes past it with no gate that says so. This
check reads the lines that the working tree adds to a base, the merge base with origin/main by default, and fails on an
added line that holds only a comment, a doc comment or a part of one, and is longer than WIDTH. The lines that main
holds already are no debt of the change, and each goes with the next change of its file. A line inside a string, as a
fixture of another language, is no comment. A line of code is rustfmt's: it leaves a long string as it stands.

Usage: python3 script/rustwidth.py [base]
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WIDTH = 100

OPENERS = re.compile(r"(?:b?r(#*)\")|(?:b?\")")


def block(text: str, i: int, line: int, start: int) -> tuple[int, int, int]:
  """The place after a block comment that opens at i, which nests."""
  depth, i = 1, i + 2
  while i < len(text) and depth:
    if text.startswith("/*", i):
      depth, i = depth + 1, i + 2
    elif text.startswith("*/", i):
      depth, i = depth - 1, i + 2
    else:
      line, start = (line + 1, i + 1) if text[i] == "\n" else (line, start)
      i += 1
  return i, line, start


def string(text: str, m: re.Match[str], line: int, start: int) -> tuple[int, int, int]:
  """The place after a string that m opens, raw or not, and a backslash that ends a line counts that line."""
  raw, close, i = m[0].count("r") and m[1] is not None, '"' + (m[1] or ""), m.end()
  while i < len(text):
    if text[i] == "\n":
      line, start = line + 1, i + 1
    if raw and text.startswith(close, i):
      return i + len(close), line, start
    if not raw and text[i] == '"':
      return i + 1, line, start
    if not raw and text[i] == "\\":
      if text[i + 1 : i + 2] == "\n":
        line, start = line + 1, i + 2
      i += 1
    i += 1
  return i, line, start


def quote(text: str, i: int) -> int:
  """The place after a quote that is a character or a lifetime."""
  if text.startswith("'\\", i):
    end = text.find("'", i + 3)
    return len(text) if end < 0 else end + 1
  return i + (3 if text[i + 2 : i + 3] == "'" else 1)


def comments(text: str) -> list[int]:
  """The numbers, from 1, of the lines that hold only a comment, outside each string and each block comment."""
  found, i, line, start = [], 0, 1, 0
  while i < len(text):
    if text[i] == "\n":
      line, start, i = line + 1, i + 1, i + 1
    elif text.startswith("//", i):
      if not text[start:i].strip():
        found.append(line)
      i = text.find("\n", i) % (len(text) + 1)
    elif text.startswith("/*", i):
      i, line, start = block(text, i, line, start)
    elif (m := OPENERS.match(text, i)) and not (i and (text[i - 1].isalnum() or text[i - 1] == "_")):
      i, line, start = string(text, m, line, start)
    else:
      i = quote(text, i) if text[i] == "'" else i + 1
  return found


def long(text: str) -> list[int]:
  """The numbers of the comment lines that go past WIDTH."""
  lines = text.split("\n")
  return [n for n in comments(text) if len(lines[n - 1]) > WIDTH]


def failures(texts: dict[str, str], added: dict[str, set[int]]) -> list[str]:
  """Each added line of a text that is a comment past WIDTH, as `file:line: ...`."""
  return [
    f"{file}:{n}: a comment line that this change adds goes past {WIDTH} characters"
    for file, nums in sorted(added.items())
    for n in long(texts.get(file, ""))
    if n in nums
  ]


def git(*args: str) -> subprocess.CompletedProcess[str]:
  return subprocess.run([shutil.which("git") or "git", *args], cwd=ROOT, capture_output=True, check=False, text=True)  # noqa: S603


def base() -> str:
  for ref in ("origin/main", "main"):
    got = git("merge-base", "HEAD", ref)
    if got.returncode == 0:
      return got.stdout.strip()
  return "HEAD"


def parse(diff: str) -> dict[str, set[int]]:
  """The numbers of the lines that a diff with no context adds, for each file."""
  file, added = "", {}
  for row in diff.splitlines():
    if row.startswith("+++ "):
      file = row[6:] if row.startswith("+++ b/") else ""
    elif file and (m := re.match(r"@@ -\S+ \+(\d+)(?:,(\d+))? @@", row)):
      first, count = int(m[1]), int(m[2] if m[2] is not None else 1)
      added.setdefault(file, set()).update(range(first, first + count))
  return added


def main(argv: list[str] | None = None) -> int:
  args = sys.argv[1:] if argv is None else argv
  added = parse(git("diff", "-U0", args[0] if args else base(), "--", "*.rs").stdout)
  texts = {x: (ROOT / x).read_text(encoding="utf-8", errors="replace") for x in added if (ROOT / x).is_file()}
  found = failures(texts, added)
  for line in found:
    print(line)  # noqa: T201
  return 1 if found else 0


if __name__ == "__main__":
  sys.exit(main())
