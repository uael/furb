"""Stand, what a chain stands on."""

import pytest

from conftest import STANDS, Sand, attr, life, plain, relived, said, settle, sown, tags
from furb import engine
from furb.engine import WORLD

LATER = ((("operator", (), 200000), ("o", ("low",), 200000)), "/z", "o/low")
"""What a later World offers: another roster, another directory and another default actor."""


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_what_a_chain_stands_on() -> None:
  """What a chain stands on, which every chain without a source asks the World for as it opens, and a chain with a source asks of its origin, and which binds the actor and the directory of that chain from then on."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  assert [a[3] for a in said(sand.calls, "stand")] == [root, two]
  assert engine.modules[root]["actor"] == "m/low" and engine.cwd(on=root) == "/w"
  assert engine.modules[two]["actor"] == "m/low" and engine.cwd(on=two) == "/w"
  side = engine.chain("side", source=root)
  await settle(300)
  assert [a[3] for a in said(sand.calls, "stand")] == [root, two]
  assert engine.modules[side]["actor"] == "m/low" and engine.cwd(on=side) == "/w"


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_change_of_the_world_between_two_lives_enters_the_transcript_of_a_chain() -> None:
  """A change of the World between two lives enters the transcript of a chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  was = tags(engine.turns(on=root), "opened")[1]
  assert was == ("opened", [("id", root), ("roster", STANDS[0]), ("directory", "/w"), ("actor", "m/low")], None)
  _, over = await relived(Sand(stands=LATER), plain(sand.record))
  now = tags(engine.turns(on=over), "opened")[1]
  assert now == ("opened", [("id", over), ("roster", LATER[0]), ("directory", "/z"), ("actor", "o/low")], None)
  assert engine.cwd(on=over) == "/z" and engine.modules[over]["actor"] == "o/low"


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_world_answers_a_stand_with_the_roster_the_directory_and_the_actor() -> None:
  """The World answers a stand with the roster, the directory and the actor."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  asked = said(log, "stand")[0]
  answered = next(a for a in said(log, "done") if a[1] == asked[1])
  assert answered[2] == WORLD and answered[3] == STANDS
  assert answered[3] == (STANDS[0], "/w", "m/low")
  assert said(sand.calls, "stand") == [asked]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
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
  assert [attr(tag, "roster") for tag in tags(asked[5], "opened") if ("id", over) in tag[1] and len(tag[1]) == 4] == [
    LATER[0]
  ]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_world_answers_it_while_the_chain_waits() -> None:
  """The World answers it while the chain waits, since what a chain stands on is asked and never done."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  asked = said(log, "stand")[0]
  assert asked[1] == "stand://operator.1.1"
  assert asked[1] in engine.asked and asked[1] not in engine.acts
  assert engine.modules[root]["actor"] == "m/low"


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_chain_asks_what_it_stands_on_at_its_open_and_never_again_in_that_life() -> None:
  """A chain asks what it stands on at its open and never again in that life, and every life asks the World again, so a change of the World reaches every chain of the next life."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert [a[3] for a in said(sand.calls, "stand")] == [root]
  later = Sand(stands=LATER)
  await relived(later, plain(sand.record))
  assert [a[3] for a in said(later.calls, "stand")] == [root]
  assert engine.cwd(on=root) == "/z" and engine.modules[root]["actor"] == "o/low"
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert [a[3] for a in said(later.calls, "stand")] == [root]
  assert engine.modules[twin]["actor"] == "o/low"


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_each_standing_binds_the_default_actor_of_the_chain_under_the_name_actor() -> None:
  """Each standing binds the default actor of the chain, under the name actor."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert engine.modules[root]["actor"] == "m/low" == said(log, "ask")[0][4]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_chain_tells_the_standing_it_was_answered_under_the_name_opened() -> None:
  """The chain tells the standing it was answered under the name opened, with the roster, the directory and the actor."""
  sand = sown()
  _, root = life(sand)
  told = tags(engine.turns(on=root), "opened")
  assert dict(told[1][1]) == {"id": root, "roster": STANDS[0], "directory": "/w", "actor": "m/low"}


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_chain_holds_no_stand() -> None:
  """The chain holds no stand, since the standing it tells is what its transcript holds of it."""
  sand = sown()
  _, root = life(sand)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert said(held, "stand") == []
  assert [a for a in held if a[1].startswith("stand://")] == []
  assert dict(tags(engine.turns(on=root), "opened")[1][1])["roster"] == STANDS[0]
