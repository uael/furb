"""name_of, the name that a text holds."""

from pathlib import Path

from conftest import STANDS
from extensions.conftest import naming
from furb import engine


async def test_name_of_gives_the_name_that_a_text_holds(tmp_path: Path) -> None:
  """name_of(text) gives the name that a text holds: its first line that holds more than quotes, marks and periods, with none of them around it, cut to NAME_LONGEST characters, else an empty str."""
  sand, root = naming(tmp_path, STANDS[0])
  sand.script[root] = [
    "close([name_of('\\n ** \\n\"Fix it.\"\\nmore'), name_of('# ' + 'x' * 70 + ' y'), name_of(' .. ')])"
  ]
  assert await engine.prompt(list, "names", on=root) == ["Fix it", "x" * 60, ""]
