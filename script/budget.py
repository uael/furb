"""Runs the command of a part of the suite and fails when the run passes the limit of that part.

Usage: python3 script/budget.py <part> -- <command> [argument...]

The limits stand in script/budgets.toml, one for each part, and the hooks and the gates read them there: the table
[local] holds those of this machine and [ci] those of a runner of the gates. A part with no limit only reports its time.
A run past its limit exits with 1 and names its slowest tests, as its runner prints them with their times (pytest with
--durations, cargo test with --report-time). A command that fails gives its own exit code.
"""
# ruff: noqa: T201

import os
import re
import subprocess
import sys
import time
from pathlib import Path

HOME = Path(os.environ.get("FURB_BUDGETS") or Path(__file__).with_name("budgets.toml"))
# A line of a test with its time: pytest gives `12.34s call path::name`, cargo test `test name ... ok <1.234s>`.
TIME = re.compile(
  r"^\s*(?P<lead>\d+\.\d+)s\s+\w+\s+(?P<a>\S+)|^(?:\s*\d+\.\d+ )?test (?P<b>\S+) \.\.\. \w+ <(?P<trail>\d+\.\d+)s>"
)


def slowest(lines: list[str], count: int = 10) -> list[tuple[float, str]]:
  found = [(float(hit["lead"] or hit["trail"]), hit["a"] or hit["b"]) for line in lines if (hit := TIME.match(line))]
  return sorted(found, reverse=True)[:count]


def limits(text: str, table: str) -> dict[str, int]:
  """The whole numbers of a table of the home, read with no library, since the hook runs on the python3 of the machine."""
  found: dict[str, int] = {}
  here = ""
  for line in text.splitlines():
    if hit := re.fullmatch(r"\[(\w+)\]", line.strip()):
      here = hit[1]
    elif here == table and (hit := re.fullmatch(r"(\w+)\s*=\s*(\d+)\s*(?:#.*)?", line.strip())):
      found[hit[1]] = int(hit[2])
  return found


def main(argv: list[str]) -> int:
  part, command = argv[0], argv[2:]
  limit = limits(HOME.read_text(encoding="utf-8"), "ci" if os.environ.get("CI") else "local").get(part)
  start = time.monotonic()
  lines: list[str] = []
  with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True) as run:  # noqa: S603
    assert run.stdout is not None
    for line in run.stdout:
      sys.stdout.write(line)
      lines.append(line)
  took = time.monotonic() - start
  print(f"budget: {part} took {took:.0f} s" + (f" of {limit} s" if limit else ", with no limit yet"), flush=True)
  if run.returncode or not limit or took <= limit:
    return run.returncode
  print(f"budget: {part} passed its limit by {took - limit:.0f} s. Make it faster, and never raise the limit.")
  print("budget: its slowest tests:")
  for seconds, name in slowest(lines) or [(0.0, "none named: run the part with the times of its tests")]:
    print(f"  {seconds:>7.2f} s  {name}")
  return 1


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))
