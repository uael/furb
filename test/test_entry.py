"""Entry, one entry of the record."""

from conftest import Dead, Sand, acts, born, lived, plain, relived, said, settle, sown
from furb import engine
from furb.engine import Text


async def test_one_entry_of_the_record_the_fact() -> None:
  """One entry of the record: a fact, its place, which counts the facts that the life said before it, and whether the life rested as it said it, an act among them, and the answer of an act after it, as every other fact about it."""
  sand, log, _ = await lived()
  facts = [a for a in log if a[0] != "keep"]
  assert all(len(e) == 3 and facts[e[1]] == e[0] for e in sand.record)
  assert [e[0][:2] for e in sand.record if e[2]][:2] == [("chain", "chain1"), ("stand", "stand1")]
  assert ("read", "read1") in [e[0][:2] for e in sand.record if not e[2]]
  facts = [e[0] for e in sand.record]
  read = next(i for i, a in enumerate(facts) if a[0] == "read")
  assert facts[read + 1] == ("done", facts[read][1], "world", Text("/w/a.txt", "one\ntwo\n"))


async def test_the_order_of_the_record_is_what_puts_an_entry_back_in_its_place() -> None:
  """The order of the record is what puts an entry back in its place in a later life."""
  sand, log, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  again, over = await relived(Dead(), list(sand.record))
  assert over == root
  assert [one[1] for one in said(again, "prompt")] == [said(log, "prompt")[0][1]]
  assert engine.peek(said(log, "prompt")[0][1]) == 1
  facts = [(e[0][0], e[0][1], *e[0][3:]) for e in sand.record if not engine.question(e[0])]
  heard = [(a[0], a[1], *a[3:]) for a in again if a[2] == "journal" and a[0] != "keep"]
  assert [a for a in heard if not engine.question(a)] == facts != []


async def test_the_world_keeps_each_entry_as_the_journal_says_it_plain_or_not() -> None:
  """The World keeps each entry as the journal says it, plain or not."""
  sand, log, root = await lived()
  assert {one[2] for one in said(log, "keep")} == {"journal"}
  kept = plain(sand.record)
  assert kept == list(sand.record)
  later = sown()
  _, over = await relived(later, kept)
  assert over == root and engine.peek(said(log, "prompt")[0][1]) == 2
  assert [one[0] for one in later.calls] == ["stand"]


async def test_the_record_is_a_sequence_of_entries_about_acts() -> None:
  """The record is a sequence of entries about acts."""
  sand, log, _ = await lived()
  assert all(isinstance(e, tuple) for e in sand.record)
  assert all(e[0][1] in acts(log) for e in sand.record)
  kinds = ["chain", "stand", "done", "prompt", "rung", "run", "reply", "started", "done", "run", "read"]
  kinds += ["done"]
  kinds += ["bash", "started", "merged", "wants", "out", "done"]
  assert [e[0][0] for e in sand.record] == kinds


async def test_an_entry_that_the_life_said_as_it_rested_the_journal_says_only_once_the_life_rests_again() -> None:
  """An entry that the life said as it rested, and an entry with no place, as each entry that seed gives, the journal says only once the life rests again, so each ear heard each fact before it, as in the life that said it."""
  sand, first, root = born()
  word = "b = bash('echo hi')\nawait wait(0.05, b)\nlast = bash('echo ' + str(peek(b).code))\nawait last"
  await engine.rung(word, on=root)
  await settle()
  rested = [e[0][:2] for e in sand.record if e[2]]
  assert ("out", "bash1") in rested and ("done", "bash1") in rested
  again, _ = await relived(Sand(), list(sand.record))
  heard = [a[:2] for a in first if a[0] != "keep"]
  assert [a[:2] for a in again if a[0] != "keep"][: len(heard)] == heard
