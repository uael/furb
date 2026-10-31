"""ouroboros, the doctrine of a chain, told."""

from pathlib import Path

from conftest import chained, of, settle
from extensions.conftest import extended
from extensions.ouroboros.test.test_lore import LORE
from furb import engine


async def test_the_doctrine_of_a_chain_told(tmp_path: Path) -> None:
  """The doctrine of a chain, told: an act on the chain that tells LORE when the chain holds no tell of an ouroboros act, and that is done with nothing at once, so a chain with a source that holds the doctrine of its origin reads it once."""
  _, root = extended("ouroboros", tmp_path)
  await engine.rung("ouroboros()", on=root)
  two = engine.chain("two", root)
  await settle()
  await engine.rung("ouroboros()", on=two)
  assert [engine.peek(one, "living") for one in ("ouroboros1", "ouroboros2")] == [None, None]
  assert [len(of(engine.turns(on=one), "ouroboros")) for one in (root, two)] == [1, 1]


async def test_the_extension_has_no_life_word(tmp_path: Path) -> None:
  """The extension has no life word, since the setup of the root reads the file of the extension, and the turn after that read holds the code of the extension alone, which holds LORE, so each chain with no source reads the doctrine once, in its setup, and the chain tells no act of it."""
  _, root = extended("ouroboros", tmp_path, lives=True, doctrine="Read first.\n")
  two = await chained("two")
  for on in (root, two):
    turns = engine.turns(on=on)
    (at,) = [n for n, one in enumerate(turns) if one[0] == "assistant" and one[1].endswith("ouroboros.py', HIDDEN)")]
    assert turns[at + 1][0] == "user"
    assert turns[at + 1][1].startswith("import json\n") and LORE in turns[at + 1][1]
    assert of(turns, "ouroboros") == []
