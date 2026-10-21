"""LABEL, the label of a chain that compact makes."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_label_is_the_label_of_the_chain_that_compact_makes(tmp_path: Path) -> None:
  """LABEL is the label of the chain that compact makes when it is given none, and it is compacted unless a word rebinds it."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["made = compact()\nLABEL = 'kept'\nclose([made, compact(), compact(label='mine')])"]
  chains = await engine.prompt(list, "three", on=root)
  assert [engine.get(one)[4] for one in chains] == ["compacted", "kept", "mine"]
