"""Thread, the shape, the markdown and the actor of a thread."""

from conftest import born, said
from furb import engine


async def test_a_thread_carries_the_name_of_the_shape_the_markdown_and_the_actor_of_a_prompt() -> None:
  """A thread carries the name of the shape, the markdown and the actor of a thread."""
  _, log, root = born()
  act = engine.thread(int, "how many?", "m/high", on=root)
  other = engine.thread(None, "and now?", on=root)
  asked = said(log, "thread")
  assert [a[1] for a in asked] == [act, other]
  assert [a[4:] for a in asked] == [("int", "how many?", "m/high"), ("None", "and now?", "")]
