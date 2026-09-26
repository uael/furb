"""Keep, one entry of the record, kept and said."""

import json

from conftest import Sand, born, lived, relived, said, settle, wire
from furb import engine
from furb.engine import Exit


async def test_one_entry_of_the_record_kept_and_said() -> None:
  """One entry of the record, kept and said: the journal holds it for this life and the World for the next."""
  sand, log, root = born()
  act = engine.bash("echo hi", on=root)
  await act
  await settle()
  kept = said(log, "keep")
  assert [one[3] for one in kept] == sand.record
  assert [fact[0] for fact, *_ in sand.record] == ["chain", "stand", "done", "bash", "started", "merged", "out", "done"]
  assert {one[2] for one in kept} == {"journal"}
  _, over = await relived(Sand(), list(sand.record))
  got = engine.peek(act)
  assert over == root and isinstance(got, Exit) and got.code == 0


async def test_the_journal_says_one_keep_per_entry() -> None:
  """The journal says one keep per entry, and the World keeps it as plain data if it likes."""
  sand, log, _ = await lived()
  assert [one[3] for one in said(log, "keep")] == sand.record
  wired = wire(sand.record)
  assert json.loads(json.dumps(wired)) == wired
