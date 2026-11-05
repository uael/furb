"""Fill src/furb/engines with each engine that main shipped since a release, one folder by digest.

A record names the digest of the engine and of the Kernel that made it, as `engine()` of `src/world/store.rs` gives
it. A build replays a record on that engine, so it keeps each engine of its major as it shipped: the folder
`src/furb/engines/<digest>/` holds the `engine.py` and the `kernel.py` of a commit of main that give that digest. The
sheet of the gate is the build's own, as the rest of the outside is. A record replays on an engine of the archive only
at the version of the replay of the build, `VERSION` in `src/world/store.rs`, so the archive keeps the engines of the
commits at that version alone, and drops the others.

Usage: python3 script/engines.py [--since v0.4.2] [--ref origin/main] [--check]

With --check it writes nothing, and fails when a folder holds files whose digest is not its name.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ARCHIVE = ROOT / "src" / "furb" / "engines"
FILES = ("engine.py", "kernel.py")
GIT = shutil.which("git") or "git"


def shown(rev: str, path: str) -> bytes | None:
  got = subprocess.run([GIT, "-C", str(ROOT), "show", f"{rev}:{path}"], capture_output=True, check=False)  # noqa: S603
  return got.stdout if got.returncode == 0 else None


def git(*words: str) -> str:
  return subprocess.run([GIT, "-C", str(ROOT), *words], capture_output=True, text=True, check=True).stdout  # noqa: S603


def digest(engine: bytes, kernel: bytes) -> str:
  held = hashlib.sha256()
  for one in (engine, kernel):
    held.update(one)
    held.update(b"\0")
  return held.hexdigest()


def version(rev: str) -> int | None:
  store = shown(rev, "src/world/store.rs") or b""
  found = re.search(rb"pub const VERSION: i64 = (\d+);", store)
  return int(found[1]) if found else None


def say(line: str) -> None:
  sys.stdout.write(line + "\n")


def checked() -> int:
  wrong = 0
  for folder in sorted(x for x in ARCHIVE.iterdir() if x.is_dir()):
    missing = [name for name in FILES if not (folder / name).is_file()]
    if missing:
      say(f"{folder.name} lacks {', '.join(missing)}")
      wrong += 1
      continue
    made = digest((folder / "engine.py").read_bytes(), (folder / "kernel.py").read_bytes())
    if made != folder.name:
      say(f"{folder.name} holds the engine {made}")
      wrong += 1
  return wrong


def filled(since: str, ref: str) -> None:
  touched = git("log", "--format=%H", f"{since}..{ref}", "--", "src/furb/engine.py", "src/furb/kernel.py").split()
  now, kept = version("HEAD"), set()
  ARCHIVE.mkdir(parents=True, exist_ok=True)
  for rev in [since, *reversed(touched)]:
    engine, kernel = shown(rev, "src/furb/engine.py"), shown(rev, "src/furb/kernel.py")
    if version(rev) != now or engine is None or kernel is None:
      continue
    folder = ARCHIVE / digest(engine, kernel)
    kept.add(folder.name)
    if folder.exists():
      continue
    folder.mkdir()
    (folder / "engine.py").write_bytes(engine)
    (folder / "kernel.py").write_bytes(kernel)
    say(f"{folder.name[:16]} from {git('log', '-1', '--format=%h %s', rev).strip()}")
  for folder in sorted(x for x in ARCHIVE.iterdir() if x.is_dir() and x.name not in kept):
    shutil.rmtree(folder)
    say(f"{folder.name[:16]} dropped, of another version")


def main() -> int:
  parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
  parser.add_argument("--since", default="v0.4.2")
  parser.add_argument("--ref", default="origin/main")
  parser.add_argument("--check", action="store_true")
  given = parser.parse_args()
  if not given.check:
    filled(given.since, given.ref)
  return 1 if checked() else 0


if __name__ == "__main__":
  sys.exit(main())
