"""look, the doctrine of a chain of a local model, told."""

from pathlib import Path

from conftest import heads, of, settle
from extensions.conftest import extended
from furb import engine

LOCAL = "local:qwen"
"""A model of a local server, as the actor of a chain."""


async def test_the_doctrine_of_a_chain_of_a_local_model_told(tmp_path: Path) -> None:
  """The doctrine of a chain of a local model, told: an act on the chain that tells LOOKING when the actor of the chain is a model of a local server and the chain holds no tell of a look act, and that is done with nothing at once, so a chain with a source that holds the doctrine of its origin reads it once."""
  _, root = extended("look", tmp_path, lives=True, actor=LOCAL)
  two = engine.chain("two", root)
  await settle()
  assert [engine.peek(one, "living") for one in ("look1", "look2")] == [None, None]
  assert [len(of(engine.turns(on=one), "look")) for one in (root, two)] == [1, 1]


async def test_on_a_chain_of_any_other_actor_look_tells_nothing(tmp_path: Path) -> None:
  """On a chain of any other actor, look tells nothing, so a model of the network reads none of it."""
  _, root = extended("look", tmp_path, lives=True)
  await settle()
  assert (engine.peek("look1", "living"), of(engine.turns(on=root), "look")) == (None, [])


async def test_the_life_word_of_the_extension_is_look(tmp_path: Path) -> None:
  """The life word of the extension is look(), so a chain of a local model reads the doctrine at its birth, before its first prompt asks a model."""
  _, root = extended("look", tmp_path, lives=True, actor=LOCAL)
  two = engine.chain("two")
  engine.prompt(None, "go", on=two)
  await settle()
  said = heads(engine.turns(on=two))
  assert said.index("#look2") < said.index("#prompt1")
  assert of(engine.turns(on=root), "look1") != []
