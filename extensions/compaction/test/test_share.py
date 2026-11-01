"""SHARE, the share of the window past which compacted compacts."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_share_is_the_share_of_the_window_past_which_compacted_compacts(tmp_path: Path) -> None:
  """SHARE is the share of the window of its actor that an answer reads past, after which compacted asks choice what to keep of its chain, and it is 0.6 unless a word rebinds it."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  sand.cost = (240000, 0, 0, 0, 0.0)
  sand.script[root] = ["x = 1", "close(SHARE)"]
  assert await engine.prompt(float, "share", on=root) == 0.6
  assert engine.get("chain2") is None
  sand.cost = (244000, 0, 0, 0, 0.0)
  sand.script[root] = ["x = 2", "close(x)"]
  assert await engine.prompt(int, "past", on=root) == 2
  assert engine.get("chain2")[3:] == (root, "chosen", "")
