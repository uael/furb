"""journal, what boot takes as its journal."""

from conftest import WORLD, Sand, given, kernel, lived, settle, watched
from furb import checkpoint, engine
from furb.engine import OPERATOR


async def test_journal_gives_what_boot_takes_as_its_journal() -> None:
  """journal gives what boot takes as its journal, from the names of an engine and a record, so a life that boots on it says that record again and keeps what it says after it, as boot says."""
  sand, _, root = await lived()
  record, turns = list(sand.record), engine.turns(on=root)
  later, log = Sand(), list[tuple]()
  made = checkpoint.journal(vars(engine), given(record))
  over = engine.boot(made, **kernel(), probe=watched(log), world=later.hears())
  await settle(300)
  assert over == root and engine.turns(on=over) == turns and [a[0] for a in later.calls] == ["stand"]
  assert [e[0][:3] for e in later.record] == [("stand", "stand2", OPERATOR), ("done", "stand2", WORLD)]


async def test_a_journal_that_is_given_no_record_says_nothing_again() -> None:
  """A journal that is given no record says nothing again, so its life is a new life, which names each act by its number alone."""
  sand, log = Sand(), list[tuple]()
  root = engine.boot(checkpoint.journal(vars(engine)), **kernel(), probe=watched(log), world=sand.hears())
  assert root == "chain1" and engine.transcript(root, "journal") == []
  assert [e[0][:2] for e in sand.record] == [("chain", "chain1"), ("stand", "stand1"), ("done", "stand1")]
  assert engine.bash("echo one", on=root) == "bash1" and engine.bash("echo two", on=root) == "bash2"
