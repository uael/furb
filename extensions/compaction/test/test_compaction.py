"""compaction, the fill of a chain, watched."""

from pathlib import Path

from conftest import heads, of, settle
from extensions.conftest import Sand, extended
from furb import engine

FULL = (300000, 0, 0, 0, 0.0)
"""An answer that reads three quarters of the window of the default actor of the suite, which is 400000 tokens."""


def filled(id: str, share: float, word: str) -> str:
  """The paragraph that compaction tells when an answer fills the chain past its share."""
  return f"#{id} filled\n{id}_share = {share!r}\n{id}_word = {word!r}"


async def watching(tmp_path: Path, cost: tuple, to: str = "") -> tuple[Sand, str, str]:
  """A root that watches its fill, and whose prompt the model answers at that cost: the World, the root and the watcher."""
  sand, root = extended("compaction", tmp_path)
  watcher = await engine.rung("close(compaction())", on=root)
  assert isinstance(watcher, str)
  sand.cost = cost
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "go", to=to, on=root) == 1
  await settle()
  return sand, root, watcher


async def test_the_fill_of_a_chain_watched(tmp_path: Path) -> None:
  """The fill of a chain, watched: an act on the chain that, at the first answer of a model on the chain that reads more than SHARE of the window of its actor, tells the share that the answer filled and the word that goes on in a chain with a source and a take of the old work, once, and that never completes, so its end asks the model nothing."""
  sand, root, watcher = await watching(tmp_path, FULL)
  word = "chain('compacted', source='chain1', filter=take(inside=False))"
  assert of(engine.turns(on=root), "compaction") == [filled(watcher, 0.75, word)]
  sand.script[root] = ["close(2)"]
  assert await engine.prompt(int, "again", on=root) == 2
  await settle()
  assert len(of(engine.turns(on=root), "compaction")) == 1
  assert engine.peek(watcher, "watching") == "watching"


async def test_the_share_is_the_tokens_that_the_answer_read_over_the_window_of_the_actor_of_its_reply(
  tmp_path: Path,
) -> None:
  """The share is the tokens that the answer read over the window of the actor of its reply, in the standing where the answer lands."""
  _, root, watcher = await watching(tmp_path, (150000, 0, 0, 0, 0.0), to="n/low")
  assert of(engine.turns(on=root), "compaction")[0].split("\n")[1] == f"{watcher}_share = 0.75"


async def test_an_answer_that_reads_share_of_the_window_or_less_tells_nothing(tmp_path: Path) -> None:
  """An answer that reads SHARE of the window or less tells nothing, and the act watches on."""
  _, root, watcher = await watching(tmp_path, (240000, 0, 0, 0, 0.0))
  assert of(engine.turns(on=root), "compaction") == []
  assert engine.peek(watcher, "watching") == "watching"


async def test_the_word_that_it_tells_opens_a_chain_whose_source_is_the_chain(tmp_path: Path) -> None:
  """The word that it tells opens a chain whose source is the chain, and whose filter is a take that is not inside of each prompt on the chain that is done, so the new chain keeps the work that is still open."""
  sand, root = extended("compaction", tmp_path)
  watcher = await engine.rung("close(compaction())", on=root)
  assert isinstance(watcher, str)
  sand.script[root] = ["close(1)", "close(2)"]
  done = engine.prompt(int, "one", on=root)
  assert await done == 1
  sand.cost = FULL
  assert await engine.prompt(int, "two", on=root) == 2
  await settle()
  word = f"chain('compacted', source='chain1', filter=take({done!r}, inside=False))"
  assert of(engine.turns(on=root), "compaction") == [filled(watcher, 0.75, word)]


async def test_it_does_not_act_for_the_model(tmp_path: Path) -> None:
  """It does not act for the model: it tells the word and runs nothing, so the model decides whether and when to go on in the new chain."""
  _, root, _ = await watching(tmp_path, FULL)
  assert engine.get("chain2") is None
  assert [a[0] for a in engine.transcript(on=root) if a[0] in ("chain", "prompt")] == ["prompt"]


async def test_the_life_word_of_the_extension_is_compaction(tmp_path: Path) -> None:
  """The life word of the extension is compaction(), so each chain watches its fill from its birth, and a host turns it off as it turns off any extension, with false in a config."""
  sand, root = extended("compaction", tmp_path, lives=True)
  two = engine.chain("two")
  sand.cost = FULL
  sand.script[two] = ["close(None)"]
  await engine.prompt(None, "go", on=two)
  await settle()
  assert "#compaction2 filled" in heads(engine.turns(on=two))
  assert of(engine.turns(on=root), "compaction") == []
