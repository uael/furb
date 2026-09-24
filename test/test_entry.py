"""Entry, one entry of the record."""

from conftest import STANDS, Dead, Sand, life, lived, plain, relived, said, settle, sown
from furb import engine


async def test_one_entry_of_the_record_the_fact() -> None:
  """One entry of the record: the fact; for a query of a run, the query and what it was answered beside, since a query is answered at once and its answer travels with it."""
  sand = sown()
  log, _ = await lived(sand)
  clocks = [e for e in sand.record if e[0][0] == "clock"]
  assert [(len(e), e[1]) for e in clocks] == [(2, 1001.0)]
  assert [len(e) for e in sand.record if e[0][0] not in ("clock", "stand")] == [1] * (len(sand.record) - 2)
  assert [e[0] for e in sand.record if len(e) == 1] == [one for one in log if (one,) in sand.record]


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
  assert engine.outcomes[said(log, "prompt")[0][1]] == 1
  facts = [(e[0][0], e[0][1], *e[0][3:]) for e in sand.record if not engine.question(e[0])]
  assert [(a[0], a[1], *a[3:]) for a in again if a[2] == "record" and a[0] != "done"] == facts != []


async def test_the_world_keeps_each_entry_as_the_journal_says_it_plain_or_not() -> None:
  """The World keeps each entry as the journal says it, plain or not."""
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
  await lived(sand)
  assert all(isinstance(e, tuple) for e in sand.record)
  assert all(e[0][1] in engine.acts or e[0][1] in engine.asked for e in sand.record)
  kinds = ["chain", "stand", "prompt", "rung", "answer", "clock", "wait", "done", "rung", "answer"]
  assert [e[0][0] for e in sand.record] == kinds
