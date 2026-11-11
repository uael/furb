"""lane_ask, the ask of the owner kept as a row of the lane."""

import json

import pytest

from skills.conftest import Bench
from furb.engine import Refused


async def test_lane_ask_keeps_the_ask_verbatim_as_a_row(laning: Bench) -> None:
  """lane_ask(id, ask, source, done) keeps the ask verbatim as an open row of LANE_ITEMS, with its source, the command that says it is done and each other key of row, and refuses an id that a row holds."""
  await laning.word('lane_ask("a", "Fix it,  now.", "message 3", "test -f a", tree="t")')
  row = {"id": "a", "ask": "Fix it,  now.", "source": "message 3", "done": "test -f a", "state": "open", "tree": "t"}
  assert json.loads(laning.sand.files[laning.sand.placed(laning.root, ".furb/lane.json")]) == [row]
  with pytest.raises(Refused, match="the row a stands"):
    await laning.word('lane_ask("a", "Again.", "message 4")')
  assert await laning.value("lane_rows()") == [row]


async def test_lane_ask_refuses_a_worker_with_no_why(laning: Bench) -> None:
  """It refuses a worker with no why, since the effort follows the kind of the work: medium for work, high for a hard row, and more only with a reason."""
  with pytest.raises(Refused, match="names the worker claude-cli:opus/high with no why"):
    await laning.word('lane_ask("a", "Fix it.", "message 3", worker="claude-cli:opus/high")')
  await laning.word('lane_ask("a", "Fix it.", "message 3", worker="claude-cli:opus/high", why="a ruling")')
  assert [x["why"] for x in await laning.value("lane_rows()")] == ["a ruling"]
