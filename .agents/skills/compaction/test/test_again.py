"""AGAIN, the growth after a compaction before compacted asks choice again."""

from pathlib import Path

from furb import engine
from skills.conftest import compacting


async def test_again_is_the_share_that_an_answer_reads_past_the_first_answer_after_a_compaction(tmp_path: Path) -> None:
  """AGAIN is the share of the window that an answer reads past the first answer after a compaction of compacted before compacted asks choice again, and it is 0.15 unless the word of an extension rebinds it, since a word of the chain never reaches the module of the extensions."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close(AGAIN)", "await rung(Text('mine.py', 'AGAIN = 0.3'))\nclose(AGAIN)"]
  assert (await engine.prompt(float, "again", on=root), await engine.prompt(float, "again", on=root)) == (0.15, 0.3)
