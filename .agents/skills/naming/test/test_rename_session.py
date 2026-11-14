"""rename_session, the name that the operator gives the session."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import naming


async def test_rename_session_is_the_name_that_the_operator_gives_the_session(tmp_path: Path) -> None:
  """The name that the operator gives the session: an act on the root that is done at once with the name, less the spaces around it."""
  naming(tmp_path)
  root = "chain1"
  await settle(300)
  assert await engine.rung("rename_session('  Mine ')", on=root) is None
  (made,) = [a for a in engine.transcript(root) if a[0] == "rename_session"]
  assert made[3] == root and engine.peek(made[1]) == "Mine"


async def test_its_value_is_empty_when_the_name_holds_only_spaces(tmp_path: Path) -> None:
  """Its value is empty when the name holds only spaces, which gives the session its default name again."""
  naming(tmp_path)
  root = "chain1"
  await settle(300)
  assert await engine.rung("rename_session('   ')", on=root) is None
  (made,) = [a for a in engine.transcript(root) if a[0] == "rename_session"]
  assert engine.peek(made[1]) == ""
