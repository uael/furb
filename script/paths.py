"""Check that each path of this repository that a script or a workflow names stands.

A move of a file in one pull request leaves its old path in a script that no test of that request runs, and the script
fails later. So this check reads each file of script/ and of .github/workflows/, and it fails on a path whose first
part is a folder of the repository and that is neither a file nor a folder that git holds. A path that a build makes
and git ignores stands, a path that a script makes outside the repository is listed in MADE, and a path with a variable
or a pattern after it is not a whole path, so it is not checked. A path after `$root/`, which the scripts use for the
root of the repository, is checked as a path of the repository.

Usage: python3 script/paths.py
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WATCHED = ("script/", ".github/workflows/")
# The names that a script gives to the root of the repository.
ROOTS = ("root", "ROOT")
# A path that a script makes in a folder that is not the repository.
MADE = frozenset({"src/main.rs"})


def git(*args: str, stdin: str = "") -> list[str]:
  out = subprocess.run(  # noqa: S603
    [shutil.which("git") or "git", *args], cwd=ROOT, capture_output=True, check=False, input=stdin, text=True
  ).stdout
  return [x for x in out.split("\0") if x]


def tracked() -> set[str]:
  return {x for x in git("ls-files", "-z") if (ROOT / x).exists()}


def stands(path: str, files: set[str]) -> bool:
  return path in files or any(x.startswith(path + "/") for x in files)


def stale(
  files: set[str], texts: dict[str, str], made: Callable[[list[str]], set[str]] | None = None
) -> list[tuple[str, int, str]]:
  """Each path that a text names, as (file, line, path), whose first part is a folder of `files` and that no file or
  folder of `files` is, less those that `made` says a build makes."""
  tops = sorted({x.split("/")[0] for x in files if "/" in x}, key=len, reverse=True)
  before = "|".join([r"(?<![\w./@$}-])", *(rf"(?<=\${name}/)|(?<=\$\{{{name}\}}/)" for name in ROOTS)])
  named = re.compile(rf"(?:{before})((?:{'|'.join(map(re.escape, tops))})/[^\s\"'`()=,;:|<>&\\\[\]{{}}]*)")
  found = []
  for file, text in sorted(texts.items()):
    for number, line in enumerate(text.splitlines(), 1):
      for one in named.finditer(line):
        path = one[1].rstrip("./")
        if not (set(path) & set("*$?%") or path in MADE or stands(path, files)):
          found.append((file, number, path))
  gone = made(sorted({path for _, _, path in found})) if made else set()
  return [x for x in found if x[2] not in gone]


def ignored(paths: list[str]) -> set[str]:
  """The paths that git ignores. A folder that does not stand yet is ignored by a rule that ends in a slash only when
  it is asked as a folder, so each path is asked in both forms."""
  if not paths:
    return set()
  asked = [*paths, *(x + "/" for x in paths)]
  return {x.rstrip("/") for x in git("check-ignore", "-z", "--stdin", stdin="\0".join(asked) + "\0")}


def main() -> int:
  files = tracked()
  texts = {x: (ROOT / x).read_text(errors="replace") for x in sorted(files) if x.startswith(WATCHED)}
  found = stale(files, texts, ignored)
  for file, number, path in found:
    print(f"{file}:{number}: {path} is no file or folder of git")  # noqa: T201
  return 1 if found else 0


if __name__ == "__main__":
  sys.exit(main())
