"""SHARE, the share of the window past which compaction tells."""

from pathlib import Path

from conftest import settle
from extensions.conftest import extended
from furb import engine


async def test_share_is_the_share_of_the_window_of_an_actor(tmp_path: Path) -> None:
  """SHARE is the share of the window of an actor that an answer of a model reads past, at which compaction tells the model to go on in a new chain."""
  sand, root = extended("compaction", tmp_path)
  watcher = await engine.rung("close(compaction())", on=root)
  assert isinstance(watcher, str)
  sand.cost = (244000, 0, 0, 0, 0.0)
  sand.script[root] = ["close(SHARE)"]
  assert await engine.prompt(float, "share", on=root) == 0.6
  await settle()
  assert engine.turns(on=root)[-1][1].startswith(f"#{watcher} filled\n{watcher}_share = 0.61\n")
