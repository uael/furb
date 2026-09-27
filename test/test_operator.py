"""OPERATOR, the name of the operator."""

from conftest import STANDS, born, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_operator_is_the_name_of_the_operator_in_the_roster_and_as_an_actor() -> None:
  """OPERATOR is the name of the operator in the roster and as an actor."""
  assert OPERATOR == "operator"
  assert [OPERATOR, [], 200000] in STANDS[0]
  _, log, root = born()
  assert engine.get(root) == ("chain", "chain1", OPERATOR, "", "root", "")
  wanted = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert said(log, "reply") == [] and engine.peek(wanted, ...) is ...
  engine.close(12, wanted)
  await settle()
  assert (await wanted) == 12
