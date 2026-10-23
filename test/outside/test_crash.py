"""The report of the bench of a crash, on a copy of a run that the bench kept."""

import importlib.util
import shutil
from pathlib import Path
from types import ModuleType

from furb.world import kept

RUN = Path(__file__).parent / "crash" / "ink-grid-box-layout"
SCRIPT = Path(__file__).resolve().parents[2] / "script" / "bench" / "crash.py"


def bench() -> ModuleType:
  """The module of the bench, loaded from its file."""
  spec = importlib.util.spec_from_file_location("crash", SCRIPT)
  assert spec and spec.loader
  module = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(module)
  return module


def test_a_reply_that_says_interrupted_is_no_cut(tmp_path: Path) -> None:
  """A reply whose text says "was interrupted" is no cut of a command, and the command that a wake closed is."""
  run = shutil.copytree(RUN, tmp_path / RUN.name)
  record = [one for (one, *_) in kept(run / "record.jsonl")]
  assert [x[1] for x in record if x[0] == "done" and x[1] == "reply4" and "was interrupted" in repr(x[3])] == ["reply4"]
  got = bench().measured(run)
  assert got["commands"] == 26
  assert got["interrupted"] == ["npx ava"]
  assert got["ran_again_after_a_kill"] == []
  assert got["replies_asked_again"] == ["reply2"]
  assert got["kills"] == 2
  assert got["extra_calls"] == 4.0


def test_a_command_that_ran_again_counts(tmp_path: Path) -> None:
  """A later command with the same line as a cut counts as a command that ran again after the kill."""
  run = shutil.copytree(RUN, tmp_path / RUN.name)
  with (run / "record.jsonl").open("a", encoding="utf-8") as out:
    out.write('[["bash","bash27","rung9","chain1","npx ava","",{"is":"float","args":["600"]}],1792720300000]\n')
  got = bench().measured(run)
  assert got["ran_again_after_a_kill"] == [{"command": "npx ava", "interrupted": "bash16", "again": ["bash27"]}]


def test_the_report_covers_each_run(tmp_path: Path) -> None:
  """The report gives the measures of each run under its folder, and their sums."""
  shutil.copytree(RUN, tmp_path / RUN.name)
  args = type("Args", (), {"out": tmp_path})()
  assert bench().report(args) == 0
  said = (tmp_path / "report.json").read_text(encoding="utf-8")
  assert '"tasks": 1' in said
  assert '"replies_asked_again": 1' in said
