"""latest, the filter of the last read or write of each path."""

from pathlib import Path

from conftest import of, settle
from extensions.conftest import compacting
from furb import engine


async def test_latest_keeps_of_each_path_only_the_last_read_or_write(tmp_path: Path) -> None:
  """latest() keeps of each path only the last read or write, so a text that a later read or write of that path replaced is gone, and it keeps every other act."""
  sand, root, work = compacting(tmp_path)
  word = "read('a.txt')\nwrite(Text('a.txt', 'new'))\nread('a.txt')\nclose(chain('two', 'chain1', latest()))"
  sand.script[root] = [word]
  two = await engine.prompt(str, "read and compact", on=root)
  await settle()
  assert of(engine.turns(on=two), "read") == ["#read2\nread2_path = 'a.txt'\nread2_text = 'new'"]
  assert [turn[1] for turn in engine.turns(on=two) if turn[0] == "assistant"] == [word]
  assert work in sand.files[f"{work}/a.txt"] or sand.files[f"{work}/a.txt"] == "new"
