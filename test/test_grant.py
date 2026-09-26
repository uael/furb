"""grant, a ceiling on a chain."""

from asyncio import CancelledError

from conftest import STANDS, Sand, heads, life, paragraphs, ran, rows, said, settle
from furb import engine
from furb.engine import OPERATOR, Refused

COST = (80000, 0, 0, 0, 1.5)
"""One answer of a model: a dollar and a half, and a fifth of the window of the actor the suite stands on."""
BOUND = "chain1: Act[object] = Act('chain1')\ngrant1: Act[None] = Act('grant1')\nprompt1: Act[int] = Act('prompt1')"
"""The rung the root writes at its first ask, which binds the acts its first turn opened: itself, a grant, a prompt."""


async def test_a_ceiling_on_a_chain_in_dollars_in_the_share_of_the_window_or_both() -> None:
  """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last one filled, and it tells that ledger at each answer of a model, so no turn a reply has sent grows a line after it."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  engine.grant(usd=10.0, on=root)
  sand.script[root] = ["a = 1", "b = 2", "close(a + b)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  one, two, three = [a[2] for a in said(log, "reply")]
  final = engine.turns(on=root)
  assert [line for line in heads(final) if " ledger " in line] == [
    f"#{one} ledger spent=1.5 filled=0.2",
    f"#{two} ledger spent=3.0 filled=0.2",
    f"#{three} ledger spent=4.5 filled=0.2",
  ]
  assert [final[: len(turns)] == turns for turns in sand.turns.values()] == [True, True, True]


async def test_grant_on_a_chain_puts_a_ceiling_on_it_dollars_a_share_of_the_window_or_both() -> None:
  """grant on a chain puts a ceiling on it: dollars, a share of the window, or both."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  two, three = engine.chain("two"), engine.chain("three")
  await settle()
  engine.grant(usd=1.0, on=root)
  engine.grant(share=0.1, on=two)
  engine.grant(usd=1.0, share=0.1, on=three)
  await settle()
  made = said(log, "grant")
  assert [(one[3], one[4], one[5]) for one in made] == [(root, 1.0, None), (two, None, 0.1), (three, 1.0, 0.1)]
  for who in (root, two, three):
    sand.script[who] = ["close(1)"]
    engine.prompt(int, "spend", on=who)
  await settle()
  assert sorted((one[1], one[2]) for one in said(log, "pause")) == [(one[3], one[1]) for one in made]


async def test_the_engine_enters_a_pause_on_the_chain_when_a_response_carries_the_ledger_to_its_ceiling() -> None:
  """The engine enters a pause on the chain when a response carries the ledger to its ceiling."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  ceiling = engine.grant(usd=1.0, on=root)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert engine.peek(act, ...) is ...
  assert [(one[1], one[2]) for one in said(log, "pause")] == [(root, ceiling)]


async def test_the_word_of_the_response_that_crossed_the_ceiling_runs() -> None:
  """The word of the response that crossed the ceiling runs."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  engine.grant(usd=1.0, on=root)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert ran(log) == [BOUND, "a = 1"] and engine.module(root)["a"] == 1
  assert engine.peek(act, ...) is ...


async def test_no_reply_follows_the_response_that_carried_the_ledger_to_the_ceiling_until_a_wake() -> None:
  """No reply follows the response that carried the ledger to the ceiling, until a wake."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  engine.grant(usd=1.0, on=root)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert len(said(log, "reply")) == 1 and engine.peek(act, ...) is ...
  engine.wake(root)
  await settle()
  assert len(said(log, "reply")) == 2


async def test_the_model_continues_after_a_later_grant_and_a_wake() -> None:
  """The model continues after a later grant and a wake."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = life(sand)
  engine.grant(usd=1.0, on=root)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert engine.peek(act, ...) is ...
  engine.grant(usd=4.0, on=root)
  engine.wake(root)
  await settle()
  assert (await act) == 2


async def test_an_answer_that_carries_the_ledger_past_the_ceiling_pauses_the_chain() -> None:
  """An answer that carries the ledger past the ceiling pauses the chain, so the word that answer brought runs and what it gave waits, and no rung of the chain asks until the wake."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  engine.grant(usd=1.0, on=root)
  sand.script[root] = ["close(5)"]
  act = engine.prompt(int, "spend", on=root)
  await settle()
  assert ran(log) == [BOUND, "close(5)"] and engine.peek(act, ...) is ...
  assert len(said(log, "reply")) == 1
  engine.wake(root)
  await settle()
  assert (await act) == 5


async def test_lifting_a_ceiling_wakes_nothing() -> None:
  """Lifting a ceiling wakes nothing: the pause stands until a wake, ceiling or no ceiling."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = life(sand)
  ceiling = engine.grant(usd=1.0, on=root)
  sand.script[root] = ["close(5)"]
  act = engine.prompt(int, "spend", on=root)
  await settle()
  assert engine.peek(act, ...) is ...
  engine.cancel(ceiling)
  await settle()
  assert isinstance(engine.peek(ceiling), CancelledError) and engine.peek(act, ...) is ...
  engine.wake(root)
  await settle()
  assert (await act) == 5


