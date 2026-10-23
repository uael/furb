"""LISTED, the most acts of one step that choice lists."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_listed_is_the_most_acts_of_one_step_that_choice_lists(tmp_path: Path) -> None:
  """LISTED is the most acts of one step that choice lists, and it is 40 unless a word rebinds it."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close(LISTED)"]
  assert await engine.prompt(int, "listed", on=root) == 40
