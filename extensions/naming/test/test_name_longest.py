"""NAME_LONGEST, the longest name of the session."""

from pathlib import Path

from conftest import STANDS
from extensions.conftest import naming
from furb import engine


async def test_name_longest_is_the_number_of_characters_past_which_a_name_is_cut(tmp_path: Path) -> None:
  """NAME_LONGEST is the number of characters past which a name of the session is cut."""
  sand, root = naming(tmp_path, STANDS[0])
  sand.script[root] = ["close([NAME_LONGEST, len(name_of('y' * 200))])"]
  assert await engine.prompt(list, "how long", on=root) == [60, 60]
