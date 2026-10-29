"""catchup, the catch-up of a chain."""

from conftest import LATER, Sand, chained, paragraphs, plain, said, settle, sown, takes
from extensions.conftest import CATCHUP, catching, caught, caught_up, number
from furb import engine
from furb.engine import Text

CAP = number(CATCHUP, "CAP")
BUDGET = number(CATCHUP, "BUDGET")


async def test_the_life_word_of_the_extension_is_catchup() -> None:
  """The life word of the extension is catchup(), and the extension plays on every chain, so every chain catches up from its birth."""
  assert CATCHUP["life"] == "catchup()" and not CATCHUP["root"]
  sand, log, root = await caught_up()
  two = await chained("two")
  assert [a[3] for a in said(log, "catchup")] == [root, two]
  engine.read("a.txt", on=two)
  sand.files["/w/a.txt"] = "one\nthree\n"
  engine.stand()
  assert caught(engine.turns(on=two), two) == [f"#{two} caught up\n{two}_changed = {{'/w/a.txt': {{2: 'three'}}}}"]


async def test_on_the_root_it_reads_the_clock_as_it_starts_and_at_each_stand() -> None:
  """On the root it reads the clock as it starts and at each stand, once the chain told the standing that the stand gives it, so the record holds a reading of the clock at the tip of every life."""
  sand, log, root = await caught_up()
  two = await chained("two")
  (watcher,) = [a[1] for a in said(log, "catchup") if a[3] == root]
  assert [a[:4] for a in said(log, "clock")] == [("clock", "clock1", watcher, root)] and engine.peek("clock1") == 1001.0
  later = Sand(stands=LATER)
  again, _ = await catching(later, plain(sand.record))
  assert [a[1:4] for a in said(again, "clock")] == [("clock1", watcher, root), ("clock2", watcher, root)]
  held = engine.transcript(root)
  told = held.index(("tell", root, root, [takes(root, LATER)]))
  assert [a[0] for a in held[told + 1 :]][:2] == ["clock", "done"]
  assert [a[0] for a in later.calls] == ["stand", "clock"]
  assert [a for a in engine.transcript(two) if a[0] == "clock"] == []


async def test_the_catch_up_of_a_chain() -> None:
  """The catch-up of a chain: an act on the chain that never completes, and that catches the chain up at each stand: it reads again, with a hidden show, the path of each text that the last read or write of that path on the chain gave, but a path of a scheme and a door, and it tells what changed in one paragraph headed caught up, about the chain."""
  sand, log, root = await caught_up(files={"/w/a.txt": "one\ntwo\n", "x://y": "y\n"})
  two = await chained("two")
  await engine.bash("echo hi", on=root)
  for path in ("a.txt", "x://y", "bash1/stdout"):
    engine.read(path, on=root)
  engine.read("a.txt", on=two)
  engine.write(Text("b.txt", "b\n"), on=two)
  sand.files.update({"/w/a.txt": "one\nthree\n", "x://y": "z\n", "/w/b.txt": "c\n"})
  calls = len(sand.calls)
  engine.stand()
  assert [(a[3], a[4]) for a in said(sand.calls[calls:], "read")] == [
    (root, "/w/a.txt"),
    (two, "/w/b.txt"),
    (two, "/w/a.txt"),
  ]
  assert caught(engine.turns(on=root), root) == ["#chain1 caught up\nchain1_changed = {'/w/a.txt': {2: 'three'}}"]
  assert caught(engine.turns(on=two), two) == [
    f"#{two} caught up\n{two}_changed = {{'/w/b.txt': {{1: 'c'}}, '/w/a.txt': {{2: 'three'}}}}"
  ]
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#read")] == []
  watchers = [a[1] for a in said(log, "catchup")]
  assert [a for a in said(log, "tell") if a[1] in (root, two) and "caught up" in a[3][0]] == [
    ("tell", root, watchers[0], caught(engine.turns(on=root), root)),
    ("tell", two, watchers[1], caught(engine.turns(on=two), two)),
  ]
  assert [engine.peek(one, "open") for one in watchers] == ["open", "open"]


async def test_the_paragraph_binds_changed_to_the_paths_that_changed() -> None:
  """The paragraph binds changed to the paths that changed, the most recent first, each to the lines that differ by their numbers, with None for a line past the new end, or to their count when they are more than CAP, or to gone when the read is refused."""
  many = "".join(f"{i}\n" for i in range(CAP + 10))
  sand, _, root = await caught_up(
    files={"/w/a.txt": "one\ntwo\nthree\n", "/w/b.txt": many, "/w/c.txt": "c\n", "/w/d.txt": "d\n"}
  )
  for path in ("a.txt", "b.txt", "c.txt", "d.txt"):
    engine.read(path, on=root)
  sand.files.update({"/w/a.txt": "one\nTWO\n", "/w/b.txt": many.replace("\n", "!\n")})
  del sand.files["/w/c.txt"]
  engine.stand()
  assert caught(engine.turns(on=root), root) == [
    "#chain1 caught up\nchain1_changed = {'/w/c.txt': 'gone', '/w/b.txt': 50, '/w/a.txt': {2: 'TWO', 3: None}}"
  ]
  engine.stand()
  assert len(caught(engine.turns(on=root), root)) == 1


