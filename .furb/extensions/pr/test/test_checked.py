"""checked waits for the checks of a PR within one bound, and gives a failure with its kind and its first lines."""

import asyncio
import json
import os
import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)
LINK = "https://github.com/uael/furb/actions/runs/77/job/{}"


@dataclass
class Text:
  path: str
  content: str = ""


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Refused(Exception):
  pass


class World:
  """The engine as the word reads it: bash answers from a script, wait moves a clock, and every command is kept."""

  def __init__(self, polls: list[list[dict]], logs: dict[str, str] | None = None) -> None:
    self.polls, self.logs, self.now = polls, logs or {}, 0.0
    self.commands: list[str] = []
    self.waits: list[float] = []
    ns: dict = {
      "bash": self.bash,
      "wait": self.wait,
      "clock": lambda: self.now,
      "Exit": Exit,
      "Refused": Refused,
      "span": lambda lo, hi: (lo, hi),
      "HIDDEN": "hidden",
    }
    exec(compile(IMPORT.sub("", (HERE / "pr.py").read_text(encoding="utf-8")), "pr.py", "exec"), ns)  # noqa: S102
    self.checked = ns["checked"]

  async def bash(self, command: str, **_: object) -> Exit:
    self.commands.append(command)
    if "actions/jobs" in command:
      job = re.search(r"id=(\d+)", command)
      assert job
      return Exit(0, Text("out", self.shell(command, self.logs[job[1]])), Text("err"))
    rows = self.polls.pop(0) if len(self.polls) > 1 else self.polls[0]
    return Exit(0, Text("out", "abc1234def\n" + (json.dumps(rows) if rows is not None else "")), Text("err"))

  def shell(self, command: str, log: str) -> str:
    """Run the command in sh, with a gh that prints the log: the sorting of a log is the script's."""
    with tempfile.TemporaryDirectory() as folder:
      Path(folder, "log").write_text(log, encoding="utf-8")
      Path(folder, "gh").write_text('#!/bin/sh\ncat "$LOG"\n', encoding="utf-8")
      Path(folder, "gh").chmod(0o755)
      env = {**os.environ, "PATH": folder + os.pathsep + os.environ["PATH"], "LOG": str(Path(folder, "log"))}
      return subprocess.run(["sh", "-c", command], env=env, capture_output=True, text=True, check=True).stdout

  async def wait(self, seconds: float) -> None:
    self.waits.append(seconds)
    self.now += seconds


def row(name: str, bucket: str, job: int = 1) -> dict:
  return {"name": name, "bucket": bucket, "link": LINK.format(job)}


def run(world: World, **words: object) -> tuple[str, list[str]]:
  return asyncio.run(world.checked(5, **words))


def test_a_poll_with_no_check_waits_a_minute_and_reads_again_until_all_pass() -> None:
  world = World([[], [row("hooks", "pending")], [row("hooks", "pass"), row("scope", "skipping")]])
  assert run(world) == ("pass", ["head abc1234: 2 checks, 0 failed"])
  assert world.waits == [60.0, 60.0]


def test_a_check_that_pends_past_the_bound_gives_pending_with_its_name() -> None:
  world = World([[row("hooks", "pending"), row("scope", "pass")]])
  assert run(world, bound=120.0) == ("pending", ["head abc1234: still pending after 120 s: hooks"])
  assert world.waits == [60.0, 60.0]


def test_a_bound_that_ends_with_no_check_started_says_so() -> None:
  assert run(World([[]]), bound=30.0) == ("pending", ["head abc1234: still pending after 0 s: no check has started"])


def test_a_failed_check_waits_for_the_checks_that_pend_before_it_speaks() -> None:
  world = World(
    [[row("a", "fail", 1), row("b", "pending")], [row("a", "fail", 1), row("b", "pass")]],
    {"1": "FAILED test_x.py::test_one - boom\nProcess completed with exit code 1"},
  )
  state, lines = run(world)
  assert state == "fail" and world.waits == [60.0]
  assert lines == [
    "head abc1234: 2 checks, 1 failed",
    "a [defect] run 77 job 1",
    "  FAILED test_x.py::test_one - boom",
  ]


def test_a_log_with_a_fault_of_the_runner_is_a_machine_fault() -> None:
  world = World([[row("a", "fail", 2)]], {"2": "build\nThe runner has received a shutdown signal.\n##[error]Process completed"})
  assert run(world)[1][1] == "a [machine] run 77 job 2"


def test_a_log_with_only_the_generic_error_gives_its_last_lines() -> None:
  log = "\n".join(f"line {n}" for n in range(60)) + "\n##[error]Process completed with exit code 1."
  lines = run(World([[row("a", "fail", 3)]], {"3": log}))[1]
  assert lines[1] == "a [defect] run 77 job 3" and lines[2] == "  line 16" and len(lines) == 2 + 45


def test_a_cancelled_check_is_named_and_a_log_that_does_not_load_is_unread() -> None:
  world = World([[row("a", "cancel", 4), {"name": "b", "bucket": "fail", "link": "https://x.test/y"}]])
  assert run(world)[1][1:] == ["a [cancelled] https://github.com/uael/furb/actions/runs/77/job/4", "b [unread] https://x.test/y"]


def test_a_directory_is_entered_before_each_command() -> None:
  world = World([[row("a", "pass")]])
  run(world, where="/work tree")
  assert world.commands[0].startswith("cd '/work tree' || exit 1; n=5\n")
