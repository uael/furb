"""reground, the files that a chain kept, read again."""

import re
from pathlib import Path

from conftest import of, settle
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


async def test_reground_shows_a_file_whole_again_when_its_kept_read_is_one_of_a_reground(tmp_path: Path) -> None:
  """reground() shows a file whole again when the read that the chain holds of it is one that a reground made, which showed only what changed, so a chain cut twice in a row still sees the text of each file that it kept."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["read('a.txt')\nclose(1)"]
  assert await engine.prompt(int, "read", on=root) == 1
  for n in range(2):
    sand.script[root].append("close(compact())")
    assert await engine.prompt(str, f"compact {n}", on=root) == root
    await settle()
  shown = [one for one in of(engine.turns(on=root), "read") if "one\ntwo" in one or "'one'" in one]
  assert len(shown) == 1
