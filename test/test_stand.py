"""Stand, what a chain stands on."""

from conftest import STANDS, Sand, life, paragraphs, plain, relived, rows, said, settle, sown, stood, where
from furb import engine
from furb.engine import WORLD

LATER = [[["operator", [], 200000], ["o", ["low"], 200000]], "/z", "o/low"]
"""What a later World offers: another roster, another directory and another default actor."""


async def test_what_a_chain_stands_on() -> None:
  """What a chain stands on, which every chain without a source asks the World for as it opens, and a chain with a source asks of its origin, and which binds the actor and the directory of that chain from then on."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  assert [a[3] for a in said(sand.calls, "stand")] == [root, two]
  assert engine.modules[root]["actor"] == "m/low" and where(root) == "/w"
  assert engine.modules[two]["actor"] == "m/low" and where(two) == "/w"
  side = engine.chain("side", source=root)
  await settle(300)
  assert [a[3] for a in said(sand.calls, "stand")] == [root, two]
  assert engine.modules[side]["actor"] == "m/low" and where(side) == "/w"


async def test_a_change_of_the_world_between_two_lives_enters_the_transcript_of_a_chain() -> None:
  """A change of the World between two lives enters the transcript of a chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  assert paragraphs(engine.turns(on=root))[1] == stood(root)
  _, over = await relived(Sand(stands=LATER), plain(sand.record))
  standings = [one for one in paragraphs(engine.turns(on=over)) if one.startswith(f"#{over} roster ")]
  assert over == root and standings == [stood(over), stood(over, LATER)]
  _, held = engine.ask("transcript", over, over)
  assert isinstance(held, list)
  assert [a for a in held if a[0] == "tell" and a[3][0].startswith(f"#{over} roster ")][-1] == (
    "tell",
    over,
    over,
    rows(over, LATER),
  )
  assert where(over) == "/z" and engine.modules[over]["actor"] == "o/low"


async def test_the_world_answers_a_stand_with_the_roster_the_directory_and_the_actor() -> None:
  """The World answers a stand with the roster, the directory and the actor."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  asked = said(log, "stand")[0]
  answered = next(a for a in said(log, "done") if a[1] == asked[1])
  assert answered[2] == WORLD and answered[3] == STANDS
  assert answered[3] == [STANDS[0], "/w", "m/low"]
  assert said(sand.calls, "stand") == [asked]


async def test_a_model_asked_on_any_chain_of_a_later_life_finds_the_new_roster() -> None:
  """A model asked on any chain of a later life finds the new roster in the transcript of its chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  later = Sand(stands=LATER)
  again, over = await relived(later, plain(sand.record))
  later.script[over] = ["close(1)"]
  assert await engine.prompt(int, "count", to="o/low", on=over) == 1
  asked = said(again, "ask")[-1]
  assert [one for one in paragraphs(asked[5]) if one.startswith(f"#{over} roster ")] == [
    stood(over),
    stood(over, LATER),
  ]


async def test_the_world_answers_it_while_the_chain_waits() -> None:
  """The World answers it while the chain waits, since what a chain stands on is asked and never done."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  asked = said(log, "stand")[0]
  assert asked[1] == f"stand@{root}.1" == "stand@chain1.1"
  assert asked[1] in engine.asked and asked[1] not in engine.acts
  assert engine.modules[root]["actor"] == "m/low"


async def test_a_chain_asks_what_it_stands_on_at_its_open_and_the_journal_keeps_that_stand() -> None:
  """A chain asks what it stands on at its open, and the journal keeps that stand with its answer beside, so a later life opens the chain on what it stood on and replays it there."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert [a[3] for a in said(sand.calls, "stand")] == [root]
  assert [e for e in sand.record if e[0][0] == "stand"] == [(said(log, "stand")[0], STANDS)]
  later = Sand(stands=LATER)
  again, _ = await relived(later, plain(sand.record))
  opened = [a for a in said(again, "done") if a[1] == said(log, "stand")[0][1]]
  assert [a[2] for a in opened] == ["record"] and [a[3] for a in said(later.calls, "stand")] == [""]
  assert [a[6] for a in said(again, "rung") if not a[4]] == ["m/low"] and said(again, "ask") == []


async def test_at_its_tip_the_journal_asks_the_world_what_it_stands_on() -> None:
  """At its tip, once the record is said again whole, the journal asks the World what it stands on, and says it as a stood to every chain that stands on something else, so a change of the World reaches every chain after what it replayed."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  side = engine.chain("side", source=root)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  later = Sand(stands=LATER)
  again, _ = await relived(later, plain(sand.record))
  assert [a[1] for a in said(again, "stood")] == [root, two, side]
  assert [e[0][:2] for e in later.record] == [("stood", root), ("stood", two), ("stood", side)]
  for one in (root, two, side):
    assert where(one) == "/z" and engine.modules[one]["actor"] == "o/low"
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert engine.modules[twin]["actor"] == "o/low"
  same = Sand(stands=LATER)
  third, _ = await relived(same, [*plain(sand.record), *plain(later.record)])
  assert {a[2] for a in said(third, "stood")} == {"record"} and same.record == []


async def test_each_standing_binds_the_default_actor_of_the_chain_under_the_name_actor() -> None:
  """Each standing binds the default actor of the chain, under the name actor."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert engine.modules[root]["actor"] == "m/low" == said(log, "ask")[0][4]


async def test_the_chain_tells_each_standing_it_takes_in_one_paragraph_of_three_headers() -> None:
  """The chain tells each standing it takes in one paragraph of three headers, one for each part: the roster under the header roster as python shows it, then the directory under the header cwd and the actor under the header actor, each as it is."""
  sand = sown()
  log, root = life(sand)
  answered = next(a[3] for a in said(log, "done") if a[1] == said(log, "stand")[0][1])
  assert answered == STANDS
  assert paragraphs(engine.turns(on=root))[1] == (
    "#chain1 roster [['operator', [], 200000], ['m', ['low', 'high'], 400000], ['n', ['low'], 200000]]\n"
    "#chain1 cwd /w\n"
    "#chain1 actor m/low"
  )
  await relived(Sand(stands=LATER), plain(sand.record))
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{root} roster ")][-1] == (
    "#chain1 roster [['operator', [], 200000], ['o', ['low'], 200000]]\n#chain1 cwd /z\n#chain1 actor o/low"
  )


async def test_the_chain_holds_no_stand() -> None:
  """The chain holds no stand, since the standing it tells is what its transcript holds of it."""
  sand = sown()
  log, root = life(sand)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  stood = said(log, "stand")[0][1]
  assert stood == f"stand@{root}.1"
  assert [a for a in held if a[0] == "stand" or a[1] == stood] == []
  assert held[1] == ("tell", root, root, rows(root))
