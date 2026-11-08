"""Runs each proof that the page of the promise names, and fails when one is gone or does not pass.

Usage: uv run python script/promise.py [page]

The page, docs/promise.md, lists under "The proofs that a machine runs" each test that proves a part of the promise, as
`file::name`. A release publishes nothing while one of them fails: .github/workflows/publish.yml runs this script
before any job that publishes. The tests of python run in one pytest, and the tests of rust in one cargo test for each
target that holds them. A proof that its run does not report as passed fails the script, which names it.
"""
# ruff: noqa: T201

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PAGE = ROOT / "docs" / "promise.md"
# Each proof of the page, in the part that starts at its heading and ends where the page lists what no machine proves.
PROOF = re.compile(r"`([\w./\\:-]+\.(?:py|rs))::(\w+)`")


def proofs(text: str) -> list[tuple[str, str]]:
  """The file and the name of each proof that the page names, once each."""
  start = text.index("## The proofs that a machine runs")
  end = text.find("What no machine proves yet", start)
  return list(dict.fromkeys(PROOF.findall(text[start : end if end > 0 else None])))


def target(file: str) -> list[str]:
  """The arguments of cargo test that build the target which holds a test of rust in this file."""
  package = "furb-host" if file.startswith("host/") else "furb-cli" if file.startswith("cli/") else "furb"
  if hit := re.fullmatch(r"(?:host/|cli/)?tests/(\w+)\.rs", file):
    return ["-p", package, "--test", hit[1]]
  return ["-p", package, "--lib"]


def runs(found: list[tuple[str, str]]) -> list[tuple[list[str], list[str], list[tuple[str, str]]]]:
  """Each command to run, the pattern of the line of each proof that it passes, and those proofs."""
  python = [one for one in found if one[0].endswith(".py")]
  made = []
  if python:
    ids = [f"{file}::{name}" for file, name in python]
    command = [sys.executable, "-m", "pytest", "-q", "--no-cov", "-p", "no:cacheprovider", "-n", "0", "-rA", *ids]
    # pytest names a file by its path from its root, so the line of a proof is matched by the name of its file; on Windows,
    # a file on another drive than the root has no path from it, and pytest names it by the test alone.
    lines = [rf"^PASSED (\S*[/\\])?(?:{re.escape(re.split(r'[/\\]', file)[-1])})?::{name}\b" for file, name in python]
    made.append((command, lines, python))
  rust: dict[tuple[str, ...], list[tuple[str, str]]] = {}
  for one in found:
    if one[0].endswith(".rs"):
      rust.setdefault(tuple(target(one[0])), []).append(one)
  for args, held in rust.items():
    names = [name for _, name in held]
    made.append((["cargo", "test", *args, "--", *names], [rf"^test (\S+::)?{name} \.\.\. ok$" for name in names], held))
  return made


def main(argv: list[str]) -> int:
  page = Path(argv[0]) if argv else PAGE
  found = proofs(page.read_text(encoding="utf-8"))
  if not found:
    print(f"promise: {page} names no proof")
    return 1
  failed = []
  for command, patterns, held in runs(found):
    print(f"promise: {' '.join(command)[:200]}", flush=True)
    run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=False)  # noqa: S603
    out = run.stdout + run.stderr
    missed = [held[at] for at, one in enumerate(patterns) if not re.search(one, out, re.MULTILINE)]
    if missed:
      print(out[-4000:])
    failed += missed
  for file, name in failed:
    print(f"promise: {file}::{name} did not pass")
  print(f"promise: {len(found) - len(failed)} of {len(found)} proofs passed")
  return 1 if failed else 0


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))
