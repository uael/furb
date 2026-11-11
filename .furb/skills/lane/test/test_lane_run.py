"""lane_run, the runner chain of each lane."""

from conftest import said, settle
from skills.conftest import Bench


async def until(laning: Bench, done: object) -> None:
  for _ in range(100):
    if done():
      return
    await settle()


async def test_lane_run_runs_two_lanes_at_once(laning: Bench) -> None:
  """lane_run(label, word) runs word on the runner chain of label, which it makes once, so the lanes of two labels run at once."""
  laning.sand.hold = "lane-two"
  word = "await bash('echo lane-two')"
  await laning.word(f"a = lane_run('a', {word!r})\nb = lane_run('b', {word!r})\nagain = lane_run('a', 'x = 1')")
  held = lambda: [x for x in said(laning.log, "bash") if "lane-two" in x[4]]  # noqa: E731
  await until(laning, lambda: len(held()) == 2)
  runners = laning.ns["lane_runners"]
  assert [laning.engine.get(runners[k][0])[4] for k in ("a", "b")] == ["run a", "run b"]
  assert sorted(x[3] for x in held()) == sorted(runners[k][0] for k in ("a", "b"))
  assert not any(laning.engine.peek(x[1]) for x in held())
  assert laning.engine.get(laning.ns["again"])[3] == runners["a"][0]
  laning.sand.release()
  await until(laning, lambda: all(laning.engine.peek(laning.ns[k]) is not None for k in ("a", "b")))
  assert all(laning.engine.peek(x[1]).code == 0 for x in held())


async def test_lane_run_plays_the_kit_again_when_it_changed(laning: Bench) -> None:
  """It plays the word of the kit on the runner before, and again when the kit changed since, so a change of a function reaches each lane at its next word."""
  await laning.word("lane_run('a', 'x = 1')\nlane_run('a', 'x = 2')")
  runner = laning.ns["lane_runners"]["a"][0]
  plays = lambda: [x for x in said(laning.log, "rung") if x[3] == runner and "def lane_run" in str(x[4])]  # noqa: E731
  await until(laning, lambda: laning.engine.module(runner).get("x") == 2)
  assert len(plays()) == 1
  laning.kit(laning.text + "\n\nlane_mark = 3\n")
  await laning.word("lane_run('a', 'y = lane_mark')")
  await until(laning, lambda: laning.engine.module(runner).get("y") == 3)
  assert len(plays()) == 2
