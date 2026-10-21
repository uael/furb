"""ouroboros, the doctrine of a chain, told."""

from pathlib import Path

from conftest import heads, of, settle
from extensions.conftest import extended
from furb import engine


async def test_the_doctrine_of_a_chain_told(tmp_path: Path) -> None:
  """The doctrine of a chain, told: an act on the chain that tells LORE when the chain holds no tell of an ouroboros act, and that is done with nothing at once, so a chain with a source that holds the doctrine of its origin reads it once."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  two = engine.chain("two", root)
  await settle()
  assert [engine.peek(one, "living") for one in ("ouroboros1", "ouroboros2")] == [None, None]
  assert [len(of(engine.turns(on=one), "ouroboros")) for one in (root, two)] == [1, 1]


async def test_the_life_word_of_the_extension_is_ouroboros(tmp_path: Path) -> None:
  """The life word of the extension is ouroboros(), so a chain reads the doctrine at its birth, before its first prompt asks a model."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  two = engine.chain("two")
  engine.prompt(None, "go", on=two)
  await settle()
  said = heads(engine.turns(on=two))
  assert said.index("#ouroboros2") < said.index("#prompt1")
  assert of(engine.turns(on=root), "ouroboros1") != []
