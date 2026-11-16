"""todo_check, a task that a word checks."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import tasks, todo


async def test_todo_check_is_a_task_that_a_word_checks(tmp_path: Path) -> None:
  """A task that a word checks, or unchecks when it is checked: an act that is done at once with its text, which the list hears, and which changes each task of that text."""
  _, root = todo(tmp_path)
  await settle(300)
  await engine.rung("todo_add('a')\ntodo_add('b')\ntodo_add('a')", on=root)
  await settle(300)
  await engine.rung("todo_check('a')", on=root)
  await settle(300)
  assert tasks(root) == ["✓ a", "○ b", "✓ a"]
  await engine.rung("todo_check('a')", on=root)
  await settle(300)
  checked = [engine.peek(a[1]) for a in engine.transcript(root) if a[0] == "todo_check"]
  assert checked == ["a", "a"] and tasks(root) == ["○ a", "○ b", "○ a"]
