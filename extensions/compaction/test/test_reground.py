"""reground, the files that a chain kept, read again."""

import re
from pathlib import Path

from conftest import of, settle
from extensions.conftest import compacting
from furb import engine


async def test_reground_reads_again_each_file_that_the_chain_holds_a_read_or_write_of(tmp_path: Path) -> None:
  """reground() reads again each file that the chain holds a read or write of from another chain, once each, in the order of the first read or write, shows only the lines that changed since the last text that the chain holds of it, and skips a file that it cannot read, so the chain sees each file that it kept as it is now."""
  sand, root, work = compacting(tmp_path)
  word = "read('a.txt')\nwrite(Text('b.txt', 'b'))\nwrite(Text('c.txt', 'c'))\nread('a.txt')\nclose(1)"
  sand.script[root] = [word]
  assert await engine.prompt(int, "read and write", on=root) == 1
  sand.files[f"{work}/a.txt"] = "one\nfresh\n"
  del sand.files[f"{work}/c.txt"]
  two = engine.chain("two", root)
  await settle()
  assert await engine.rung("reground()", on=two) is None
  told = [re.sub(r"read\d+", "readN", one) for one in of(engine.turns(on=two), "read")[-2:]]
  assert told == [
    f"#readN\nreadN_path = {f'{work}/a.txt'!r}\nreadN_text = 'fresh'",
    f"#readN\nreadN_path = {f'{work}/b.txt'!r}\nreadN_lines = 1",
  ]