async def test_it_stands_until_it_is_lifted_as_the_chain_it_is_on_does() -> None:
  """It stands until it is lifted, as the chain it is on does, so what it comes to is what lifted it: nothing for a later grant that closes it, and a CancelledError for a cancel."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.grant(usd=1.0, on=root)
  await settle()
  assert engine.peek(one, ...) is ...
  two = engine.grant(share=0.5, on=root)
  await settle()
  assert (await one) is None and engine.peek(two, ...) is ...
  engine.cancel(two)
  await settle()
  assert isinstance(engine.peek(two), CancelledError)


async def test_a_grant_of_nothing_of_a_ceiling_under_zero_or_of_a_share_past_one_is_no_ceiling() -> None:
  """A grant of nothing, of a ceiling under zero, or of a share past one is no ceiling: it is done with the refusal, which whoever made it takes by awaiting it, and it tells nothing, since it never stood; a prompt to no actor of the roster is closed the same way after it has told its open."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  none = engine.grant(on=root)
  below = engine.grant(usd=-1.0, on=root)
  beyond = engine.grant(share=1.5, on=root)
  await settle()
  got = [engine.peek(act) for act in (none, below, beyond)]
  assert [type(one) for one in got] == [Refused, Refused, Refused]
  assert [str(one) for one in got] == ["None/None no ceiling", "-1.0/None no ceiling", "None/1.5 no ceiling"]
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0]]
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.peek(ghost), Refused) and said(log, "reply") == []
  assert paragraphs(engine.turns(on=root))[2:] == [
    f"#{ghost} hi\n{ghost}: Act[int] = Act({ghost!r})",
    f"#{ghost} closed Refused('ghost no actor')",
  ]


async def test_a_later_grant_that_stands_closes_every_grant_of_the_chain_before_it_that_stands() -> None:
  """A later grant that stands closes every grant of the chain before it that stands, and none that is over, so the ledger counts from the new one alone; one that is no ceiling closes nothing, and the ceiling that stands stands on."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  ceiling = engine.grant(usd=2.0, on=root)
  sand.script[root] = ["a = 1", "b = 2", "close(a + b)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert engine.peek(act, ...) is ...
  top = engine.grant(usd=4.0, on=root)
  await settle()
  assert (await ceiling) is None and engine.peek(top, ...) is ...
  none = engine.grant(usd=-1.0, on=root)
  await settle()
  assert isinstance(engine.peek(none), Refused) and engine.peek(top, ...) is ...
  engine.wake(root)
  assert await act == 3
  await settle()
  one, two, three = [a[2] for a in said(log, "reply")]
  final = heads(engine.turns(on=root))
  assert [line for line in final if " ledger " in line] == [
    f"#{one} ledger spent=1.5 filled=0.2",
    f"#{two} ledger spent=3.0 filled=0.2",
    f"#{three} ledger spent=1.5 filled=0.2",
  ]
  assert [line for line in final if line.split(" ")[0] in (f"#{ceiling}", f"#{top}", f"#{none}")] == [
    f"#{ceiling} usd=2.0 share=None",
    f"#{ceiling} closed None",
    f"#{top} usd=4.0 share=None",
  ]


async def test_a_cancel_of_it_lifts_the_ceiling_since_it_is_an_act_like_any_other() -> None:
  """A cancel of it lifts the ceiling, since it is an act like any other."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  ceiling = engine.grant(usd=1.0, on=root)
  await settle()
  engine.cancel(ceiling)
  await settle()
  assert isinstance(engine.peek(ceiling), CancelledError)
  sand.script[root] = ["a = 1", "close(2)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  assert said(log, "pause") == []


async def test_a_grant_is_any_callers_on_any_chain() -> None:
  """A grant is any caller's, on any chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  mine = engine.grant(usd=1.0, on=two)
  await settle()
  sand.script[root] = [f"theirs = grant(usd=2.0, on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "grant", on=root) == 1
  await settle()
  step = said(log, "reply")[0][2]
  assert [(one[3], one[2]) for one in said(log, "grant")] == [(two, OPERATOR), (two, step)]
  assert (await mine) is None


async def test_a_grant_finds_the_grants_of_its_chain_among_the_acts_of_the_life() -> None:
  """A grant finds the grants of its chain among the acts of the life, so a grant on a chain with a source closes no grant of its origin."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  first = engine.grant(usd=5.0, on=root)
  await settle()
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = engine.grant(usd=9.0, on=twin)
  await settle()
  assert engine.peek(first, ...) is ... and engine.peek(theirs, ...) is ...
  mine = engine.grant(usd=7.0, on=root)
  await settle()
  assert engine.peek(first) is None and engine.peek(theirs, ...) is ... and engine.peek(mine, ...) is ...
