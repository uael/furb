"""Stand, what a chain stands on."""

from conftest import STANDS, WORLD, Sand, life, paragraphs, plain, relived, rows, said, settle, sown, takes, tip
from furb import engine
from furb.engine import OPERATOR

LATER = [[["operator", [], 200000], ["o", ["low"], 200000]], "/z", "o/low"]
"""What a later World offers: another roster, another directory and another default actor."""


async def test_a_stand_is_the_question_of_what_the_chains_stand_on() -> None:
  """A stand is the question of what the chains stand on, which boot asks the World on the root at the tip of every life, and whose answer binds the actor and the directory of every chain from its place on."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
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
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  assert paragraphs(engine.turns(on=root))[1] == takes(root)
  _, over = await relived(Sand(stands=LATER), plain(sand.record))
  standings = [one for one in paragraphs(engine.turns(on=over)) if one.startswith(f"#{over} roster ")]
  assert over == root and standings == [takes(over), takes(over, LATER)]
  held = engine.transcript(over)
  assert [a for a in said(held, "tell") if a[3][0].startswith(f"#{over} roster ")][-1] == (
    "tell",
    over,
    over,
    rows(over, LATER),
  )
  assert engine.cwd(on=over) == "/z" and engine.module(over)["actor"] == "o/low"


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
  asked = later.turns[said(again, "reply")[-1][1]]
  assert [one for one in paragraphs(asked) if one.startswith(f"#{over} roster ")] == [takes(over), takes(over, LATER)]


async def test_the_world_answers_it_with_a_done_at_once() -> None:
  """The World answers it with a done at once, so the life stands on that answer before boot returns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  asked = said(log, "stand")[0]
  assert asked[1] == "stand1" and engine.peek(asked[1]) == STANDS == engine.standing()
  assert [a[1] for a in said(log, "started") if a[1] == asked[1]] == []
  assert engine.module(root)["actor"] == "m/low"


async def test_the_record_keeps_each_stand_and_its_answer() -> None:
  """The record keeps each stand and its answer, so a later life says them again at their places and replays every chain on what it stood on there."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert [a[3] for a in said(sand.calls, "stand")] == [root]
  first = said(log, "stand")[0]
  assert [e[0] for e in sand.record if e[0][1] == first[1]] == [first, ("done", first[1], "world", STANDS)]
  later = Sand(stands=LATER)
  again, _ = await relived(later, plain(sand.record))
  opened = [a for a in said(again, "done") if a[1] == said(log, "stand")[0][1]]
  assert [a[2] for a in opened] == ["record"] and [a[1] for a in said(later.calls, "stand")] == ["stand2"]
  assert [a[6] for a in said(again, "rung") if not a[4]] == ["m/low"] and said(later.calls, "reply") == []


async def test_at_its_tip_boot_stands_the_life_again() -> None:
  """At its tip, once the record is said again whole, boot stands the life again, so a change of the World reaches every chain after what it replayed."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
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
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert engine.module(twin)["actor"] == "o/low"


async def test_each_standing_binds_the_default_actor_of_the_chain_under_the_name_actor() -> None:
  """Each standing binds the default actor of the chain, under the name actor."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert engine.module(root)["actor"] == "m/low" == said(log, "reply")[0][4]


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


async def test_the_standing_a_chain_tells_is_what_its_transcript_holds_of_it() -> None:
  """The standing a chain tells is what its transcript holds of it, and the stand itself tells nothing."""
  sand = sown()
  log, root = life(sand)
  held = engine.transcript(root)
  stand = said(log, "stand")[0][1]
  assert [a for a in held if a[0] == "tell" and a[1] == stand] == []
  assert ("tell", root, root, rows(root)) in held


async def test_every_chain_hears_the_done_of_every_stand() -> None:
  """Every chain hears the done of every stand, and a chain whose standing that answer changes binds its default actor and tells it there, so the transcript grows at one end."""
  first = Sand(stands=STANDS)
  _, root = life(first)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  was = engine.turns(on=root)
  _, over = await relived(Sand(stands=LATER), plain(first.record))
  now = engine.turns(on=over)
  assert now[0][1][: len(was[0][1])] == was[0][1]
  assert (
    paragraphs(now)[-1]
    == takes(root, LATER)
    == "\n".join(
      ["#chain1 roster [['operator', [], 200000], ['o', ['low'], 200000]]", "#chain1 cwd /z", "#chain1 actor o/low"]
    )
  )
  assert engine.module(root)["actor"] == "o/low"
