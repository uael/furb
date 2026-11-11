"""lane_fill, the rows that the lane starts."""

from conftest import said, settle
from skills.conftest import Bench

from furb.engine import OPERATOR


async def test_lane_fill_starts_rows_up_to_the_cap(laning: Bench) -> None:
  """lane_fill() starts lane_item on the runner of each next row while fewer rows run than the key cap of the skill in the configs, and gives the ids that it started."""
  laning.say("run list", "success\n")
  await laning.word('lane_ask("a", "a", "s", tree="ta")\nlane_ask("b", "b", "s", hard=2, tree="tb")\nlane_ask("c", "c", "s", hard=1, tree="tc")')
  assert await laning.value("await lane_fill()") == ["b", "c"]
  assert [x["state"] for x in await laning.value("lane_rows()")] == ["open", "running", "running"]
  runners = laning.ns["lane_runners"]
  assert sorted(runners) == ["b", "c"]
  assert [x[4].endswith("\nawait lane_item('b')") for x in said(laning.log, "rung") if x[3] == runners["b"][0] and "lane_item('b')" in str(x[4])] == [True]
  assert await laning.value("await lane_fill()") == []


async def test_lane_fill_reads_main_as_red_when_its_gates_failed(laning: Bench) -> None:
  """It reads main as red when the last run of the gates on main failed."""
  laning.say("run list --repo uael/furb --branch main --workflow gates.yml", "failure\n")
  await laning.word('lane_ask("a", "a", "s", hard=5, tree="ta")\nlane_ask("m", "m", "s", main=True, tree="tm")')
  assert await laning.value("await lane_fill()") == ["m"]


async def test_lane_fill_starts_nothing_after_three_blocked_rows(laning: Bench) -> None:
  """After three blocked rows in a row, it starts nothing and tells the operator once."""
  laning.say("run list", "success\n")
  await laning.word("\n".join(f'lane_ask("x{n}", "x", "s", state="blocked", ended={n}.0)' for n in (1, 2, 3)) + '\nlane_ask("c", "c", "s", tree="tc")')
  assert await laning.value("await lane_fill()") == []
  assert await laning.value("await lane_fill()") == []
  told = [x[5] for x in said(laning.log, "prompt") if x[6] == OPERATOR and x[4] == "None"]
  assert told == ["The lane starts nothing: x1, x2, x3 ended blocked in a row."]


async def test_lane_fill_keeps_the_watcher_of_memory_while_a_row_runs(laning: Bench) -> None:
  """It keeps script/memwatch.sh running while a row runs, and ends it when none runs."""
  laning.say("run list", "success\n")
  laning.sand.hold = "memwatch"
  await laning.word('lane_ask("a", "a", "s", tree="ta")')
  await laning.value("await lane_fill()")
  [guard] = [x[1] for x in said(laning.log, "bash") if x[4] == "sh script/memwatch.sh"]
  assert laning.engine.peek(guard) is None
  await laning.value("await lane_fill()")
  assert [x[1] for x in said(laning.log, "bash") if x[4] == "sh script/memwatch.sh"] == [guard]
  await laning.word('lane_put("a", state="landed")')
  await laning.value("await lane_fill()")
  await settle()
  assert laning.engine.peek(guard) is not None


async def test_lane_fill_reopens_a_row_that_runs_on_no_runner(laning: Bench) -> None:
  """It takes a running row whose runner holds no open word back to open, since that row runs nowhere and holds no place."""
  laning.say("run list", "success\n")
  laning.sand.files[laning.sand.placed(laning.root, "settings://lane")] = '{"cap": 1}'
  await laning.word('lane_ask("a", "a", "s", state="running", tree="ta")\nlane_ask("b", "b", "s", tree="tb")')
  assert await laning.value("await lane_fill()") == ["a"]
  assert [x["state"] for x in await laning.value("lane_rows()")] == ["running", "open"]
  assert await laning.value("await lane_fill()") == []
