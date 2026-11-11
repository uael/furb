"""Cut, what the model of a chain reads from it on."""

from conftest import born, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_cut_names_what_the_model_of_a_chain_reads_from_it_on() -> None:
  """A cut names what the model of a chain reads from it on: the facts that it keeps, then each fact said on the chain after it."""
  _, _, root = born("say('cut', ROOT, [a for a in transcript() if a[0] != 'tell'])\nclose(k)")
  assert await engine.rung("k = 7", on=root) is None
  assert await engine.prompt(int, "the k", on=root) == 7
  told = [a for a in engine.transcript(root) if a[0] == "tell" and a[1] == "rung1"]
  assert told and [a for a in engine.reading(root) if a[0] == "tell" and a[1] == "rung1"] == []
  assert [a for a in engine.reading(root) if a[:2] == ("done", "prompt1")]


async def test_a_cut_takes_nothing_from_the_transcript_of_its_chain() -> None:
  """A cut takes nothing from the transcript of its chain, so the chain only grows, and its module, its ladders and its acts go on."""
  _, log, root = born("say('cut', ROOT, [])\nclose(k)")
  assert await engine.rung("k = 7", on=root) is None
  before = [a[:3] for a in engine.transcript(root)]
  chains = said(log, "chain")
  assert await engine.prompt(int, "the k", on=root) == 7
  assert [a[:3] for a in engine.transcript(root)][: len(before)] == before
  assert said(log, "chain") == chains and engine.module(root)["k"] == 7


async def test_a_cut_that_a_prefix_or_a_cut_carries_in_its_list_cuts_nothing() -> None:
  """A cut that a prefix or a cut carries in its list cuts nothing, since only a cut said on a chain names what its model reads."""
  _, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  engine.say("cut", root, [("cut", root, OPERATOR, [])])
  assert await engine.rung("k = 2", on=root) is None
  twin = engine.chain("twin", source=root)
  await settle()
  shown = repr(engine.turns(on=twin))
  assert "k = 2" in shown and [a for a in engine.reading(twin) if a[0] == "cut"]
