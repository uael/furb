"""FILTERS, the filters that compact applies when it is given none."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_filters_are_the_filters_that_compact_applies_in_order(tmp_path: Path) -> None:
  """FILTERS are the filters that compact applies in order when it is given none: latest(), then recent(8)."""
  sand, root, _ = compacting(tmp_path)
  word = "acts = [a for a in transcript() if question(a)]\nclose([f(acts) == g(acts) for f, g in zip(FILTERS, [latest(), recent(8)], strict=True)])"
  sand.script[root] = ["read('a.txt')\nread('a.txt')", word]
  assert await engine.prompt(list, "the defaults", on=root) == [True, True]
