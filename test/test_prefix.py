"""Prefix, what a chain with a source holds of the transcript of its origin."""

from conftest import Sand, life, said, settle
from furb import engine
from furb.engine import take


async def test_a_prefix_carries_what_a_chain_with_a_source_holds_of_the_transcript_of_its_origin() -> None:
  """A prefix carries what a chain with a source holds of the transcript of its origin, which the chain says at its birth, before its open."""
  sand = Sand()
  log, root = life(sand)
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
  sand = Sand()
  log, root = life(sand)
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
