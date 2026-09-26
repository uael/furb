"""Cd, the question that answers with its path."""

from conftest import Sand, life, said
from furb import engine
from furb.engine import OPERATOR


async def test_a_cd_is_a_question_that_answers_with_its_path_which_the_chain_holds() -> None:
  """A cd is a question that answers with its path, which the chain holds."""
  sand = Sand()
  log, root = life(sand)
  assert engine.cd("/x", on=root) == "/x"
  held = engine.transcript(root)
  word = next(a for a in held if a[0] == "cd")
  assert engine.question(word) and word == ("cd", word[1], OPERATOR, root, "/x")
  answered = [a for a in said(log, "done") if a[1] == word[1]]
  assert [(a[2], a[3]) for a in answered] == [(root, "/x")]
  assert [a for a in sand.calls if a[0] == "cd"] == []
