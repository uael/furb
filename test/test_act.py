"""act, the way to make a question that lives."""

from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, life, paragraphs, said, settle
from furb import engine
from furb.engine import OPERATOR, Act, Refused, take


def noting(heard: list[object], how: str = ""):  # noqa: ANN201
  """A life of the suite: it keeps the name it is given and every fact it hears, and says two facts when it is
  given a way, one by yielding it and one by calling the bus.
  """

  def lives(id: str) -> Generator[tuple | None, tuple]:
    heard.append(id)
    if how:
      yield "tell", id, [f"#{id} {how} by yield"]
      engine.send("tell", id, [f"#{id} {how} by the bus"])
    while True:
      if (a := (yield)) is not None:
        heard.append(a)

  return lives


async def test_the_way_to_make_a_question_that_lives() -> None:
  """The way to make a question that lives: it takes a name when it is made, it is logged, its ear is brought to life under that name and given the name, and the name is given back, which is the act to whoever holds it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[object] = []
  one = engine.act("note", root, noting(heard), "one")
  assert isinstance(one, Act) and one == "note1"
  assert said(log, "note") == [("note", one, OPERATOR, root, "one")]
  assert engine.acts[one] == ("note", one, OPERATOR, root, "one")
  assert heard[0] == one


async def test_an_act_said_it_is_begun_and_what_the_call_gives_is_its_name() -> None:
  """An act said: it is begun, and what the call gives is its name, which is awaited for what the act comes to."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(str, "hi", to=OPERATOR, on=root)
  await settle()
  assert isinstance(one, Act) and engine.peek(one) is None
  assert paragraphs(engine.turns(on=root))[-1] == "#prompt1 hi\nprompt1: Act[str] = Act('prompt1')"
  engine.close("yes", one)
  assert (await one) == "yes"


async def test_an_act_said_twice_under_one_name_is_one_act() -> None:
  """An act said twice under one name is one act, and the second saying brings no second ear and gives the name back."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  first = engine.rung("close(wait(0))", on=root)
  again = engine.rung("close(wait(0))", retells=first, on=root)
  assert (await first) == "wait1" and (await again) is None
  assert len(said(log, "wait")) == 1
  assert [a[1] for a in sand.calls if a[0] == "start"] == ["wait1"]


async def test_two_acts_that_say_the_same_words_under_one_name_are_one_act() -> None:
  """Two acts that say the same words under one name are one act."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  first = engine.rung("close(wait(0))", on=root)
  again = engine.rung("close(wait(0))", retells=first, on=root)
  one = await first
  assert isinstance(one, str)
  assert (await again) is None
  assert engine.get(one) == ("wait", one, first, root, 0)
  await settle()
  assert one in engine.outcomes and engine.peek(one) is None
  assert [e[0][1] for e in sand.record if e[0][0] == "wait"] == [one]


async def test_the_engine_refuses_an_act_said_from_outside_a_run_that_names_no_chain() -> None:
  """The engine refuses an act said from outside a run that names no chain, a chain apart."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  heard: list[object] = []
  with pytest.raises(Refused, match="no chain"):
    engine.act("note", "", noting(heard))
  assert said(log, "note") == [] and heard == []
  assert engine.chain("two") == "chain2"


async def test_the_chain_an_act_is_on_is_the_chain_named_to_the_call() -> None:
  """The chain an act is on is the chain named to the call, or the scope of the one that made it when the call names none."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  heard: list[object] = []
  named = engine.act("note", two, noting(heard), "one")
  assert engine.get(named)[3] == two
  sand.script[root] = ["close(wait(0))"]
  unsaid = await engine.prompt(str, "start one", on=root)
  assert engine.get(unsaid)[3] == root


async def test_the_ear_of_an_act_is_given_the_name_of_the_act_and_hears_every_fact_said_after_its_birth() -> None:
  """The ear of an act is given the name of the act and hears every fact said after its birth, and it speaks by yielding a saying or by calling the bus."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  heard: list[object] = []
  one = engine.act("note", root, noting(heard, "spoke"), "one")
  two = engine.prompt(str, "hi", to=OPERATOR, on=root)
  await settle()
  assert heard[0] == one and ("prompt", two, OPERATOR, root, "str", "hi", OPERATOR) in heard[1:]
  assert paragraphs(engine.turns(on=root))[2:] == [
    "#note1 spoke by yield",
    "#note1 spoke by the bus",
    "#prompt1 hi\nprompt1: Act[str] = Act('prompt1')",
  ]


async def test_an_act_carries_the_words_of_its_kind() -> None:
  """An act carries the words of its kind, which are the plain arguments the verb was given, in the order of the verb, and a show or a filter is none of them."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.wait(5.0, on=root)
  assert engine.get(one) == ("wait", one, OPERATOR, root, 5.0)
  two = engine.chain("two", source=root, filter=take(root))
  assert engine.get(two) == ("chain", two, OPERATOR, "", "two", root)
