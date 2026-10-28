"""reground, the files that a chain kept, read again."""

import re
from pathlib import Path

from conftest import of
from extensions.conftest import compacting
from furb import engine


async def test_reground_reads_again_each_file_of_each_read_or_write_whose_tell_the_chain_holds(tmp_path: Path) -> None:
  """reground() reads again each file of each read or write whose tell the chain holds, once each, in the order of the first read or write, shows only the lines that changed since the last text that the chain holds of it, and skips a file that it cannot read, so the chain sees each file that it kept as it is now."""
  sand, root, work = compacting(tmp_path)
  word = "read('a.txt')\nwrite(Text('b.txt', 'b'))\nwrite(Text('c.txt', 'c'))\nread('a.txt')\nread('d.txt', HIDDEN)\nclose(1)"
  sand.files[f"{work}/d.txt"] = "d"
  sand.script[root] = [word]
  assert await engine.prompt(int, "read and write", on=root) == 1
  sand.files[f"{work}/a.txt"] = "one\nfresh\n"
  sand.files[f"{work}/d.txt"] = "e"
  del sand.files[f"{work}/c.txt"]
  reads = len([a for a in engine.transcript(root) if a[0] == "read"])
  assert await engine.rung("reground()", on=root) is None
  told = [re.sub(r"read\d+", "readN", one) for one in of(engine.turns(on=root), "read")[-2:]]
  assert told == [
    f"#readN\nreadN_path = {f'{work}/a.txt'!r}\nreadN_text = 'fresh'",
    f"#readN\nreadN_path = {f'{work}/b.txt'!r}",
  ]
  assert len([a for a in engine.transcript(root) if a[0] == "read"]) == reads + 3
