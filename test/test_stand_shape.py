"""Stand, what a chain stands on."""

import pytest

from conftest import (
  LATER,
  STANDS,
  WORLD,
  Sand,
  born,
  chained,
  paragraphs,
  plain,
  relived,
  said,
  settle,
  sown,
  takes,
  tip,
)
from furb import engine
from furb.engine import BUDGET, CAP, OPERATOR, Refused, Text


async def test_a_stand_is_the_question_of_what_the_chains_stand_on() -> None:
  """A stand is the question of what the chains stand on, which boot asks the World on the root at the tip of every life, and whose answer binds the actor and the directory of every chain from its place on."""
  sand, _, root = born()
  two = engine.chain("two")
  side = engine.chain("side", source=root)
  await settle(300)
  assert [(a[1], a[3]) for a in said(sand.calls, "stand")] == [("stand1", root)]
  for one in (root, two, side):
    assert engine.module(one)["actor"] == "m/low" and engine.cwd(on=one) == "/w"
  sand.stands = LATER
  engine.stand()
  for one in (root, two, side):
    assert engine.module(one)["actor"] == "o/low" and engine.cwd(on=one) == "/z"


async def test_a_change_of_the_world_between_two_lives_enters_the_transcript_of_a_chain() -> None:
  """A change of the World between two lives enters the transcript of a chain."""
  sand, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  assert paragraphs(engine.turns(on=root))[1] == takes(root)
  _, over = await relived(Sand(stands=LATER), plain(sand.record))
  standings = [one for one in paragraphs(engine.turns(on=over)) if one.startswith(f"#{over} standing")]
  assert over == root and standings == [takes(over), takes(over, LATER)]
  held = engine.transcript(over)
  assert [a for a in said(held, "tell") if a[3][0].startswith(f"#{over} standing")][-1] == (
    "tell",
    over,
    over,
    [takes(over, LATER)],
  )
  assert engine.cwd(on=over) == "/z" and engine.module(over)["actor"] == "o/low"


async def test_the_world_answers_a_stand_with_the_roster_the_directory_and_the_actor() -> None:
  """The World answers a stand with the roster, the directory and the actor."""
  sand, log, _ = born()
  asked = said(log, "stand")[0]
  answered = next(a for a in said(log, "done") if a[1] == asked[1])
  assert answered[2] == WORLD and answered[3] == STANDS
  assert answered[3] == [STANDS[0], "/w", "m/low"]
  assert said(sand.calls, "stand") == [asked]


async def test_a_model_asked_on_any_chain_of_a_later_life_finds_the_new_roster() -> None:
  """A model asked on any chain of a later life finds the new roster in the transcript of its chain."""
  sand, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  later = Sand(stands=LATER)
  again, over = await relived(later, plain(sand.record))
  later.script[over] = ["close(1)"]
  assert await engine.prompt(int, "count", to="o/low", on=over) == 1
  asked = later.turns[said(again, "reply")[-1][1]]
  assert [one for one in paragraphs(asked) if one.startswith(f"#{over} standing")] == [takes(over), takes(over, LATER)]


async def test_the_world_answers_it_with_a_done_at_once() -> None:
  """The World answers it with a done at once, so the life stands on that answer before boot returns."""
  _, log, root = born()
  asked = said(log, "stand")[0]
  assert asked[1] == "stand1" and engine.peek(asked[1]) == STANDS == engine.standing()
  assert [a[1] for a in said(log, "started") if a[1] == asked[1]] == []
  assert engine.module(root)["actor"] == "m/low"


async def test_the_journal_keeps_each_stand_and_its_answer() -> None:
  """The journal keeps each stand and its answer, so a later life says them again at their places and replays every chain on what it stood on there."""
  sand, log, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert [a[3] for a in said(sand.calls, "stand")] == [root]
  first = said(log, "stand")[0]
  assert [e[0] for e in sand.record if e[0][1] == first[1]] == [first, ("done", first[1], "world", STANDS)]
  later = Sand(stands=LATER)
  again, _ = await relived(later, plain(sand.record))
  opened = [a for a in said(again, "done") if a[1] == said(log, "stand")[0][1]]
  assert [a[2] for a in opened] == ["journal"] and [a[1] for a in said(later.calls, "stand")] == ["stand2"]
  assert [a[6] for a in said(again, "rung") if not a[4]] == ["m/low"] and said(later.calls, "reply") == []


async def test_at_its_tip_boot_stands_the_life_again() -> None:
  """At its tip, once the record is said again whole, boot stands the life again, so a change of the World reaches every chain after what it replayed."""
  sand, _, root = born()
  two = engine.chain("two")
  side = engine.chain("side", source=root)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  later = Sand(stands=LATER)
  again, _ = await relived(later, plain(sand.record))
  assert [(a[1], a[2]) for a in said(again, "stand")] == [("stand1", OPERATOR), ("stand2", OPERATOR)]
  assert later.record == tip("stand2", root, LATER)
  for one in (root, two, side):
    assert engine.cwd(on=one) == "/z" and engine.module(one)["actor"] == "o/low"
  twin = await chained("twin", root, 300)
  assert engine.module(twin)["actor"] == "o/low"


