"""Remove what earlier runs left in the caches of the machine, and nothing else.

Usage: python3 script/clean-caches.py [--scratch DIR] [--uv DIR]

A scratch directory of a record that holds nothing but its mark file, and that no life holds, goes: a life makes it
again from its record. A life holds its directory by a shared lock on the mark, so a directory that a life holds, a
directory that holds a file, the folder of the scratch directories and its stamp stay. The cache of uv is pruned, which
removes each unpacked wheel and each editable build that no entry points to. It reads no config: the project
sets no-cache for uv, which makes a prune in the tree prune a cache that holds nothing. The folders default to those of this
machine.
"""

import argparse
import fcntl
import os
import re
import subprocess
import sys
from pathlib import Path

MARK = ".furb-scratch"
RECORD = re.compile(r"[0-9a-f]{16}(\.new)?")


def machine() -> Path:
  """The folder of the scratch directories of the records, where the crate puts it."""
  home = Path.home() / ("Library/Caches" if sys.platform == "darwin" else ".cache")
  return Path(os.environ.get("XDG_CACHE_HOME") or home) / "furb" / "scratch"


def swept(folder: Path) -> bool:
  """Whether the directory of a record held nothing but its mark and no life held it, and went."""
  mark = folder / MARK
  try:
    if [one.name for one in folder.iterdir()] != [MARK]:
      return False
    with mark.open("rb") as held:
      fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
      mark.unlink()
      try:
        folder.rmdir()
      except OSError:
        mark.touch()
        return False
  except OSError:
    return False
  return True


def main(args: list[str]) -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--scratch", type=Path, default=machine())
  parser.add_argument("--uv", type=Path)
  given = parser.parse_args(args)
  folders = [one for one in given.scratch.iterdir() if one.is_dir() and RECORD.fullmatch(one.name)]
  sys.stdout.write(f"{sum(map(swept, folders))} scratch directories removed, of {len(folders)}\n")
  env = {**os.environ, **({"UV_CACHE_DIR": str(given.uv)} if given.uv else {})}
  return subprocess.run(["uv", "--no-config", "cache", "prune"], env=env, check=False).returncode  # noqa: S607


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))
