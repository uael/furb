"""AGAIN, the growth after a compaction before compacted asks choice again."""

from pathlib import Path

from furb import engine
from skills.conftest import compacting


async def test_again_is_the_share_that_an_answer_reads_past_the_first_answer_after_a_compaction(tmp_path: Path) -> None:
  """AGAIN is the share of the window that an answer reads past the first answer after a compaction of compacted before compacted asks choice again, and it is 0.15 unless a word rebinds it."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close(AGAIN)", "AGAIN = 0.3\nclose(AGAIN)"]
  assert (await engine.prompt(float, "again", on=root), await engine.prompt(float, "again", on=root)) == (0.15, 0.3)
