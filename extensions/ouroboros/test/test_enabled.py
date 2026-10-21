"""enabled, the extensions that the life runs."""

from pathlib import Path

from extensions.conftest import extended
from furb import engine


async def test_enabled_gives_the_name_of_each_extension_that_the_life_runs(tmp_path: Path) -> None:
  """enabled gives the name of each extension that the life runs, once, in the order the life enabled it, as the enable facts of the root say, and it asks nothing."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  before = len(engine.transcript(on=root))
  assert await engine.rung("close(enabled())", on=root) == ["ouroboros"]
  assert {a[0] for a in engine.transcript(on=root)[before:] if engine.question(a)} == {"rung", "gate", "run"}