async def test_the_paragraph_binds_hours_to_the_hours_between_the_last_two_readings_of_the_clock_of_the_root() -> None:
  """The paragraph binds hours to the hours between the last two readings of the clock of the root, to one decimal place, only when they are more than one, so it says the time since the life before."""
  sand, _, root = await caught_up()
  two = await chained("two")
  await settle()
  _, _ = await catching(Sand(tick=3600), plain(sand.record))
  assert caught(engine.turns(on=root), root) == [] and caught(engine.turns(on=two), two) == []
  _, _ = await catching(Sand(tick=7380), plain(sand.record))
  assert engine.peek("clock2") == 8381.0
  assert caught(engine.turns(on=root), root) == ["#chain1 caught up\nchain1_hours = 2.0"]
  assert caught(engine.turns(on=two), two) == [f"#{two} caught up\n{two}_hours = 2.0"]


async def test_past_budget_bytes_the_paragraph_gives_the_paths_alone() -> None:
  """Past BUDGET bytes the paragraph gives the paths alone, the most recent first, as many as fit."""
  long = "".join(f"{'x' * 100}\n" for _ in range(CAP))
  few = [f"/w/{i}.txt" for i in range(3)]
  many = [f"/w/{'p' * 90}{i:02}.txt" for i in range(60)]
  sand, _, root = await caught_up(files=dict.fromkeys([*few, *many], "a\n") | dict.fromkeys(few, long))
  for path in few:
    engine.read(path, on=root)
  sand.files.update(dict.fromkeys(few, long.replace("x", "y")))
  engine.stand()
  assert caught(engine.turns(on=root), root) == [f"#chain1 caught up\nchain1_changed = {few[::-1]!r}"]
  for path in many:
    engine.read(path, on=root)
  sand.files.update(dict.fromkeys(many, "b\n"))
  engine.stand()
  head = "#chain1 caught up\nchain1_changed = "
  fit = max(n for n in range(len(many) + 1) if len(f"{head}{many[::-1][:n]!r}".encode()) <= BUDGET)
  assert 0 < fit < len(many)
  assert caught(engine.turns(on=root), root)[-1] == f"{head}{many[::-1][:fit]!r}"


async def test_a_chain_where_nothing_changed_tells_nothing_at_a_stand() -> None:
  """A chain where nothing changed tells nothing at a stand, so its catch-up costs no token."""
  sand, _, root = await caught_up()
  engine.read("a.txt", on=root)
  two = await chained("two")
  engine.read("a.txt", on=two)
  was = [engine.turns(on=one) for one in (root, two)]
  engine.stand()
  assert [engine.turns(on=one) for one in (root, two)] == was
  await catching(sown(), plain(sand.record))
  assert [engine.turns(on=one) for one in (root, two)] == was


async def test_the_journal_keeps_each_read_and_each_clock_of_a_catch_up() -> None:
  """The journal keeps each read and each clock of a catch-up, so a later life on the same record tells the same catch-up at the same place."""
  sand, _, root = await caught_up()
  engine.read("a.txt", on=root)
  second = Sand(files={"/w/a.txt": "one\nTWO\n"}, tick=7200)
  await catching(second, plain(sand.record))
  told = caught(engine.turns(on=root), root)
  assert told == ["#chain1 caught up\nchain1_hours = 2.0\nchain1_changed = {'/w/a.txt': {2: 'TWO'}}"]
  kept = [e[0][:2] for e in second.record]
  assert ("read", "read2") in kept and ("done", "read2") in kept and ("clock", "clock2") in kept
  third = Sand(files={"/w/a.txt": "ONE\nTWO\n"}, tick=7200)
  await catching(third, plain(sand.record + second.record))
  assert caught(engine.turns(on=root), root) == [*told, "#chain1 caught up\nchain1_changed = {'/w/a.txt': {1: 'ONE'}}"]
  assert [a[0] for a in third.calls] == ["stand", "clock", "read"]


async def test_it_hears_no_control() -> None:
  """It hears no control, as the chain that it serves hears none, so a pause of the chain holds no catch-up."""
  sand, log, root = await caught_up()
  engine.read("a.txt", on=root)
  (watcher,) = [a[1] for a in said(log, "catchup") if a[3] == root]
  engine.pause(root)
  engine.cancel(watcher)
  sand.files["/w/a.txt"] = "one\nTWO\n"
  engine.stand()
  assert caught(engine.turns(on=root), root) == ["#chain1 caught up\nchain1_changed = {'/w/a.txt': {2: 'TWO'}}"]
  assert engine.peek(watcher, "open") == "open"
