"""scope, the chain a question is on."""

from conftest import life, said, settle, sown
from furb import engine


async def test_the_scope_of_a_question_from_its_name() -> None:
  """The scope of a question, from its name: the chain it is on, and itself for a chain, and nothing for a name of no question of the life."""
  sand = sown()
  log, root = life(sand)
  assert engine.scope(root) == root
  act = engine.bash("echo hi", on=root)
  assert engine.scope(act) == root
  assert (await act).code == 0
  await settle()
  engine.read("a.txt", on=root)
  _, asking, *_ = said(log, "read")[0]
  assert engine.scope(asking) == root
  assert engine.scope("bash://operator.9") == ""
  assert engine.scope("") == ""
