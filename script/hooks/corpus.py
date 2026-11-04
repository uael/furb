"""Refuses a change of each file of a folder of tests/corpus that a tag holds.

Each folder of tests/corpus but next is named by the version of a release, and the tag v<version> holds it. Once that
tag stands, no file of the folder changes again, so each later build proves that it opens the records of that release.
Before 1.0.0 no promise covers a release, so a change whose title has ! may delete the files of a folder of 0.x.

With no argument it reads the staged changes, as a hook does, which knows no title and so lets a folder of 0.x go. With
--base <ref> it reads the changes from the merge base of <ref> to HEAD, as CI does, and --title gives the title of the
pull request.

Usage: python3 script/hooks/corpus.py [--base <ref> --title <title>]
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from collections.abc import Iterator

CORPUS = "tests/corpus/"


def git(*args: str) -> str:
  """The output of a command of git."""
  return subprocess.run([shutil.which("git") or "git", *args], capture_output=True, text=True, check=True).stdout  # noqa: S603


def changes(base: str) -> Iterator[tuple[str, str]]:
  """The status and the path of each change of a file of the corpus."""
  span = ["--cached"] if not base else [f"{base}...HEAD"]
  for line in git("diff", "--name-status", "--no-renames", *span).splitlines():
    status, _, path = line.partition("\t")
    if path.startswith(CORPUS):
      yield status[:1], path


def breaking(title: str) -> bool:
  """Whether a title has !, as a change that breaks."""
  return re.match(r"\w+(\([^)]*\))?!:", title) is not None


def refused(base: str = "", title: str | None = None) -> list[str]:
  """Each change that a tag forbids, with why."""
  found = []
  for status, path in changes(base):
    folder, _, rest = path.removeprefix(CORPUS).partition("/")
    if not rest or folder == "next" or not git("tag", "--list", f"v{folder}").strip():
      continue
    if status == "D" and folder.startswith("0.") and (title is None or breaking(title)):
      continue
    found.append(f"{path}: the tag v{folder} holds {CORPUS}{folder}, so no file of it changes again")
  return found


def main() -> int:
  """Prints each refused change and fails when there is one."""
  parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  parser.add_argument("--base", default="")
  parser.add_argument("--title")
  given = parser.parse_args()
  found = refused(given.base, given.title)
  if found:
    tail = "The corpus only grows: add a record to tests/corpus/next, as tests/corpus/README.md says."
    sys.stderr.write("\n".join([*found, tail]) + "\n")
  return 1 if found else 0


if __name__ == "__main__":
  sys.exit(main())
