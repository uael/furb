"""lane_next, the next row of the lane."""

from skills.conftest import Bench

ROWS = """lane_ask("z", "z", "s", state="landed")
lane_ask("a", "a", "s", hard=1, after=["z"])
lane_ask("b", "b", "s")
lane_ask("c", "c", "s", hard=3, tree="t")
lane_ask("d", "d", "s", state="running", tree="t")
lane_ask("e", "e", "s", hard=2, after=["b"])
lane_ask("m", "m", "s", main=True)"""


async def test_lane_next_gives_the_hardest_row_that_waits_for_nothing(laning: Bench) -> None:
  """lane_next() gives the id of the hardest open row whose after list names only landed or void rows and whose tree and branch no running row holds, or an empty string."""
  await laning.word(ROWS)
  assert await laning.value("lane_next()") == "a"
  await laning.word('LANE_ITEMS = "none.json"')
  assert await laning.value("lane_next()") == ""


async def test_lane_next_gives_only_the_fix_of_main_while_main_is_red(laning: Bench) -> None:
  """When red says that main is red, it gives only a row whose key main says that it fixes main."""
  await laning.word(ROWS)
  assert await laning.value("lane_next(True)") == "m"
