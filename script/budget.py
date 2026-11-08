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

from __future__ import annotations

import ctypes
import os
import re
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from ctypes import wintypes
from pathlib import Path

try:
  import resource
except ImportError:  # Windows has no resource module: a job object counts the CPU time of the command there.
  resource = None

HOME = Path(os.environ.get("FURB_BUDGETS") or Path(__file__).with_name("budgets.toml"))
# A line of a test with its time: pytest gives `12.34s call path::name`, cargo test `test name ... ok <1.234s>`.
TIME = re.compile(
  r"^\s*(?P<lead>\d+\.\d+)s\s+\w+\s+(?P<a>\S+)|^(?:\s*\d+\.\d+ )?test (?P<b>\S+) \.\.\. \w+ <(?P<trail>\d+\.\d+)s>"
)


# The log of the guard of memory, script/memwatch.sh: a line for each process that it ended, as its time in seconds of
# the epoch, its pid and its command. It is the one evidence that the guard ended a process, and not the kernel, a
# timeout or a crash, which end it by the same signal.
WATCH_LOG = Path(os.environ.get("FURB_WATCH_LOG") or Path(tempfile.gettempdir()) / "furb-memwatch.log")


def watched(since: float, until: float, root: Path, said: str) -> list[str]:
  """The processes of this run that the log of the guard of memory says it ended between two times of the epoch: each
  whose program lies under the folder of the run, or that the output of the run names, as cargo names a test binary that
  a signal ended."""
  try:
    text = WATCH_LOG.read_text(encoding="utf-8")
  except OSError:
    return []
  found = []
  for line in text.splitlines():
    when, pid, command = [*line.split(" ", 2), "", ""][:3]
    program = command.split(" ", 1)[0]
    ours = program.startswith(f"{root}{os.sep}") or (bool(program) and program in said)
    if when.isdigit() and since - 1 <= int(when) <= until + 1 and ours:
      found.append(f"pid {pid}: {command}")
  return found


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


def windows_job() -> tuple[Callable[[subprocess.Popen[str]], None], Callable[[], float]] | None:
  """The two calls of a job object of Windows: join a command to it, and read the CPU seconds that it and its children used.

  Windows counts no CPU time of waited children, as POSIX does, but a job object holds a process with each child that
  it starts, and its accounting adds their user and kernel time. None where there is no such object.
  """
  if sys.platform != "win32":
    return None

  class Accounting(ctypes.Structure):
    _fields_ = (
      ("user", ctypes.c_int64),
      ("kernel", ctypes.c_int64),
      ("period_user", ctypes.c_int64),
      ("period_kernel", ctypes.c_int64),
      ("faults", wintypes.DWORD),
      ("started", wintypes.DWORD),
      ("active", wintypes.DWORD),
      ("ended", wintypes.DWORD),
    )

  kernel = ctypes.WinDLL("kernel32", use_last_error=True)
  kernel.CreateJobObjectW.restype = wintypes.HANDLE
  kernel.CreateJobObjectW.argtypes = (ctypes.c_void_p, wintypes.LPCWSTR)
  kernel.AssignProcessToJobObject.argtypes = (wintypes.HANDLE, wintypes.HANDLE)
  kernel.QueryInformationJobObject.argtypes = (
    wintypes.HANDLE,
    ctypes.c_int,
    ctypes.c_void_p,
    wintypes.DWORD,
    ctypes.c_void_p,
  )
  job = kernel.CreateJobObjectW(None, None)
  if not job:
    return None

  def join(run: subprocess.Popen[str]) -> None:
    kernel.AssignProcessToJobObject(job, int(run._handle))  # type: ignore[attr-defined]

  def seconds() -> float:
    info = Accounting()
    # The class 1 of a job object is its basic accounting, whose times count in units of 100 ns.
    kernel.QueryInformationJobObject(job, 1, ctypes.byref(info), ctypes.sizeof(info), None)
    return (info.user + info.kernel) / 1e7

  return join, seconds


def cpu(job: tuple[Callable[[subprocess.Popen[str]], None], Callable[[], float]] | None) -> float:
  """The CPU seconds that the children of this process, waited for, have used, or those of the job of Windows."""
  if resource is not None:
    use = resource.getrusage(resource.RUSAGE_CHILDREN)
    return use.ru_utime + use.ru_stime
  return job[1]() if job else 0.0


def load() -> float:
  """The load average of the last minute, or the one that FURB_BUDGET_LOAD gives."""
  if given := os.environ.get("FURB_BUDGET_LOAD"):
    return float(given)
  return os.getloadavg()[0] if hasattr(os, "getloadavg") else 0.0


def main(argv: list[str]) -> int:
  part, command = argv[0], argv[2:]
  table = limits(HOME.read_text(encoding="utf-8"), "ci" if os.environ.get("CI") else "local")
  wall_limit = table.get(part, 0)
  job = windows_job() if resource is None else None
  counts = resource is not None or job is not None
  cpu_limit = table.get(part + "_cpu", 0) if counts else 0
  cores, busy = os.cpu_count() or 1, load()
  start, began, used = time.monotonic(), time.time(), cpu(job)
  lines: list[str] = []
  with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True) as run:  # noqa: S603
    assert run.stdout is not None
    if job:
      job[0](run)
    for line in run.stdout:
      sys.stdout.write(line)
      lines.append(line)
  took, worked = time.monotonic() - start, cpu(job) - used
  said = f"budget: {part} took {took:.0f} s" + (f" of {wall_limit} s" if wall_limit else "")
  if counts:
    said += f" and {worked:.0f} s of CPU" + (f" of {cpu_limit} s" if cpu_limit else "")
  print(said + ("" if wall_limit or cpu_limit else ", with no limit yet"), flush=True)
  if run.returncode:
    if ended := watched(began, time.time(), Path.cwd(), "".join(lines)):
      print(f"budget: the guard of memory, script/memwatch.sh, ended a test process of {part}, by its log {WATCH_LOG}:")
      for one in ended:
        print(f"  {one}")
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
