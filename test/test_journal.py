"""Journal, what boot takes, which says again what the World kept of the life before it and keeps what it says."""

from conftest import Bare, Sand, keeping, kernel
from furb import engine


async def test_boot_calls_the_journal_with_the_four_names_and_the_tables_of_the_life() -> None:
  """boot calls the journal with numbered, offers, drive and lives, the four names that the journal binds again, and by name with the tables of the life: the ears of the outside, its acts by name, the names of the acts that an ear took, its facts by chain and by view, the function that holds a fact on a chain, its log, which holds each fact until each ear that waits for it heard it, and the highest number of each kind; boot binds those four names again to the first four that the journal gives, and brings the ear that the journal gives last to life last, under the name journal."""
  heard: list[tuple] = []
  journal = Bare(keeping(heard))
  root = engine.boot(journal, **kernel(), world=Sand().hears())
  assert [x.__name__ for x in journal.names] == ["numbered", "offers", "drive", "lives"]  # ty: ignore[unresolved-attribute]
  assert [*journal.names] == [engine.numbered, engine.offers, engine.drive, engine.lives]
  tables = journal.tables
  outside, known, taken, held = tables["outside"], tables["known"], tables["taken"], tables["held"]
  holds, log, top = tables["holds"], tables["log"], tables["top"]
  assert [*outside] == [*kernel(), "world"] and outside["world"] is engine.listening()["world"]
  assert known[root] == engine.get(root) and known["stand1"] == engine.get("stand1") and {root, "stand1"} <= taken
  assert held[root, ""] == engine.transcript(root) and held[root, "stand"] == engine.transcript(root, "stand")
  assert (log, dict(top)) == ([], {"chain": 1, "stand": 1})
  holds(root, [("note", root, "mine")])
  assert engine.transcript(root, "note") == engine.transcript(root, "mine") == [("note", root, "mine")]
  assert engine.listening()["journal"] is journal.ear and [*engine.listening()][-2:] == ["journal", root]
  assert engine.get(root) in heard and engine.bash("echo one", on=root) == "bash1" and top["bash"] == 1
