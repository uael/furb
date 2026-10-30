"""stand, the question of what the chains stand on."""

from conftest import LATER, STANDS, Sand, born, chained, of, paragraphs, relived, said, settle, stood
from furb import engine
from furb.engine import OPERATOR


async def test_stand_asks_the_world_what_the_chains_stand_on_and_gives_the_answer() -> None:
  """stand asks the World what the chains stand on and gives the answer, which boot does on the root at the tip of every life."""
  sand, _, root = born()
  assert [(a[1], a[2], a[3]) for a in said(sand.calls, "stand")] == [("stand1", OPERATOR, root)]
  sand.stands = LATER
  assert engine.stand() == LATER and engine.standing() == LATER
  await settle()
  later = Sand()
  await relived(later, list(sand.record))
  assert [(a[1], a[2], a[3]) for a in said(later.calls, "stand")] == [("stand3", OPERATOR, root)]


async def test_only_a_stand_on_the_root_changes_the_standing() -> None:
  """Only a stand on the root changes the standing, since the standing has one home."""
  sand, _, root = born()
  two = engine.chain("two")
  sand.stands = LATER
  assert engine.stand(on=two) == LATER
  assert engine.standing() == STANDS and engine.module(two)["actor"] == engine.module(root)["actor"] == "m/low"
  assert engine.stand() == LATER and engine.module(two)["actor"] == engine.module(root)["actor"] == "o/low"


async def test_a_stand_from_inside_an_act_tells_its_answer_under_its_own_name() -> None:
  """A stand from inside an act tells its answer under its own name, with the roster and the actor of the standing, as stand2_roster and stand2_actor, and no directory, which a cd tells, and no path of the doctrine, which the World reads in a turn of its own."""
  _, log, root = born(stands=[*STANDS, "/w/DOCTRINE.md"])
  two = await chained("two")
  was = paragraphs(engine.turns(on=root))
  step = engine.rung("got = stand()", on=two)
  assert await step is None
  asked = said(log, "stand")[-1][1]
  assert asked == "stand2" and engine.module(two)["got"] == [*STANDS, "/w/DOCTRINE.md"]
  assert of(engine.turns(on=two), asked) == [stood(asked)]
  assert stood(asked) == (
    "#stand2\nstand2_roster = [['operator', [], 200000], ['m', ['low', 'high'], 400000], ['n', ['low'], 200000]]\n"
    "stand2_actor = 'm/low'"
  )
  assert [a for a in engine.transcript(two) if a[0] == "tell" and a[1] == asked] == [
    ("tell", asked, step, [stood(asked)])
  ]
  assert paragraphs(engine.turns(on=root)) == was


async def test_a_stand_from_outside_an_act_tells_nothing() -> None:
  """A stand from outside an act tells nothing, so the stand that boot makes tells nothing."""
  sand, log, root = born()
  assert [a for a in engine.transcript(root) if a[0] == "tell" and a[1] == "stand1"] == []
  was = engine.transcript(root)
  sand.stands = LATER
  assert engine.stand() == LATER
  assert said(log, "stand")[-1][1] == "stand2"
  assert [a for a in engine.transcript(root) if a[0] == "tell"] == [a for a in was if a[0] == "tell"]


async def test_a_stand_that_names_no_chain_is_on_the_chain_of_the_act_that_makes_it() -> None:
  """A stand that names no chain is on the chain of the act that makes it, as every verb, and on the root from outside an act, as the stand of boot."""
  _, log, root = born()
  two = await chained("two")
  await engine.rung("stand()", on=two)
  engine.stand()
  assert [(a[1], a[3]) for a in said(log, "stand")] == [("stand1", root), ("stand2", two), ("stand3", root)]
  assert engine.get("stand2")[2] == said(log, "rung")[-1][1]
