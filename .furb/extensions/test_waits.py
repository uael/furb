"""Each word of the extensions of the project names an act in each wait, since the engine refuses a wait of no act."""

import ast
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
WORDS = sorted(
  one
  for place in (ROOT / ".furb" / "extensions", ROOT / "extensions")
  for one in place.rglob("*.py")
  if "test" not in one.parent.name and not one.name.startswith(("test_", "conftest"))
)


def bare(call: ast.Call) -> bool:
  """Whether a wait may name no act: it has no act after its seconds, or only a starred list that may be empty, unless
  that list falls back on an act with or."""
  acts = call.args[1:]
  sure = lambda one: not isinstance(one, ast.Starred) or isinstance(one.value, ast.BoolOp)  # noqa: E731
  return not any(map(sure, acts))


def waits(text: str) -> list[int]:
  calls = [x for x in ast.walk(ast.parse(text)) if isinstance(x, ast.Call)]
  return sorted(x.lineno for x in calls if isinstance(x.func, ast.Name) and x.func.id == "wait" and bare(x))


@pytest.mark.parametrize("word", WORDS, ids=lambda one: str(one.relative_to(ROOT)))
def test_each_wait_of_a_word_names_an_act(word: Path) -> None:
  assert waits(word.read_text(encoding="utf-8")) == []


def test_a_wait_of_seconds_alone_or_of_a_list_that_may_be_empty_is_bare() -> None:
  text = "wait(60)\nwait(EVERY, *living())\nwait(1, x)\nwait(1, *(acts or [here]))\n"
  assert waits(text) == [1, 2]
