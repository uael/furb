"""Overflow, what a reply is refused with when its prompt passes the window of its model."""

import pytest

from conftest import born
from furb import engine
from furb.engine import Overflow, Refused


async def test_what_a_reply_is_refused_with_when_its_prompt_passes_the_window_of_its_model() -> None:
  """What a reply is refused with when its prompt passes the window of its model, so the chain can cut its acts before its next reply."""
  _, _, root = born(auto=False)
  asked = engine.prompt(int, "go", on=root)
  engine.close(Overflow("m answered nothing: Prompt is too long"), asked)
  with pytest.raises(Refused, match="Prompt is too long") as raised:
    await asked
  assert isinstance(raised.value, Overflow)
