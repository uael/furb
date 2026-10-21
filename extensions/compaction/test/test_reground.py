"""reground, the files read again."""

from pathlib import Path

from conftest import of, settle
from extensions.conftest import compacting
from furb import engine


async def test_reground_reads_again_each_file_that_the_source_read_or_wrote(tmp_path: Path) -> None:
  """reground(source) reads again each file that the source read or wrote, once each, in the order of the first read or write, and skips a file that it cannot read, so the chain sees each file as it is now and no old text of it."""
  sand, root, work = compacting(tmp_path)
  word = "read('a.txt')\nwrite(Text('b.txt', 'b'))\nwrite(Text('c.txt', 'c'))\nread('a.txt')\nclose(1)"
  sand.script[root] = [word]
  assert await engine.prompt(int, "read and write", on=root) == 1
  sand.files[f"{work}/a.txt"] = "fresh"
  del sand.files[f"{work}/c.txt"]
  two = engine.chain("two")
  await settle()
  assert await engine.rung("reground('chain1')", on=two) is None
  assert of(engine.turns(on=two), "read") == [
    f"#read3\nread3_path = '{work}/a.txt'\nread3_text = 'fresh'",
    f"#read4\nread4_path = '{work}/b.txt'\nread4_text = 'b'",
  ]
