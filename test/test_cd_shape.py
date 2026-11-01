"""Cd, the question of the directory a path names."""

from conftest import WORLD, born, said
from furb import engine
from furb.engine import OPERATOR


async def test_a_cd_is_the_question_of_the_directory_a_path_names() -> None:
  """A cd is the question of the directory a path names, which the World answers and the transcript of the chain holds."""
  sand, log, root = born()
  assert engine.cd("/x", on=root) == "/x"
  held = engine.transcript(root)
  word = next(a for a in held if a[0] == "cd")
  assert engine.question(word) and word == ("cd", word[1], OPERATOR, root, "/x")
  answered = [a for a in said(log, "done") if a[1] == word[1]]
  assert [(a[2], a[3]) for a in answered] == [(WORLD, "/x")]
  assert [a for a in sand.calls if a[0] == "cd"] == [word]
