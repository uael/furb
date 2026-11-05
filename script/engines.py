"""Give a copy of a record of an older build the texts of its engine, so that a later build replays it on them.

A fact made names the digest of the engine and of the Kernel that made a segment of its record. For each one that holds
no texts, this finds in git the commit whose src/furb/engine.py and src/furb/kernel.py have that digest, applies to that
engine the change of FIX where it applies, and writes the texts and their digest into the fact. It rewrites the file, so
give it a copy of a record, and never the record of a life.

Usage: python3 script/engines.py <copy of a record>
"""

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

WORDS = 7
"""The words of a fact made that holds no texts: its kind, its chain, its speaker, the digest, the version, the schema and
the release."""

FIX = "656c26163"
"""The commit by which a replay goes on past the raise of an ear (#1281), which each older engine takes where it applies."""


def git(*args: str, at: str | None = None, given: str | None = None) -> str:
  """The output of a command of git."""
  return subprocess.run(["git", *args], cwd=at, input=given, capture_output=True, text=True, check=True).stdout  # noqa: S603, S607


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


def fixed(engine: str) -> str:
  """The engine with the change of FIX, or as it stands when that change does not apply to it."""
  with tempfile.TemporaryDirectory() as at:
    path = Path(at, "src/furb/engine.py")
    path.parent.mkdir(parents=True)
    path.write_text(engine, encoding="utf-8")
    try:
      git("apply", "--include=src/furb/engine.py", at=at, given=git("show", FIX))
    except subprocess.CalledProcessError:
      return engine
    return path.read_text(encoding="utf-8")


def main() -> None:
  """Writes the texts of its engine into each fact made of the copy that holds none."""
  path = Path(sys.argv[1])
  lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
  for at, line in enumerate(lines):
    entry = json.loads(line) if '"made"' in line else None
    fact = entry and entry[0]
    if not (isinstance(fact, list) and fact[:1] == ["made"] and len(fact) == WORDS):
      continue
    engine, kernel = found(fact[3])
    engine = fixed(engine)
    fact[3] = digest(engine, kernel)
    fact += [engine, kernel]
    lines[at] = json.dumps(entry, ensure_ascii=False, separators=(",", ":")) + "\n"
    sys.stdout.write(f"line {at + 1}: the engine {fact[3]}\n")
  path.write_text("".join(lines), encoding="utf-8")


if __name__ == "__main__":
  main()
