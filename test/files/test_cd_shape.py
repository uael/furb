"""Cd, the query of a path that the chain holds."""

from conftest import FILES, STANDS, Sand, life, said, verb
from furb import engine
from furb.engine import OPERATOR


async def test_a_cd_is_a_query_of_a_path_which_the_chain_holds_in_its_transcript() -> None:
  """A cd is a query of a path, which the chain holds in its transcript, and which nobody need answer."""
  sand = Sand(stands=STANDS, words=FILES)
  log, root = life(sand)
  assert verb("cd", root)("/x") == "/x"
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  asked = next(a for a in held if a[0] == "cd")
  assert engine.question(asked) and asked == ("cd", asked[1], OPERATOR, root, "/x")
  assert asked[1] in engine.asked and asked[1] not in engine.acts
  assert [a for a in said(log, "done") if a[1] == asked[1]] == [] and engine.outcomes.get(asked[1]) is None
