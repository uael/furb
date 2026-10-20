"""latest, what the last act of a kind on a chain came to."""

from conftest import born
from furb import engine
from furb.engine import ROOT


async def test_what_the_last_act_of_a_kind_on_a_chain_came_to_that_is_of_the_type_of_the_empty_value() -> None:
  """What the last act of a kind on a chain came to that is of the type of the empty value it is given, and that value when no such act stands, which is how the standing and the working directory are read off a transcript, each from its one home."""
  _, _, root = born()
  assert engine.latest("cd", root, "") == "" and engine.latest("wait", root, []) == []
  assert engine.latest("stand", ROOT, []) == engine.standing() != []
  engine.cd("/x", on=root)
  engine.cd("/y", on=root)
  assert engine.latest("cd", root, "") == "/y" == engine.cwd(on=root)
  assert engine.latest("cd", root, 0) == 0
