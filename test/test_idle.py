"""idle, the generator of the file that listens and says nothing."""

import pytest

from conftest import born
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_idle_waits_for_the_facts_about_its_own_name_and_says_nothing_of_its_own() -> None:
  """idle waits for the facts about its own name and says nothing of its own, which is the ear of a reply and the ear of the operator."""
  hears = engine.idle("reply1")
  assert next(hears) == {"reply1"}
  assert hears.send(("tell", "chain1", OPERATOR, [])) == {"reply1"}
  assert hears.send(("done", "reply1", OPERATOR, 1)) == {"reply1"}
  born()
  with pytest.raises(Refused, match="operator hears"):
    engine.drive(engine.idle(OPERATOR), OPERATOR)
