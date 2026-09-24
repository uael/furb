"""grant, a ceiling on a chain."""

from asyncio import CancelledError

from conftest import GRANT, STANDS, Sand, heads, life, of, ran, said, settle, verb
from furb import engine
from furb.engine import OPERATOR, WORLD, Act, Refused

COST = (80000, 0, 0, 0, 1.5)
"""One answer of a model: a dollar and a half, and a fifth of the window of the actor the suite stands on."""
BOUND = "chain1: Act[object] = Act('chain1')\ngrant1: Act[None] = Act('grant1')\nprompt1: Act[int] = Act('prompt1')"
"""The rung the root writes at its first ask, which binds the acts its first turn opened: itself, a grant, a prompt."""


def grant(on: str, usd: float | None = None, share: float | None = None) -> Act:
  """A grant on a chain, as the operator says it."""
  got = verb("grant", on)(usd=usd, share=share)
  assert isinstance(got, Act)
  return got


async def test_a_ceiling_on_a_chain_in_dollars_in_the_share_of_the_window_or_both() -> None:
  """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last one filled, and it tells that ledger at each answer of a model, so no turn an ask has sent grows a line after it."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  grant(root, usd=10.0)
  sand.script[root] = ["a = 1", "b = 2", "close(a + b)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  one, two, three = [a[1] for a in said(log, "answer")]
  final = engine.turns(on=root)
  assert [line for line in heads(final) if " ledger " in line] == [
    f"#{one} ledger spent=1.5 filled=0.2",
    f"#{two} ledger spent=3.0 filled=0.2",
    f"#{three} ledger spent=4.5 filled=0.2",
  ]
  assert [final[: len(a[5])] == a[5] for a in said(log, "ask")] == [True, True, True]


async def test_grant_on_a_chain_puts_a_ceiling_on_it_dollars_a_share_of_the_window_or_both() -> None:
  """grant on a chain puts a ceiling on it: dollars, a share of the window, or both."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  two, three = engine.chain("two"), engine.chain("three")
  await settle()
  grant(root, usd=1.0)
  grant(two, share=0.1)
  grant(three, usd=1.0, share=0.1)
  await settle()
  made = said(log, "grant")
  assert [(one[3], one[4], one[5]) for one in made] == [(root, 1.0, None), (two, None, 0.1), (three, 1.0, 0.1)]
  for who in (root, two, three):
    sand.script[who] = ["close(1)"]
    engine.prompt(int, "spend", on=who)
  await settle()
  assert sorted((one[1], one[2]) for one in said(log, "pause")) == [(one[3], one[1]) for one in made]


async def test_a_grant_enters_a_pause_on_the_chain_when_a_response_carries_the_ledger_to_its_ceiling() -> None:
  """A grant enters a pause on the chain when a response carries the ledger to its ceiling."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  ceiling = grant(root, usd=1.0)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert act not in engine.outcomes
  assert [(one[1], one[2]) for one in said(log, "pause")] == [(root, ceiling)]


async def test_the_word_of_the_response_that_crossed_the_ceiling_runs() -> None:
  """The word of the response that crossed the ceiling runs."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  grant(root, usd=1.0)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert ran(log)[1:] == [BOUND, "a = 1"] and engine.modules[root]["a"] == 1
  assert act not in engine.outcomes


