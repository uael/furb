"""Entry, one entry of the record."""

from conftest import STANDS, Dead, Sand, life, lived, plain, relived, said, settle, sown
from furb import engine
from furb.engine import Text


async def test_one_entry_of_the_record_the_act_made_last_before_its_fact_and_the_fact() -> None:
  """One entry of the record: the act made last before its fact, and the fact; for a query of a run, the query and what it was answered beside, since a query is answered at once and its answer travels with it."""
  sand = sown()
  log, _ = await lived(sand)
  reads = [e for e in sand.record if e[1][0] == "read"]
  assert [(len(e), e[2]) for e in reads] == [(3, Text("/w/a.txt", "one\ntwo\n"))]
  assert [len(e) for e in sand.record if e[1][0] != "read"] == [2] * (len(sand.record) - 1)
  made = {one[1] for one in log if engine.question(one)}
  assert all(e[0] == "" or e[0] in made for e in sand.record)


async def test_an_entry_says_which_act_was_made_last_before_it() -> None:
  """An entry says which act was made last before it, and that is what puts the entry back in its place in a later life."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert [e[0] for e in sand.record][:2] == ["", root]
  again, over = await relived(Dead(stands=STANDS), plain(sand.record))
  assert over == root
  assert [one[1] for one in said(again, "prompt")] == [said(log, "prompt")[0][1]]


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
  assert all(e[1][1] in engine.acts or e[1][1] in engine.asked for e in sand.record)
  kinds = ["chain", "prompt", "rung", "answer", "read", "bash", "out", "exited", "rung", "answer"]
  assert [e[1][0] for e in sand.record] == kinds
