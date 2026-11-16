"""Entry, one entry of the record."""

from collections.abc import Generator

from conftest import Dead, Sand, acts, born, life, lived, plain, relived, said, settle, sown
from furb import engine
from furb.engine import Text


async def test_one_entry_of_the_record_the_fact() -> None:
  """One entry of the record: a fact, its place, which counts the facts that the life said before it, whether the life rested as it said it, and the count of the facts that waited for their ears as the life said it, an act among them, and the answer of an act after it, as every other fact about it."""
  sand, log, _ = await lived()
  facts = [a for a in log if a[0] != "keep"]
  assert all(len(e) == 4 and facts[e[1]] == e[0] for e in sand.record)
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


async def test_an_entry_that_an_ear_of_the_outside_said_as_it_heard_a_fact_the_journal_says_once_as_many_facts_wait_as_its_count() -> (
  None
):
  """An entry that an ear of the outside said as it heard a fact the journal says once as many facts wait as its count, as the first ear of the outside is about to hear the first of them, so each ear of the engine heard that fact before it, as in the life that said it."""
  word = (
    "def pong():\n  pass\ndef note():\n  pass\np = act('pong', '', None)\n"
    "def look(name):\n  yield {'ping'}\n  while True:\n    peek(p, 0) and act('note', '', None)\n    yield {'ping'}\n"
    "act('look', '', look)\nsay('ping', str(p))"
  )
  sand = sown()
  _, root = life(sand, echo=echoing())
  await engine.rung(word, on=root)
  await engine.rung("act('pong', '', None)", on=root)
  await settle()
  assert engine.peek("pong1") == 7 and engine.get("pong2") and engine.get("note1") is None
  assert [e[3] for e in sand.record if e[0][:2] == ("done", "pong1")] == [2]
  life(Sand(), plain(sand.record), echo=echoing())
  await settle(300)
  assert engine.peek("pong1") == 7 and engine.get("pong2") and engine.get("note1") is None


def echoing() -> Generator[tuple | None, tuple]:
  """An ear of the outside that takes each pong, and ends it with 7 as it hears a ping of it, after each ear of the
  engine that waits for that ping."""
  took = set()
  a = yield
  while True:
    if a[0] == "pong" and engine.question(a):
      took.add(a[1])
      yield "started", a[1]
    elif a[0] == "ping" and a[1] in took:
      took.discard(a[1])
      yield "done", a[1], 7
    a = yield
