"""Runs the command of a part of the suite and fails when the run passes the limit of that part.

Usage: python3 script/budget.py <part> -- <command> [argument...]

The limits stand in script/budgets.toml, and the hooks and the gates read them there: the table [local] holds those of
this machine and [ci] those of a runner of the gates. A part has a limit of wall time, <part>, in seconds, and may have
a limit of CPU time, <part>_cpu, in seconds, that the command and its children use together. A part with no limit only
reports its time.

The wall time stretches with the load of the machine, and the CPU time that a run needs does not: a hook that ran beside
the builds of other chains kept its work and failed its clock (the figures stand in budgets.toml). So a run that
passes its wall limit while the load average at its start is over the cores is judged by its CPU time, when the part
has a limit of it.
On a quiet machine, and for a part with no limit of CPU, the wall time judges: a test that waits on a clock shows only
there. A run past its CPU limit fails whatever the load. A run that fails names its slowest tests, as its runner prints
them with their times (pytest with --durations, cargo test with --report-time). A command that fails gives its own exit
code. FURB_BUDGET_LOAD gives the load, for a test.
"""
# ruff: noqa: T201

import os
import re
import subprocess
import sys
import time
from pathlib import Path

try:
  import resource
except ImportError:  # Windows counts no CPU time of children, so a part there has a limit of wall time alone.
  resource = None

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


def cpu() -> float:
  """The CPU seconds that the children of this process, waited for, have used."""
  if resource is None:
    return 0.0
  use = resource.getrusage(resource.RUSAGE_CHILDREN)
  return use.ru_utime + use.ru_stime


def load() -> float:
  """The load average of the last minute, or the one that FURB_BUDGET_LOAD gives."""
  if given := os.environ.get("FURB_BUDGET_LOAD"):
    return float(given)
  return os.getloadavg()[0] if hasattr(os, "getloadavg") else 0.0


def main(argv: list[str]) -> int:
  part, command = argv[0], argv[2:]
  table = limits(HOME.read_text(encoding="utf-8"), "ci" if os.environ.get("CI") else "local")
  wall_limit = table.get(part, 0)
  cpu_limit = table.get(part + "_cpu", 0) if resource is not None else 0
  cores, busy = os.cpu_count() or 1, load()
  start, used = time.monotonic(), cpu()
  lines: list[str] = []
  with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True) as run:  # noqa: S603
    assert run.stdout is not None
    for line in run.stdout:
      sys.stdout.write(line)
      lines.append(line)
  took, worked = time.monotonic() - start, cpu() - used
  said = f"budget: {part} took {took:.0f} s" + (f" of {wall_limit} s" if wall_limit else "")
  if resource is not None:
    said += f" and {worked:.0f} s of CPU" + (f" of {cpu_limit} s" if cpu_limit else "")
  print(said + ("" if wall_limit or cpu_limit else ", with no limit yet"), flush=True)
  if run.returncode:
    return run.returncode
  over_cpu = cpu_limit > 0 and worked > cpu_limit
  over_wall = wall_limit > 0 and took > wall_limit
  if over_wall and cpu_limit and not over_cpu and busy > cores:
    print(
      f"budget: {part} passed its wall limit while the load was {busy:.0f} on {cores} cores, and its CPU time fits."
    )
    over_wall = False
  if not (over_cpu or over_wall):
    return 0
  if over_cpu:
    print(f"budget: {part} used {worked:.0f} s of CPU, {worked - cpu_limit:.0f} s over its limit.")
  if over_wall:
    print(f"budget: {part} passed its wall limit by {took - wall_limit:.0f} s.")
  print("budget: make the run faster, and never raise a limit. Its slowest tests:")
  for seconds, name in slowest(lines) or [(0.0, "none named: run the part with the times of its tests")]:
    print(f"  {seconds:>7.2f} s  {name}")
  return 1


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))
