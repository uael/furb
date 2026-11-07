"""Give a copy of a record of an older build the texts of its engine, so that a later build replays it on them.

A fact made names the digest of the engine and of the Kernel that made a segment of its record. For each one that holds
no texts, this finds in git the commit whose src/furb/engine.py and src/furb/kernel.py have that digest, and writes
those texts, as git holds them, into the fact. The digest stays the one that the build wrote, and each text is the one
that the build ran. A defect of replay that a later build fixed in an engine of the major is mended at replay, by the
adapter of that engine in src/furb/adapters.py, and never in the texts. It rewrites the file, so give it a copy of a
record, and never the record of a life.

A build before the texts replayed an older record on its own engine and went on with no fact made of its own, so a
segment of several lives may hold the entries of several engines. The script says how many lives each segment holds,
and tests/tip.rs opens the copy to check it: a segment that the texts of one engine, and its adapter, cannot replay
drifts.

Usage: python3 script/engines.py <copy of a record>
"""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

WORDS = 7
"""The words of a fact made that holds no texts: its kind, its chain, its speaker, the digest, the version, the schema
and the release, each null that an older build did not write."""


def git(*args: str) -> str:
  """The output of a command of git."""
  return subprocess.run(["git", *args], capture_output=True, text=True, check=True).stdout  # noqa: S603, S607


def digest(engine: str, kernel: str) -> str:
  """The digest of an engine and of its Kernel, as a fact made names it."""
  return hashlib.sha256(engine.encode() + b"\0" + kernel.encode() + b"\0").hexdigest()


def found(named: str) -> tuple[str, str]:
  """The texts of the engine and of the Kernel of the commit whose pair has a digest."""
  for sha in git("rev-list", "--all", "--", "src/furb/engine.py", "src/furb/kernel.py").split():
    try:
      engine, kernel = git("show", f"{sha}:src/furb/engine.py"), git("show", f"{sha}:src/furb/kernel.py")
    except subprocess.CalledProcessError:
      continue
    if digest(engine, kernel) == named:
      return engine, kernel
  sys.exit(f"no commit holds the engine {named}")


def main() -> None:
  """Writes the texts of its engine into each fact made of the copy that holds none."""
  path = Path(sys.argv[1])
  lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
  lives = [at for at, line in enumerate(lines) if line.startswith('[["stand","stand') and '"operator","chain1"' in line]
  for at, line in enumerate(lines):
    entry = json.loads(line) if '"made"' in line else None
    fact = entry and entry[0]
    if not (isinstance(fact, list) and fact[:1] == ["made"] and len(fact) <= WORDS):
      continue
    fact += [None] * (WORDS - len(fact))
    engine, kernel = found(fact[3])
    fact += [engine, kernel]
    lines[at] = json.dumps(entry, ensure_ascii=False, separators=(",", ":")) + "\n"
    after = [n for n in lives if n > at]
    end = next((n for n, x in enumerate(lines) if n > at and x.startswith('[["checkpoint"')), len(lines))
    sys.stdout.write(f"line {at + 1}: the engine {fact[3]}, for {len([n for n in after if n < end])} lives\n")
  path.write_text("".join(lines), encoding="utf-8")


if __name__ == "__main__":
  main()
