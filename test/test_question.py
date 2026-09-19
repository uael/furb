"""question, whether a fact is a question."""

import pytest

from conftest import life, said, settle, sown
from furb import engine
from furb.engine import Text


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_whether_a_fact_is_a_question_which_its_name_says() -> None:
  """Whether a fact is a question, which its name says: a question is about itself, and its name is under its kind."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  made = said(log, "bash")[0]
  assert engine.question(made) and made[1] == act
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  asked = said(log, "read")[0]
  assert engine.question(asked) and asked[1].startswith("read://")
  assert engine.question(("chain", root, "operator", "", "root", ""))
  assert [a for a in log if not engine.question(a)] != []
  assert not any(engine.question(a) for a in [*said(log, "tell"), *said(log, "done"), *said(log, "keep")])
