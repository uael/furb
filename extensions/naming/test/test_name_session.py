"""name_session, the name of the session."""

import ast
from pathlib import Path

from conftest import STANDS, plain, settle
from extensions.conftest import naming
from furb import engine
from furb.engine import site
from furb_monty import _monty

NAMING = next(x for x in _monty.official() if x["name"] == "naming")
BRIEF = next(
  ast.literal_eval(one.value)
  for one in ast.parse(str(NAMING["word"])).body
  if isinstance(one, ast.Assign) and [ast.unparse(x) for x in one.targets] == ["NAME_BRIEF"]
)


def names(root: str) -> list[object]:
  """The value of each act of name_session on the root, in order."""
  return [engine.peek(a[1]) for a in engine.transcript(root) if a[0] == "name_session"]


async def test_name_session_is_the_name_of_the_session(tmp_path: Path) -> None:
  """The name of the session: an act on the root that waits for the first prompt that the operator makes in the life, and that is done with a name of the session that its message gives."""
  sand, root = naming(tmp_path)
  sand.script.update({root: ["close(1)", "close(2)"], "chain2": ["close('Fix the login form')"]})
  await settle(300)
  assert names(root) == [None]
  # A fact of the operator about a prompt that a model made is no prompt of the operator.
  token = site.set(root)
  try:
    asked = engine.prompt(str, "a model asks", to="operator", on=root)
  finally:
    site.reset(token)
  engine.pause(asked)
  await settle(300)
  assert names(root) == [None]
  assert await engine.prompt(int, "the login form fails", on=root) == 1
  await settle(300)
  assert names(root) == ["Fix the login form"]
  assert await engine.prompt(int, "now the logout", on=root) == 2
  await settle(300)
  assert names(root) == ["Fix the login form"]
  assert [a[3] for a in engine.transcript(root) if a[0] == "chain"] == [root]
  (tmp_path / "member").mkdir()
  sand, root = naming(tmp_path / "member")
  sand.script.update({root: ["close(1)"], "chain2": ["close('Fix the login form')"]})
  token = site.set("operator/uael")
  try:
    assert await engine.prompt(int, "the login form fails", on=root) == 1
  finally:
    site.reset(token)
  await settle(300)
  assert names(root) == ["Fix the login form"]


async def test_the_life_word_of_the_extension_is_name_session(tmp_path: Path) -> None:
  """The life word of the extension is name_session(), and the extension plays on the root alone, so a life names its session once."""
  assert NAMING["life"] == "name_session()" and NAMING["root"]
  sand, root = naming(tmp_path)
  two = engine.chain("two")
  await settle(300)
  sand.script[two] = ["close(1)"]
  assert await engine.prompt(int, "go", on=two) == 1
  await settle(300)
  assert [a[3] for a in engine.transcript(root) if a[0] == "name_session"] == [root]
  assert [a for a in engine.transcript(two) if a[0] == "name_session"] == []


async def test_it_asks_the_name_of_the_actor_that_automatic_gives(tmp_path: Path) -> None:
  """It asks the name of the actor that automatic() gives, with a prompt of str on a new chain of its own labelled naming, which reads NAME_BRIEF and then the message."""
  sand, root = naming(tmp_path)
  sand.script[root] = ["close(1)"]
  await engine.prompt(int, "the login form fails", on=root)
  await settle(300)
  (made,) = [a for a in engine.transcript(root) if a[0] == "chain"]
  (asked,) = [a for a in engine.transcript("chain2") if a[0] == "prompt" and a[3] == "chain2"]
  assert made[4:6] == ("naming", "") and made[2] == "name_session1"
  assert asked[4:7] == ("str", f"{BRIEF}the login form fails", "h")


async def test_with_no_such_actor_the_name_is_name_of_the_message(tmp_path: Path) -> None:
  """With no such actor, or when that prompt gives no name, the name is name_of the message."""
  sand, root = naming(tmp_path, STANDS[0])
  sand.script[root] = ["close(1)"]
  await engine.prompt(int, "\n  'the login form fails.'\nit says 500", on=root)
  await settle(300)
  assert names(root) == ["the login form fails"]
  assert [a for a in engine.transcript(root) if a[0] == "chain"] == []
  (tmp_path / "two").mkdir()
  sand, root = naming(tmp_path / "two")
  sand.script.update({root: ["close(1)"], "chain2": ["close('...')"]})
  await engine.prompt(int, "the logout", on=root)
  await settle(300)
  assert names(root) == ["the logout"]


async def test_the_journal_keeps_the_prompt_and_its_answer(tmp_path: Path) -> None:
  """The journal keeps the prompt and its answer, so a later life on the same record gives the same name and asks no model."""
  sand, root = naming(tmp_path)
  sand.script.update({root: ["close(1)"], "chain2": ["close('Fix the login form')"]})
  await engine.prompt(int, "the login form fails", on=root)
  await settle(300)
  record = plain(sand.record)
  later, again = naming(tmp_path, record=record)
  await settle(300)
  assert names(again) == ["Fix the login form"]
  assert [a for a in later.calls if a[0] == "reply"] == []
