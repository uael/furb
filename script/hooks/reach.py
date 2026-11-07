"""Run the tests that the files of a change reach, and leave the whole set to the gates.

The hook of pytest and the hook of cargo test call it with the files of the commit. A file of the engine, of its Kernel,
of the crate that the suite builds, of a manifest or of the harness reaches every test, so the whole suite runs. A test
file reaches itself, a file of an extension reaches the tests of that extension, and a script reaches the test of its
own beside the tests of the other scripts. A file of a crate reaches the tests of that crate, and a manifest of cargo
reaches the workspace. The gates run every hook on every file, so they reach the whole set.

Usage: python3 script/hooks/reach.py pytest|cargo <file>...
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# What the whole suite reads: the engine and its door, the crate and its build, the harness of the suite and the
# manifests. A change of one of them reaches every test.
WHOLE = re.compile(
  r"(src/|build/|build\.rs$|bind/python/|tests/|docs/examples/python/"
  r"|pyproject\.toml$|Cargo\.(toml|lock)$|rust-toolchain\.toml$|uv\.lock$|\.furb/config\.json$)"
)
# The harness of each suite, as files that git holds, so that script/paths.py checks each name.
HARNESS = {
  "test/conftest.py",
  "test/outside/conftest.py",
  "test/outside/doubles.py",
  ".agents/skills/conftest.py",
  ".furb/skills/conftest.py",
}
HOOK = {"script/hooks/pre-commit": "test/outside/test_hook_script.py"}
CONTRACT = re.compile(r"(test/(?!outside/)|\.agents/skills/[^/]+/test/|\.furb/skills/[^/]+/test/)")
TEST = re.compile(r"(test|\.agents/skills/[^/]+/test|\.furb/skills(/[^/]+/test)?)/(.+/)?test_[^/]*\.py$")
SKILL = re.compile(r"((?:\.furb|\.agents)/skills/[^/]+)/")
SCRIPT = re.compile(r"script/(?:hooks/)?([^/]+)\.(?:py|sh)$")


def naming(name: str, stem: str) -> list[str]:
  """Each test file that names a script, by its file or as a module that it imports."""
  marks = (name, f"import {stem}", f"from {stem} ", f"script.{stem}", f'"{stem}"')
  tests = [
    *ROOT.glob("test/**/test_*.py"),
    *ROOT.glob("script/hooks/test_*.py"),
    *ROOT.glob(".furb/skills/**/test_*.py"),
  ]
  return [
    one.relative_to(ROOT).as_posix() for one in tests if any(mark in one.read_text(encoding="utf-8") for mark in marks)
  ]


def suite(files: list[str]) -> list[str] | None:
  """The test paths that the files reach, or None for the whole suite."""
  found: list[str] = []
  for path in files:
    if WHOLE.match(path) or path in HARNESS or (path.startswith("test/") and not TEST.match(path)):
      return None
    if TEST.match(path):
      found.append(path)
      # A test of a contract is held by the hygiene laws, which check each sentence of the contract against its test.
      if CONTRACT.match(path):
        found.append("test/test_hygiene.py")
    elif path in HOOK:
      found.append(HOOK[path])
    elif hit := SKILL.match(path):
      tests = ROOT / hit[1] / "test"
      # The words of the project have tests of their own beside them, and the contract of an official one has its laws.
      found.append("test/test_hygiene.py")
      if hit[1].startswith(".furb/"):
        found.append(".furb/skills")
      if tests.is_dir():
        found.append(tests.relative_to(ROOT).as_posix())
    elif hit := SCRIPT.match(path):
      found += naming(Path(path).name, hit[1])
  return sorted({one for one in found if (ROOT / one).exists()})


# The crate of each folder, and the root crate for each other file of the crate.
CRATES = {"host/": "furb-host", "cli/": "furb-cli"}


def crates(files: list[str]) -> list[str] | None:
  """The packages of cargo that the files reach, or None for the workspace."""
  found: set[str] = set()
  for path in files:
    if re.match(r"(Cargo\.(toml|lock)|rust-toolchain\.toml)$", path):
      return None
    found.add(next((name for folder, name in CRATES.items() if path.startswith(folder)), "furb"))
  return sorted(found)


def command(kind: str, files: list[str]) -> list[str] | None:
  """The command that runs the tests that the files reach, or None when they reach none."""
  if kind == "pytest":
    reached = suite(files)
    if reached == []:
      return None
    # The coverage of the whole suite is the wall of the gates: a part of the suite cannot reach it.
    return ["uv", "run", "pytest", "-q", *(["--no-cov", *reached] if reached else [])]
  packages = crates(files)
  chosen = ["--workspace"] if packages is None else [x for one in packages for x in ("-p", one)]
  return ["cargo", "test", *chosen, "--features", "furb/typescript"]


def main(args: list[str]) -> int:
  run = command(args[0], args[1:])
  if run is None:
    return 0
  sys.stdout.write(" ".join(run) + "\n")
  sys.stdout.flush()
  os.execvp(run[0], run)  # noqa: S606
  return 1


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))
