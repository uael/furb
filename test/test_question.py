"""question, whether a fact is a question."""

from conftest import life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR


async def test_whether_a_fact_is_a_question_which_its_name_says() -> None:
  """Whether a fact is a question, which its name says: a question is about itself, and its name is under its kind."""
  sand = sown()
  log, root = life(sand)
  act = engine.wait(0, on=root)
  assert (await act) is None
  await settle()
  made = said(log, "wait")[0]
  assert engine.question(made) and made[1] == act == "wait1"
  assert engine.clock(on=root) == 1001.0
  asked = said(log, "clock")[0]
  assert engine.question(asked) and asked[1] == "clock@operator.3"
  assert engine.question(("chain", root, OPERATOR, "", "root", ""))
  assert not engine.question(("cancel", root, OPERATOR, []))
  assert not engine.question(("close", "chain12", OPERATOR, None, []))
  assert not engine.question(("chain", "chain", OPERATOR, "", "root", ""))
  assert [a for a in log if not engine.question(a)] != []
  assert not any(engine.question(a) for a in [*said(log, "tell"), *said(log, "done"), *said(log, "keep")])
