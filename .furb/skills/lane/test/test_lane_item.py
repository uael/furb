"""lane_item, one row of the lane from A to Z."""

import shlex

import pytest

from conftest import said
from skills.conftest import Bench
from furb.engine import Refused

from ...conftest import gitted


def made(laning: Bench, name: str) -> str:
  command = f"cd {shlex.quote(str(laning.tmp))} && echo {name} > {name} && git add {name} && git commit -qm {name}"
  return f'await bash({command!r})\nclose("made {name}")'


async def test_lane_item_runs_a_row_from_a_to_z(laning: Bench) -> None:
  """lane_item(id) runs the row id from A to Z: lane_open on its tree, lane_work on its done command or else on the command of the worker, and lane_land once accepted, and it keeps the state, the cause, the done command and the time of the end in the row."""
  tree = str(laning.tmp)
  laning.words["item"] = ['close(("the reading", "test -f b"))', made(laning, "b")]
  laning.words["item probe claude-cli:opus/high"] = ["close(None)"]
  await laning.word(f'lane_ask("item", "Add b.", "message 1", tree={tree!r})')
  assert await laning.value('await lane_item("item")') == ("landed", "test -f b")
  [row] = await laning.value("lane_rows()")
  assert {k: row[k] for k in ("state", "cause", "done", "ended")} == {"state": "landed", "cause": "test -f b", "done": "test -f b", "ended": 1000.0}
  assert gitted(laning.origin, "log", "--format=%s", "main").split() == ["b", "base"]
  assert [x[6] for x in said(laning.log, "prompt")] == ["claude-cli:sonnet/medium", "claude-cli:sonnet/medium", "claude-cli:opus/high"]


async def test_lane_item_refuses_a_tree_that_a_row_not_over_holds(laning: Bench) -> None:
  """It refuses a row whose tree or branch a row that is not over holds, and names that owner."""
  await laning.word('lane_ask("a", "a", "s", state="running", tree="t")\nlane_ask("b", "b", "s", tree="t")')
  with pytest.raises(Refused, match="a, which is not over, holds the tree or the branch of b"):
    await laning.word('await lane_item("b")')
  assert [x["state"] for x in await laning.value("lane_rows()")] == ["running", "open"]


async def test_lane_item_gives_the_effort_by_the_kind_of_the_row(laning: Bench) -> None:
  """Its worker is the one of the row, else sonnet at high for a hard row and at medium for any other."""
  tree = str(laning.tmp)
  for one in ("plain", "hard", "ruled"):
    laning.words[one] = ['close(("r", "true"))']
  await laning.word(f'lane_ask("plain", "p", "s", tree={tree!r})\nlane_ask("hard", "h", "s", hard=2, tree={tree!r})\nlane_ask("ruled", "r", "s", worker="claude-cli:opus/high", why="a ruling", tree={tree!r})')
  for one in ("plain", "hard", "ruled"):
    assert await laning.value(f"await lane_item({one!r})") == ("void", "the acceptance command passed before any work")
  assert [x[6] for x in said(laning.log, "prompt")] == ["claude-cli:sonnet/medium", "claude-cli:sonnet/high", "claude-cli:opus/high"]
