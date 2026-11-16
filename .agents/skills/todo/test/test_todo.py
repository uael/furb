"""todo, the doctrine of the extension."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import todo


async def test_todo_is_the_doctrine_of_the_skill(tmp_path: Path) -> None:
  """todo is the doctrine of the extension, a quote that says what it does."""
  sand, root = todo(tmp_path)
  await settle(300)
  sand.script[root] = ["close(todo)"]
  doctrine = await engine.prompt(str, "doctrine", on=root)
  parts = ["todo_list()", "todo_add(text)", "todo_check(text)", "footer", "todo", "The skill plays on the root alone."]
  assert [part for part in parts if part not in doctrine] == []
