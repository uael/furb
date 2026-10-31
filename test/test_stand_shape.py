"""Stand, what a chain stands on."""

import pytest

from conftest import (
  LATER,
  STANDS,
  WORLD,
  Sand,
  World,
  born,
  chained,
  life,
  paragraphs,
  plain,
  relived,
  said,
  settle,
  stood,
  tip,
)
from furb import engine
from furb.engine import OPERATOR, Refused


def restanding(*chains: str) -> World:
  """An ear that plays the turn stand() of the actor of the standing on each chain that it is given, at the done of
  each stand that the operator made and that the World answered in this life, as the World of a later life does on
  each chain that lives."""
  while True:
    a = yield None
    if a and a[0] == "done" and engine.question(("stand", a[1])) and a[2] == WORLD and engine.get(a[1])[2] == OPERATOR:
      for one in chains:
        engine.rung("stand()", to=str(engine.standing()[2]), on=one)


async def restood(sand: Sand, record: list, *chains: str) -> tuple[list[tuple], str]:
  """A later life on a kept record, with the World that it is given and an ear that plays stand() on each chain."""
  log, root = life(sand, record, restand=restanding(*chains))
  await settle(300)
  return log, root


async def test_a_stand_is_the_question_of_what_the_chains_stand_on() -> None:
  """A stand is the question of what the chains stand on, which boot asks the World on the root at the tip of every life, and whose answer binds the actor of every chain from its place on."""
  sand, _, root = born()
  two = engine.chain("two")
  side = engine.chain("side", source=root)
  await settle(300)
  assert [(a[1], a[3]) for a in said(sand.calls, "stand")] == [("stand1", root)]
  for one in (root, two, side):
    assert engine.module(one)["actor"] == "m/low"
  sand.stands = LATER
  engine.stand(engine.ROOT)
  for one in (root, two, side):
    assert engine.module(one)["actor"] == "o/low"


async def test_a_change_of_the_world_between_two_lives_enters_the_transcript_of_a_chain() -> None:
  """A change of the World between two lives enters the transcript of a chain."""
  sand, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  was = paragraphs(engine.turns(on=root))
  again, over = await restood(Sand(stands=LATER), plain(sand.record), root)
  asked = said(again, "stand")[-1][1]
  assert over == root and asked == "stand3"
  assert paragraphs(engine.turns(on=over))[: len(was)] == was
  assert paragraphs(engine.turns(on=over))[-1] == stood(asked, LATER)
  assert engine.module(over)["actor"] == "o/low"


async def test_the_world_answers_a_stand_with_the_roster_the_directory_and_the_actor() -> None:
  """The World answers a stand with the roster, the directory, the actor, the path of the doctrine and the extensions."""
  doctrine = "/w/.furb/DOCTRINE.md"
  sand, log, _ = born(stands=[*STANDS, doctrine, []], files={doctrine: "Look first.\n"})
  asked = said(log, "stand")[0]
  answered = next(a for a in said(log, "done") if a[1] == asked[1])
  assert answered[2] == WORLD and answered[3] == [*STANDS, doctrine, []]
  assert answered[3] == [STANDS[0], "/w", "m/low", doctrine, []]
  assert said(sand.calls, "stand") == [asked, ("stand", "stand2", "rung2", "chain1")]


async def test_a_model_asked_on_any_chain_of_a_later_life_finds_the_new_roster() -> None:
  """A model asked on any chain of a later life finds the new roster in the transcript of its chain, in the stand that its chain plays there."""
  sand, _, root = born()
  two = await chained("two")
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  later = Sand(stands=LATER)
  again, over = await restood(later, plain(sand.record), root, two)
  assert [(a[1], a[3]) for a in said(again, "stand")] == [
    ("stand1", over),
    ("stand2", over),
    ("stand3", over),
    ("stand4", two),
  ]
  for one in (over, two):
    later.script[one] = ["close(1)"]
    assert await engine.prompt(int, "count", to="o/low", on=one) == 1
    asked = later.turns[said(engine.transcript(one), "reply")[-1][1]]
    assert stood("stand3" if one == over else "stand4", LATER) in paragraphs(asked)


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
    assert engine.module(one)["actor"] == "o/low"
  twin = await chained("twin", root, 300)
  assert engine.module(twin)["actor"] == "o/low"


async def test_a_standing_binds_the_default_actor_of_the_chain_when_its_actor_differs() -> None:
  """A standing binds the default actor of the chain, under the name actor, when its actor differs from that of the standing before it, so a new roster or a new directory alone keeps the actor that a word chose."""
  sand, log, root = born("actor = 'n/low'\nclose(1)", "close(None)")
  assert await engine.prompt(int, "count", on=root) == 1
  assert said(log, "reply")[0][4] == "m/low" and engine.module(root)["actor"] == "n/low"
  sand.stands = [STANDS[0], "/v", STANDS[2]]
  engine.stand(engine.ROOT)
  assert engine.module(root)["actor"] == "n/low"
  sand.stands = [[*STANDS[0], ["q", ["low"], 300000]], "/w", STANDS[2]]
  engine.stand(engine.ROOT)
  assert engine.module(root)["actor"] == "n/low"
  sand.stands = LATER
  engine.stand(engine.ROOT)
  assert engine.module(root)["actor"] == "o/low"


async def test_every_chain_hears_the_done_of_every_stand() -> None:
  """Every chain hears the done of every stand, and binds its default actor again when the actor of the standing changes, and a chain whose standing names a doctrine plays on itself the rung of stand(), with a cd to the new directory when the directory changed and no cd of a rung with no actor moved the chain, so the transcript grows at one end."""
  first, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  was = engine.turns(on=root)
  later = Sand(stands=LATER)
  _, over = await relived(later, plain(first.record))
  assert engine.turns(on=over) == was and engine.module(root)["actor"] == "o/low"
  later.stands = Refused("no standing now")
  with pytest.raises(Refused):
    engine.stand(engine.ROOT)
  assert engine.standing() == LATER and engine.module(root)["actor"] == "o/low"
  two = await chained("two")
  told = [paragraphs(engine.turns(on=one)) for one in (over, two)]
  later.stands = STANDS
  engine.stand(engine.ROOT)
  assert [paragraphs(engine.turns(on=one)) for one in (over, two)] == told
  assert engine.module(root)["actor"] == engine.module(two)["actor"] == "m/low"
  await engine.rung("stand()", on=two)
  assert paragraphs(engine.turns(on=two)) == [*told[1], "stand()", stood("stand5")]
  assert paragraphs(engine.turns(on=over)) == told[0]
  taught, _, root = born(stands=[*STANDS, "/w/DOCTRINE.md", []], files={"/w/DOCTRINE.md": "Read first.\n"})
  await engine.rung("k = 1", on=root)
  await settle()
  last = lambda: [one[1] for one in engine.turns(on=root) if one[0] == "assistant"][-1]  # noqa: E731
  assert last() == "stand()\ncd('/w')"
  taught.stands = [STANDS[0], "/z", STANDS[2], "/w/DOCTRINE.md", []]
  engine.stand(engine.ROOT)
  await settle()
  assert last() == "stand()\ncd('/z')"
  await engine.rung("cd('/y')", on=root)
  taught.stands = [STANDS[0], "/v", STANDS[2], "/w/DOCTRINE.md", []]
  engine.stand(engine.ROOT)
  await settle()
  assert last() == "stand()" and engine.cwd(on=root) == "/y"
