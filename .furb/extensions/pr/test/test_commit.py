"""commit makes one commit of the named paths with a QA trailer for each thing to test, on the real engine."""

import subprocess

import pytest

from extensions.conftest import Bench
from furb.engine import Refused


def git(made: Bench, *args: str) -> str:
  return subprocess.run(["git", *args], cwd=made.tmp, capture_output=True, text=True, check=True).stdout


def repo(made: Bench) -> None:
  git(made, "init", "-q")
  git(made, "config", "user.email", "t@example.com")
  git(made, "config", "user.name", "t")
  git(made, "config", "commit.gpgsign", "false")
  (made.tmp / "a.txt").write_text("a\n", encoding="utf-8")
  (made.tmp / "b.txt").write_text("b\n", encoding="utf-8")


async def test_commit_gives_one_qa_trailer_for_each_entry_and_the_sha(pulling: Bench) -> None:
  """commit(title, paths) commits the named paths alone, in the worktree at where, with git commit and every hook, and gives a tuple of the short sha and the QA trailers of the commit."""
  repo(pulling)
  sha, trailers = await pulling.value('await commit("chore(pr): Add a", ["a.txt"], "Why.", qa=["open the TUI", "press q"])')
  assert trailers == ["open the TUI", "press q"]
  assert sha == git(pulling, "rev-parse", "--short", "HEAD").strip()
  assert git(pulling, "log", "-1", "--format=%(trailers:key=QA,valueonly)").split("\n")[:2] == ["open the TUI", "press q"]
  assert git(pulling, "show", "--name-only", "--format=", "HEAD").split() == ["a.txt"]


async def test_commit_with_no_entry_gives_no_trailer_and_carries_the_body(pulling: Bench) -> None:
  """It gives the body as a second paragraph of the message, and one trailer QA: <entry> for each entry of qa, so the QA reads what to test with git log."""
  repo(pulling)
  _, trailers = await pulling.value('await commit("chore(pr): Add b", ["b.txt"], "The body.")')
  assert trailers == []
  assert git(pulling, "log", "-1", "--format=%b").strip() == "The body."
  assert git(pulling, "log", "-1", "--format=%(trailers:key=QA,valueonly)").strip() == ""


async def test_commit_refuses_an_empty_list_of_paths(pulling: Bench) -> None:
  """It refuses an empty list of paths, and its bash has a timeout of 2400 s, since the hooks run the suites."""
  repo(pulling)
  with pytest.raises(Refused, match="at least one path"):
    await pulling.value('await commit("chore(pr): Nothing", [])')
  assert pulling.ns.get("got") is None
