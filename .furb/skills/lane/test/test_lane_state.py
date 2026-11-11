"""lane_state, one line of a chain for the watch of the root."""

from skills.conftest import Bench


async def test_lane_state_gives_one_line_of_a_chain(laning: Bench) -> None:
  """lane_state(c) gives one line of the chain c: its label, whether it is paused, its open prompts, commands, waits and rungs, its replies and the end of its last word."""
  laning.words["item"] = ['close("done")']
  await laning.word('c = chain("item")\njob = prompt(str, "do", to="claude-cli:sonnet/high", on=c)')
  await laning.value("await job")
  got = await laning.value("lane_state(c)")
  c = laning.ns["c"]
  assert got == f"{c} 'item': paused=False open={{'prompt': [], 'bash': [], 'wait': [], 'rung': []}} replies=1 last word='close(\"done\")'"
