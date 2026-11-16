"""todo_add, a task that a word adds to the list."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import tasks, todo


async def test_todo_add_is_a_task_that_a_word_adds_to_the_list(tmp_path: Path) -> None:
  """A task that a word adds to the list: an act that is done at once with its text, which the list hears."""
  _, root = todo(tmp_path)
  await settle(300)
  assert await engine.rung("todo_add('buy milk')\ntodo_add('walk the dog')", on=root) is None
  added = [a for a in engine.transcript(root) if a[0] == "todo_add"]
  assert [engine.peek(a[1]) for a in added] == ["buy milk", "walk the dog"]
  await settle(300)
  assert tasks(root) == ["○ buy milk", "○ walk the dog"]
