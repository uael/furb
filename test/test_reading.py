"""reading, what the model of a chain reads."""

from conftest import born, settle
from furb import engine


async def test_reading_gives_what_the_model_of_a_chain_reads() -> None:
  """reading gives what the model of a chain reads: the facts that the last cut on the chain kept, then each fact said on the chain after that cut, or the whole transcript of a chain that holds no cut."""
  _, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  assert [a[:3] for a in engine.reading(root)] == [a[:3] for a in engine.transcript(root)]
  engine.say("cut", root, [])
  engine.say("cut", root, engine.transcript(root)[:2])
  assert await engine.rung("k = 2", on=root) is None
  held = engine.transcript(root)
  at = max(i for i, a in enumerate(held) if a[0] == "cut")
  assert [a[:3] for a in engine.reading(root)] == [a[:3] for a in [*held[:2], *held[at + 1 :]]]


async def test_reading_gives_a_new_list_at_each_call_and_asks_nothing() -> None:
  """reading gives a new list at each call, and asks nothing, so no act is made and the journal keeps nothing."""
  sand, log, root = born()
  await settle()
  facts, kept = len(log), len(sand.record)
  engine.reading(root).clear()
  assert engine.reading(root) and (len(log), len(sand.record)) == (facts, kept)
