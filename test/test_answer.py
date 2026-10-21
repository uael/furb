"""answer, the answer of the last question of a kind."""

from conftest import born
from furb import engine
from furb.engine import ROOT


async def test_the_answer_of_the_last_question_of_a_kind_on_a_chain_that_is_of_the_type_of_empty_and_empty_when_no_such_question_stands() -> (
  None
):
  """The answer of the last question of a kind on a chain that is of the type of empty, and empty when no such question stands."""
  _, _, root = born()
  assert engine.answer("cd", root, "") == ""
  engine.cd("/x", on=root)
  engine.cd("/y", on=root)
  assert engine.answer("cd", root, "") == "/y"
  assert engine.answer("cd", root, 0) == 0
  assert engine.answer("stand", ROOT, []) == engine.standing() != []
