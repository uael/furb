"""Prefix, what a chain with a source holds of the transcript of its origin."""

from conftest import born, said, settle
from furb import engine
from furb.engine import take


async def test_a_prefix_carries_what_a_chain_with_a_source_holds_of_the_transcript_of_its_origin() -> None:
  """A prefix carries what a chain with a source holds of the transcript of its origin, which the chain says at its birth, before its open."""
  _, log, root = born()
  kept, dropped = await engine.rung("k = 1", on=root), engine.bash("echo hi", on=root)
  await settle()
  before = engine.transcript(root)
  child = engine.chain("child", source=root, filter=take(dropped, inside=False))
  await settle()
  (prefix,) = said(log, "prefix")
  (opened,) = [a for a in log if a[:2] == ("started", child)]
  assert kept is None and prefix[1:3] == (child, child) and log.index(prefix) < log.index(opened)
  assert prefix[3] == [a for a in before if a[1] != dropped] != before


async def test_the_transcript_of_the_chain_holds_the_facts_of_its_prefix_where_the_prefix_stands() -> None:
  """The transcript of the chain holds the facts of its prefix where the prefix stands, and not the prefix itself."""
  _, log, root = born()
  assert await engine.rung("k = 1", on=root) is None
  child = engine.chain("child", source=root)
  await settle()
  (prefix,) = said(log, "prefix")
  held = engine.transcript(child)
  assert (
    held[: len(prefix[3])] == prefix[3]
    and said(held, "prefix") == []
    and held[len(prefix[3])][:2] == ("started", child)
  )


async def test_a_prefix_takes_the_place_of_what_the_transcript_of_the_chain_that_it_is_said_on_held() -> None:
  """A prefix takes the place of what the transcript of the chain that it is said on held, so a word that says a prefix on its own chain cuts its turns, and the chain goes on under its name, with its module, its ladders and its acts."""
  _, log, root = born("say('prefix', ROOT, [a for a in transcript() if a[0] != 'tell'])\nclose(k)")
  assert await engine.rung("k = 7", on=root) is None
  chains = said(log, "chain")
  assert await engine.prompt(int, "the k", on=root) == 7
  assert said(log, "chain") == chains and engine.module(root)["k"] == 7
  assert [a for a in engine.transcript(root) if a[0] == "tell" and a[1] == "rung1"] == []
