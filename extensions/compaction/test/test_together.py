"""together, filters in order."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_together_gives_each_filter_what_the_one_before_it_kept(tmp_path: Path) -> None:
  """together(*filters) gives each filter what the one before it kept, in order, and gives what the last one kept."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = [
    "acts = [a for a in transcript() if question(a)]\n"
    "close([together(lambda x: x[1:], lambda x: x[:1])(acts) == acts[1:2], together()(acts) == acts, len(acts) > 2])"
  ]
  assert await engine.prompt(list, "in order", on=root) == [True, True, True]
