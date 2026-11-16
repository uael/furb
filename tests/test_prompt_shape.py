"""Prompt, the shape, the message and the actor of a prompt."""

from conftest import born, said
from furb import engine


async def test_a_prompt_carries_the_name_of_the_shape_the_message_and_the_actor_of_a_prompt() -> None:
  """A prompt carries the name of the shape, the message and the actor of a prompt."""
  _, log, root = born()
  act = engine.prompt(int, "how many?", "m/high", on=root)
  other = engine.prompt(None, "and now?", on=root)
  asked = said(log, "prompt")
  assert [a[1] for a in asked] == [act, other]
  assert [a[4:] for a in asked] == [("int", "how many?", "m/high"), ("None", "and now?", "")]
