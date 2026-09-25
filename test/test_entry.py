"""Entry, one entry of the record."""

from conftest import STANDS, Dead, Sand, acts, life, lived, plain, relived, said, settle, sown
from furb import engine
from furb.engine import Text


async def test_one_entry_of_the_record_the_fact() -> None:
  """One entry of the record: the fact, an act among them, and the answer of an act after it, as every other fact about it."""
  sand = sown()
  await lived(sand)
  assert all(len(e) == 1 for e in sand.record)
  facts = [e[0] for e in sand.record]
  read = next(i for i, a in enumerate(facts) if a[0] == "read")
  assert facts[read + 1] == ("done", facts[read][1], "world", Text("/w/a.txt", "one\ntwo\n"))


async def test_the_order_of_the_record_is_what_puts_an_entry_back_in_its_place() -> None:
  """The order of the record is what puts an entry back in its place in a later life."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  again, over = await relived(Dead(stands=STANDS), list(sand.record))
  assert over == root
  assert [one[1] for one in said(again, "prompt")] == [said(log, "prompt")[0][1]]
  assert engine.peek(said(log, "prompt")[0][1]) == 1
  facts = [(e[0][0], e[0][1], *e[0][3:]) for e in sand.record if e[0][0] != "started" and not engine.question(e[0])]
  heard = [(a[0], a[1], *a[3:]) for a in again if a[2] == "record" and a[0] not in ("keep", "stood")]
  assert [a for a in heard if not engine.question(a)] == facts != []


async def test_the_world_keeps_each_entry_as_the_record_says_it_plain_or_not() -> None:
  """The World keeps each entry as the record says it, plain or not."""
  sand = sown()
  log, root = await lived(sand)
  assert [one[3] for one in said(log, "keep")] == sand.record
  kept = plain(sand.record)
  assert kept != list(sand.record)
  _, over = await relived(sown(), kept)
  assert over == root


async def test_the_record_is_a_sequence_of_entries_about_acts() -> None:
  """The record is a sequence of entries about acts."""
  sand = sown()
  log, _ = await lived(sand)
  assert all(isinstance(e, tuple) for e in sand.record)
  assert all(e[0][1] in acts(log) for e in sand.record)
  kinds = ["chain", "stand", "done", "prompt", "reply", "started", "done", "gate", "done", "read", "done"]
  kinds += ["bash", "started", "merged", "out", "done", "reply", "started", "done", "gate", "done"]
  assert [e[0][0] for e in sand.record] == kinds