async def test_a_standing_binds_the_default_actor_of_the_chain_when_its_actor_differs() -> None:
  """A standing binds the default actor of the chain, under the name actor, when its actor differs from that of the standing before it, so a new roster or a new directory alone keeps the actor that a word chose."""
  sand, log, root = born("actor = 'n/low'\nclose(1)", "close(None)")
  assert await engine.prompt(int, "count", on=root) == 1
  assert said(log, "reply")[0][4] == "m/low" and engine.module(root)["actor"] == "n/low"
  sand.stands = [STANDS[0], "/v", STANDS[2]]
  engine.stand(on=root)
  assert engine.module(root)["actor"] == "n/low"
  assert paragraphs(engine.turns(on=root))[-1] == takes(root, [STANDS[0], "/v", "n/low"])
  sand.stands = LATER
  engine.stand(on=root)
  assert engine.module(root)["actor"] == "o/low"


async def test_the_chain_tells_each_standing_it_takes_in_one_paragraph_headed_standing() -> None:
  """The chain tells each standing it takes in one paragraph headed standing, which binds its roster, its directory and its actor, as chain1_roster, chain1_cwd and chain1_actor, and its roster holds no doctrine, since a chain tells the doctrine of an actor only where it asks that actor."""
  sand, log, root = born()
  answered = next(a[3] for a in said(log, "done") if a[1] == said(log, "stand")[0][1])
  assert answered == STANDS
  assert paragraphs(engine.turns(on=root))[1] == (
    "#chain1 standing\n"
    "chain1_roster = [['operator', [], 200000], ['m', ['low', 'high'], 400000], ['n', ['low'], 200000]]\n"
    "chain1_cwd = '/w'\n"
    "chain1_actor = 'm/low'"
  )
  sand.script[root] = ["close(chain1_cwd)"]
  assert await engine.prompt(str, "where", on=root) == "/w"
  assert [engine.module(root)[f"chain1_{word}"] for word in ("roster", "cwd", "actor")] == STANDS
  await relived(Sand(stands=LATER), plain(sand.record))
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{root} standing")][-1] == (
    "#chain1 standing\nchain1_roster = [['operator', [], 200000], ['o', ['low'], 200000]]\n"
    "chain1_cwd = '/z'\nchain1_actor = 'o/low'"
  )
  _, _, root = born(stands=[[*STANDS[0], ["q", ["low"], 300000, "Look first."]], "/w", "q/low"])
  assert paragraphs(engine.turns(on=root))[1] == (
    "#chain1 standing\n"
    "chain1_roster = [['operator', [], 200000], ['m', ['low', 'high'], 400000], ['n', ['low'], 200000], ['q', ['low'], 300000]]\n"
    "chain1_cwd = '/w'\n"
    "chain1_actor = 'q/low'"
  )


async def test_the_standing_a_chain_tells_is_what_its_transcript_holds_of_it() -> None:
  """The standing a chain tells is what its transcript holds of it, and the stand itself tells nothing."""
  _, log, root = born()
  held = engine.transcript(root)
  stand = said(log, "stand")[0][1]
  assert [a for a in held if a[0] == "tell" and a[1] == stand] == []
  assert ("tell", root, root, [takes(root)]) in held


async def test_every_chain_hears_the_done_of_every_stand() -> None:
  """Every chain hears the done of every stand, and a chain whose standing that answer changes tells it there, and binds its default actor again when the actor of the standing changes, so the transcript grows at one end."""
  first, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  was = engine.turns(on=root)
  later = Sand(stands=LATER)
  _, over = await relived(later, plain(first.record))
  now = engine.turns(on=over)
  assert now[0][1][: len(was[0][1])] == was[0][1]
  assert (
    paragraphs(now)[-1]
    == takes(root, LATER)
    == "\n".join(
      [
        "#chain1 standing",
        "chain1_roster = [['operator', [], 200000], ['o', ['low'], 200000]]",
        "chain1_cwd = '/z'",
        "chain1_actor = 'o/low'",
      ]
    )
  )
  assert engine.module(root)["actor"] == "o/low"
  later.stands = Refused("no standing now")
  with pytest.raises(Refused):
    engine.stand(on=over)
  assert (
    engine.standing() == LATER
    and engine.cwd(on=over) == "/z"
    and paragraphs(engine.turns(on=over))[-1] == takes(root, LATER)
  )
  two = await chained("two")
  assert paragraphs(engine.turns(on=two))[-1] == takes(two, LATER)
  later.stands = STANDS
  engine.stand(on=over)
  assert paragraphs(engine.turns(on=over))[-1] == takes(root) and engine.module(root)["actor"] == "m/low"
  assert paragraphs(engine.turns(on=two))[-1] == takes(two) and engine.module(two)["actor"] == "m/low"
  before = [paragraphs(engine.turns(on=one)) for one in (over, two)]
  engine.stand(on=over)
  assert [paragraphs(engine.turns(on=one)) for one in (over, two)] == before


