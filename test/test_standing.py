"""standing, what the chains stand on."""

from collections.abc import Generator

from conftest import STANDS, Sand, heads, kernel, life, relived, said, settle
from furb import engine


async def test_standing_gives_what_the_chains_stand_on() -> None:
  """standing gives what the chains stand on: the answer of the last stand that the transcript of the root holds, and an empty standing before the first."""
  seen: list[object] = []

  def world() -> Generator[tuple | None, tuple]:
    while True:
      a = yield
      if a[0] == "stand":
        seen.append(engine.standing())
        yield "done", a[1], STANDS

  engine.boot((), **kernel(), world=world())
  assert seen == [[]] and engine.standing() == STANDS == engine.peek("stand1")
  later = [[STANDS[0][0]], "/z", "operator"]
  seen.clear()
  sand = Sand(stands=later)
  _, root = life(sand)
  assert engine.standing() == later
  sand.stands = STANDS
  assert engine.stand() == STANDS and engine.standing() == STANDS
  assert [a[1] for a in engine.transcript(root) if a[0] == "stand"] == ["stand1", "stand2"]


async def test_it_reads_the_transcript_of_the_root_as_it_stands_where_the_call_is_made() -> None:
  """It reads the transcript of the root as it stands where the call is made, so a grant reads the window of an actor off the standing where the answer of its reply lands, and a later life reads at each place of the record the standing that the record held there."""
  sand = Sand(stands=STANDS, cost=(200000, 0, 0, 0, 0.0))
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  engine.grant(share=0.9, on=root)
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  (answered,) = [a[1] for a in said(log, "rung") if a[2] == one]
  ledger = [f"#{answered} ledger spent=0.0 filled=0.5"]
  assert [a for a in heads(engine.turns(on=root)) if " ledger " in a] == ledger
  later = Sand(stands=[[STANDS[0][0]], "/z", "operator"])
  await relived(later, list(sand.record))
  assert engine.standing() == later.stands
  assert [a for a in heads(engine.turns(on=root)) if " ledger " in a] == ledger
