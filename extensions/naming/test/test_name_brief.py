"""NAME_BRIEF, what the model that names the session reads."""

from pathlib import Path

from extensions.conftest import naming
from furb import engine


async def test_name_brief_is_what_the_model_that_names_the_session_reads(tmp_path: Path) -> None:
  """NAME_BRIEF is what the model that names the session reads before the first message of the operator."""
  sand, root = naming(tmp_path)
  await engine.rung("NAME_BRIEF = 'Name it: '", on=root)
  sand.script[root] = ["close(1)"]
  await engine.prompt(int, "the login form fails", on=root)
  (asked,) = [a for a in engine.transcript("chain2") if a[0] == "prompt" and a[2] == "name_session1"]
  assert asked[5] == "Name it: the login form fails"