async def test_no_ask_follows_the_response_that_carried_the_ledger_to_the_ceiling_until_a_wake() -> None:
  """No ask follows the response that carried the ledger to the ceiling, until a wake."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  grant(root, usd=1.0)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert len(said(log, "ask")) == 1 and act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert len(said(log, "ask")) == 2


async def test_the_model_continues_after_a_later_grant_and_a_wake() -> None:
  """The model continues after a later grant and a wake."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  _, root = life(sand)
  grant(root, usd=1.0)
  sand.script[root] = ["a = 1", "close(2)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert act not in engine.outcomes
  grant(root, usd=4.0)
  engine.wake(root)
  await settle()
  assert (await act) == 2


async def test_an_answer_that_carries_the_ledger_past_the_ceiling_pauses_the_chain() -> None:
  """An answer that carries the ledger past the ceiling pauses the chain, so the word that answer brought runs and what it gave waits, and no rung of the chain asks until the wake."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  grant(root, usd=1.0)
  sand.script[root] = ["close(5)"]
  act = engine.prompt(int, "spend", on=root)
  await settle()
  assert ran(log)[1:] == [BOUND, "close(5)"] and act not in engine.outcomes
  assert len(said(log, "ask")) == 1
  engine.wake(root)
  await settle()
  assert (await act) == 5


async def test_lifting_a_ceiling_wakes_nothing() -> None:
  """Lifting a ceiling wakes nothing: the pause stands until a wake, ceiling or no ceiling."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  _, root = life(sand)
  ceiling = grant(root, usd=1.0)
  sand.script[root] = ["close(5)"]
  act = engine.prompt(int, "spend", on=root)
  await settle()
  assert act not in engine.outcomes
  engine.cancel(ceiling)
  await settle()
  assert isinstance(engine.outcomes[ceiling], CancelledError) and act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 5


async def test_it_stands_until_it_is_lifted_as_the_chain_it_is_on_does() -> None:
  """It stands until it is lifted, as the chain it is on does, so what it comes to is what lifted it: nothing for a later grant that closes it, and a CancelledError for a cancel."""
  sand = Sand(stands=STANDS, words=GRANT)
  _, root = life(sand)
  one = grant(root, usd=1.0)
  await settle()
  assert one not in engine.outcomes
  two = grant(root, share=0.5)
  await settle()
  assert (await one) is None and two not in engine.outcomes
  engine.cancel(two)
  await settle()
  assert isinstance(engine.outcomes[two], CancelledError) and isinstance(engine.peek(two, on=root), CancelledError)


async def test_a_grant_of_nothing_of_a_ceiling_under_zero_or_of_a_share_past_one_is_no_ceiling() -> None:
  """A grant of nothing, of a ceiling under zero, or of a share past one is no ceiling: it is done with the refusal, which whoever made it takes by awaiting it, and it tells nothing, since it never stood."""
  sand = Sand(stands=STANDS, words=GRANT)
  log, root = life(sand)
  none = grant(root)
  below = grant(root, usd=-1.0)
  beyond = grant(root, share=1.5)
  await settle()
  got = [engine.outcomes[act] for act in (none, below, beyond)]
  assert [type(one) for one in got] == [Refused, Refused, Refused]
  assert [str(one) for one in got] == ["None/None no ceiling", "-1.0/None no ceiling", "None/1.5 no ceiling"]
  assert [a for a in said(log, "tell") if a[1] in (none, below, beyond)] == []
  assert [of(engine.turns(on=root), act) for act in (none, below, beyond)] == [[], [], []]
  word = "try:\n  await grant(usd=-1.0)\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "-1.0/None no ceiling"


async def test_a_later_grant_that_stands_closes_every_grant_of_the_chain_before_it_that_stands() -> None:
  """A later grant that stands closes every grant of the chain before it that stands, and none that is over, so the ledger counts from the new one alone; one that is no ceiling closes nothing, and the ceiling that stands stands on."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  ceiling = grant(root, usd=2.0)
  sand.script[root] = ["a = 1", "b = 2", "close(a + b)"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert act not in engine.outcomes
  top = grant(root, usd=4.0)
  await settle()
  assert (await ceiling) is None and top not in engine.outcomes
  none = grant(root, usd=-1.0)
  await settle()
  assert isinstance(engine.outcomes[none], Refused) and top not in engine.outcomes
  engine.wake(root)
  assert await act == 3
  await settle()
  one, two, three = [a[1] for a in said(log, "answer")]
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
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  ceiling = grant(root, usd=1.0)
  await settle()
  engine.cancel(ceiling)
  await settle()
  assert isinstance(engine.outcomes[ceiling], CancelledError)
  assert isinstance(engine.peek(ceiling, on=root), CancelledError)
  sand.script[root] = ["a = 1", "close(2)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  assert said(log, "pause") == []


async def test_a_grant_is_any_callers_on_any_chain() -> None:
  """A grant is any caller's, on any chain."""
  sand = Sand(stands=STANDS, words=GRANT)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  mine = grant(two, usd=1.0)
  await settle()
  sand.script[root] = [f"theirs = grant(usd=2.0, on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "grant", on=root) == 1
  await settle()
  step = said(log, "answer")[0][1]
  assert [(one[3], one[2]) for one in said(log, "grant")] == [(two, OPERATOR), (two, step)]
  assert (await mine) is None


async def test_a_grant_finds_the_grants_of_its_chain_among_the_acts_of_the_life() -> None:
  """A grant finds the grants of its chain among the acts of the life, so a grant on a chain with a source closes no grant of its origin."""
  sand = Sand(stands=STANDS, words=GRANT)
  _, root = life(sand)
  first = grant(root, usd=5.0)
  await settle()
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = grant(twin, usd=9.0)
  await settle()
  assert first not in engine.outcomes and theirs not in engine.outcomes
  mine = grant(root, usd=7.0)
  await settle()
  assert engine.outcomes[first] is None and theirs not in engine.outcomes and mine not in engine.outcomes


async def test_a_grant_reads_the_window_of_a_rung_off_the_standing_that_the_chain_stood_on_when_it_heard_that_rung() -> (
  None
):
  """A grant reads the window of a rung off the standing that the chain it is on stood on when it heard that rung."""
  sand = Sand(stands=STANDS, cost=COST, words=GRANT)
  log, root = life(sand)
  grant(root, usd=10.0)
  asked = engine.prompt(int, "count", on=root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == asked]
  await engine.rung("actor = 'n/low'", on=root)
  engine.send("answer", step, ("assistant", "close(1)", COST, None), by=WORLD)
  assert await asked == 1
  assert engine.acts[step][6] == "m/low" and engine.modules[root]["actor"] == "n/low"
  assert [line for line in heads(engine.turns(on=root)) if " ledger " in line] == [
    f"#{step} ledger spent=1.5 filled=0.2"
  ]
