"""lane_open, the reading of an item and its command."""

from conftest import said
from skills.conftest import Bench


async def test_lane_open_gives_the_chain_the_reading_and_the_command(laning: Bench) -> None:
  """lane_open(id, evidence, tree) makes the chain id, asks worker there, under the rules of the lane and with the ask of the row id verbatim, for its reading of the item and one shell command that fails now, and gives the chain, the reading and the command."""
  tree = str(laning.tmp)
  laning.words["item"] = ['close(("the reading", "test -f b"))']
  await laning.word(f'lane_ask("item", "Add  b.", "message 1")\nwant = lane_rules({tree!r}) + "\\nThe ask item, in the words of the owner, from message 1. They stay in this life: quote them in no commit, pull request or issue.\\nAdd  b.\\n\\nThe evidence:\\nthe evidence\\n" + LANE_OPENING')
  c, reading, command = await laning.value(f'await lane_open("item", "the evidence", {tree!r})')
  assert (laning.engine.get(c)[4], reading, command) == ("item", "the reading", "test -f b")
  assert [(x[3], x[5], x[6]) for x in said(laning.log, "prompt")] == [(c, laning.ns["want"], "claude-cli:sonnet/medium")]


async def test_lane_open_gives_the_refusal_when_the_worker_gives_no_pair(laning: Bench) -> None:
  """It gives the chain, an empty reading and the refusal when the worker gives no pair of strings within patience seconds."""
  laning.words["item"] = []
  c, reading, why = await laning.value(f'LANE_LOOK = 0.02\nlane_ask("item", "a", "s")\ngot = await lane_open("item", "", {str(laning.tmp)!r}, patience=0.05)')
  assert (laning.engine.get(c)[4], reading) == ("item", "")
  assert why.startswith("Refused('prompt") and why.endswith(" did not close in 0.05 s')")