def caught(got: list, id: str) -> list[str]:
  """Each paragraph headed caught up that the turns of a chain hold, in order."""
  return [one for one in paragraphs(got) if one.startswith(f"#{id} caught up")]


async def test_at_each_stand_the_root_reads_the_clock() -> None:
  """At each stand the root reads the clock once it told the standing that the stand gives it, so the record holds a reading of the clock at the tip of every life."""
  sand, log, root = born()
  two = await chained("two")
  assert [a[:4] for a in said(log, "clock")] == [("clock", "clock1", root, root)]
  held = engine.transcript(root)
  told = held.index(("tell", root, root, [takes(root)]))
  assert [a[0] for a in held[told + 1 :]][:2] == ["clock", "done"] and engine.peek("clock1") == 1001.0
  later = Sand()
  again, _ = await relived(later, plain(sand.record))
  assert [a[1:4] for a in said(again, "clock")] == [("clock1", root, root), ("clock2", root, root)]
  assert later.record == tip("stand2") and [a[0] for a in later.calls] == ["stand", "clock"]
  assert [a for a in engine.transcript(two) if a[0] == "clock"] == []


async def test_at_each_stand_every_chain_catches_up() -> None:
  """At each stand every chain catches up: it reads again, with a hidden show, the path of each text that the last read or write of that path on the chain gave, but a path of a scheme and a door, and it tells what changed in one paragraph headed caught up."""
  sand, _, root = born(files={"/w/a.txt": "one\ntwo\n", "x://y": "y\n"})
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


async def test_the_paragraph_binds_changed_to_the_paths_that_changed() -> None:
  """The paragraph binds changed to the paths that changed, the most recent first, each to the lines that differ by their numbers, with None for a line past the new end, or to their count when they are more than CAP, or to gone when the read is refused."""
  many = "".join(f"{i}\n" for i in range(CAP + 10))
  sand, _, root = born(files={"/w/a.txt": "one\ntwo\nthree\n", "/w/b.txt": many, "/w/c.txt": "c\n", "/w/d.txt": "d\n"})
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
  sand, _, root = born()
  two = await chained("two")
  await settle()
  _, _ = await relived(Sand(tick=3600), plain(sand.record))
  assert caught(engine.turns(on=root), root) == [] and caught(engine.turns(on=two), two) == []
  _, _ = await relived(Sand(tick=7380), plain(sand.record))
  assert engine.peek("clock2") == 8381.0
  assert caught(engine.turns(on=root), root) == ["#chain1 caught up\nchain1_hours = 2.0"]
  assert caught(engine.turns(on=two), two) == [f"#{two} caught up\n{two}_hours = 2.0"]


async def test_past_budget_bytes_the_paragraph_gives_the_paths_alone() -> None:
  """Past BUDGET bytes the paragraph gives the paths alone, the most recent first, as many as fit."""
  long = "".join(f"{'x' * 100}\n" for _ in range(CAP))
  few = [f"/w/{i}.txt" for i in range(3)]
  many = [f"/w/{'p' * 90}{i:02}.txt" for i in range(60)]
  sand, _, root = born(files=dict.fromkeys([*few, *many], "a\n") | dict.fromkeys(few, long))
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
  sand, _, root = born()
  engine.read("a.txt", on=root)
  two = await chained("two")
  engine.read("a.txt", on=two)
  was = [engine.turns(on=one) for one in (root, two)]
  engine.stand()
  assert [engine.turns(on=one) for one in (root, two)] == was
  await relived(sown(), plain(sand.record))
  assert [engine.turns(on=one) for one in (root, two)] == was


async def test_the_journal_keeps_each_read_and_each_clock_of_a_catch_up() -> None:
  """The journal keeps each read and each clock of a catch-up, so a later life on the same record tells the same catch-up at the same place."""
  sand, _, root = born()
  engine.read("a.txt", on=root)
  second = Sand(files={"/w/a.txt": "one\nTWO\n"}, tick=7200)
  await relived(second, plain(sand.record))
  told = caught(engine.turns(on=root), root)
  assert told == ["#chain1 caught up\nchain1_hours = 2.0\nchain1_changed = {'/w/a.txt': {2: 'TWO'}}"]
  kept = [e[0][:2] for e in second.record]
  assert ("read", "read2") in kept and ("done", "read2") in kept and ("clock", "clock2") in kept
  third = Sand(files={"/w/a.txt": "ONE\nTWO\n"}, tick=7200)
  await relived(third, plain(sand.record + second.record))
  assert caught(engine.turns(on=root), root) == [*told, "#chain1 caught up\nchain1_changed = {'/w/a.txt': {1: 'ONE'}}"]
  assert [a[0] for a in third.calls] == ["stand", "clock", "read"]
