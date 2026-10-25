"""carry, what a chain of another record holds, as the World reads it."""

from pathlib import Path

from conftest import plain, settle
from extensions.conftest import carried, extended, importing
from furb import engine


async def test_carry_asks_the_world_for_what_the_chain_source_of_the_record_holds(tmp_path: Path) -> None:
  """carry(record, source) asks the World for what the chain source of the record at the path record holds, as an act of the kind carry with the words record and source, and gives the act."""
  _, root, heard = importing(tmp_path)
  act = await engine.rung("close(carry('old.jsonl', 'chain3'))", on=root)
  assert isinstance(act, str)
  assert heard == [("carry", act, engine.get(act)[2], root, "old.jsonl", "chain3")]
  assert engine.peek(act) == carried(act)


async def test_the_record_keeps_its_answer(tmp_path: Path) -> None:
  """The record keeps its answer, so a later life reads it from its own record and opens no other file."""
  sand, root, heard = importing(tmp_path)
  act = await engine.rung("close(carry('old.jsonl'))", on=root)
  assert isinstance(act, str)
  extended("compaction", tmp_path, record=plain(sand.record))
  await settle()
  assert len(heard) == 1
  assert engine.peek(act) == carried(act)
