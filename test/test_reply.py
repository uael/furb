"""Reply, the question of a turn of a model, which the World answers with that turn."""

import asyncio
from dataclasses import dataclass
from functools import partial

from conftest import (
  STANDS,
  WORLD,
  Sand,
  World,
  heads,
  life,
  paragraphs,
  plain,
  relived,
  rows,
  said,
  settle,
  sown,
  takes,
  world_says,
)
from furb import engine
from furb.engine import OPERATOR, Refused


@dataclass
class Busy(Sand):
  """A World that cannot answer the first reply.

  It pauses the chain, refuses that reply, and answers every reply after it from its script.
  """

  balked: bool = False

  def hears(self) -> World:
    """The World that refuses one reply, and answers the replies after it."""
    loop = asyncio.get_running_loop()
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          yield "done", qid, self.stands or [[], "", ""]
        case ("reply", about, _, on, _):
          self.calls.append(a)
          if not self.balked:
            self.balked = True
            engine.pause(on)
            yield "done", about, Refused("the World is busy")
          else:
            yield "started", about
            word = self.script[on].pop(0)
            turn = ("assistant", word, self.cost or (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
            loop.call_soon(partial(world_says, "done", about, turn))


async def test_the_request_of_a_reply_is_the_transcript_of_the_chain_as_turns() -> None:
  """The request of a reply is the transcript of the chain as turns, which the World reads when it takes it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (asked,) = said(log, "reply")
  assert (
    sand.turns[asked[1]]
    == engine.turns(on=root)[:1]
    == [
      (
        "user",
        f"#{root} root\n{root}: Act[object] = Act({root!r})\n\n{takes(root)}\n\n"
        f"#{act} count\n{act}: Act[int] = Act({act!r})\n\n#{step} advance on {act}",
        None,
        None,
      )
    ]
  )


async def test_the_model_reads_the_turns_of_the_chain_at_each_step() -> None:
  """The model reads the turns of the chain at each step."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["k = 1", "close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  asked = [sand.turns[a[1]] for a in said(log, "reply")]
  assert [len(a) for a in asked] == [1, 3]
  assert [turn[0] for turn in asked[1]] == ["user", "assistant", "user"]
  assert list(asked[1])[:1] == list(asked[0])


async def test_a_reply_carries_the_rung_it_asks_for_as_its_maker() -> None:
  """A reply carries the rung it asks for as its maker, the chain that asks as the chain it is on, and the actor as its one word, and nothing else."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (word,) = said(log, "reply")
  assert word == ("reply", "reply1", step, root, "m/low")


async def test_a_reply_is_on_the_chain_that_asks() -> None:
  """A reply is on the chain that asks, so the World keys its facts and its cache by chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  sand.script[root] = ["close(1)"]
  sand.script[two] = ["close(2)"]
  assert await engine.prompt(int, "one", on=root) == 1
  assert await engine.prompt(int, "two", on=two) == 2
  await settle()
  assert [a[3] for a in said(log, "reply")] == [root, two]
  assert [a[3] for a in said(sand.calls, "reply")] == [root, two]


async def test_a_reply_the_world_cannot_answer_is_the_worlds_to_refuse() -> None:
  """A reply the World cannot answer is the World's to refuse: it pauses the chain first when it wants a wake, and is done with the refusal, which the rung comes to and the prompt asks again after."""
  sand = Busy(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  one = engine.prompt(int, "count", on=root)
  await settle()
  (first,) = [a[1] for a in said(log, "rung") if a[2] == one]
  assert engine.peek(one) is None and [a[2] for a in said(log, "reply")] == [first]
  assert [(a[1], a[2]) for a in said(log, "pause")] == [(root, WORLD)]
  assert [type(a[3]).__name__ for a in said(log, "done") if a[1] == "reply1"] == ["Refused"]
  engine.wake(root)
  await settle()
  assert [type(a[3]).__name__ for a in said(log, "done") if a[1] == first] == ["Refused"]
  assert (await one) == 7
  assert [a[2] for a in said(log, "reply")] == [a[1] for a in said(log, "rung") if a[2] == one] == [first, "rung3"]


async def test_the_world_reads_the_turns_of_the_chain_whole_when_it_takes_a_reply() -> None:
  """The World reads the turns of the chain whole when it takes a reply, folded again for that reply, and the record keeps the answer and no turns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["k = 1", "close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  asked = [sand.turns[a[1]] for a in said(log, "reply")]
  assert asked[0][0] == asked[1][0] and asked[0][0] is not asked[1][0]
  kept = [e[0] for e in sand.record if e[0][0] in ("reply", "done") and e[0][1].startswith("reply")]
  assert [len(a) for a in kept if a[0] == "reply"] == [5, 5] and [a[3][1] for a in kept if a[0] == "done"] == [
    "k = 1",
    "close(k + 1)",
  ]


async def test_the_world_answers_a_reply_with_a_done_whose_value_is_the_turn_of_the_model() -> None:
  """The World answers a reply with a done whose value is the turn of the model, which stands as the turn it is, with its usage and the blocks of the provider."""
  sand = Sand(stands=STANDS, cost=(80000, 7, 0, 0, 1.5))
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  (answer,) = [a for a in said(log, "done") if a[1] == "reply1"]
  assert answer == ("done", "reply1", WORLD, ("assistant", "close(1)", (80000, 7, 0, 0, 1.5), ["signed 8"]))
  assert [turn for turn in engine.turns(on=root) if turn[0] == "assistant"] == [answer[3]]


async def test_many_prompts_are_pending_on_one_chain_at_once() -> None:
  """Many prompts are pending on one chain at once."""
  sand = sown()
  _, root = life(sand)
  one = engine.prompt(int, "one", to=OPERATOR, on=root)
  two = engine.prompt(int, "two", to=OPERATOR, on=root)
  await settle()
  assert engine.peek(one) is None and engine.peek(two) is None
  engine.close(1, one)
  engine.close(2, two)
  await settle()
  assert ((await one), (await two)) == (1, 2)


async def test_a_chain_has_at_most_one_reply_in_flight() -> None:
  """A chain has at most one reply in flight."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["close(1)", "close(2)", "close(None)", "close(None)"]
  first = engine.prompt(int, "first", on=root)
  second = engine.prompt(int, "second", on=root)
  await settle()
  assert ((await first), (await second)) == (1, 2)
  heard = engine.transcript(root)
  opens = {a[2]: i for i, a in enumerate(heard) if a[0] == "reply"}
  ends = {a[1]: i for i, a in enumerate(heard) if a[0] == "ready" and a[1] in opens}
  assert all(ends[one] < opens[other] for one, other in zip(opens, list(opens)[1:], strict=False))


async def test_across_chains_there_is_no_limit_on_the_replies_in_flight() -> None:
  """Across chains there is no limit on the replies in flight."""
  sand = sown()
  log, root = life(sand)
  two = engine.chain("two")
  here = engine.prompt(int, "here", on=root)
  there = engine.prompt(int, "there", on=two)
  await settle()
  assert [a[3] for a in said(log, "reply")] == [root, two]
  assert engine.peek(here) is None and engine.peek(there) is None
  assert sand.script.get(root, []) == []


async def test_every_model_asked_on_a_chain_reads_all_the_turns_of_the_chain_as_the_turns_grow() -> None:
  """Every model asked on a chain reads all the turns of the chain, as the turns grow."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  first, second = [sand.turns[a[1]] for a in said(log, "reply")][:2]
  assert paragraphs(second)[: len(paragraphs(first))] == paragraphs(first)
  assert second[:1] == first and [turn[0] for turn in second] == ["user", "assistant", "user"]


async def test_as_many_replies_as_there_are_chains_are_in_flight_together() -> None:
  """As many replies as there are chains are in flight together."""
  sand = sown()
  log, root = life(sand)
  two, three = engine.chain("two"), engine.chain("three")
  for one in (root, two, three):
    engine.prompt(int, "work", on=one)
  await settle()
  assert [a[3] for a in said(log, "reply")] == [root, two, three]


async def test_a_later_life_asks_no_model_and_does_no_act_whose_close_the_record_already_holds() -> None:
  """A later life asks no model, and does no act, whose close the record already holds."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  later = Sand(stands=STANDS)
  await relived(later, plain(sand.record))
  assert [a for a in later.calls if a[0] in ("reply", "bash")] == []


async def test_a_new_prompt_reads_the_whole_transcript_of_the_chain_the_cancelled_work_included() -> None:
  """A new prompt reads the whole transcript of the chain, the cancelled work included."""
  sand = sown()
  log, root = life(sand)
  first = engine.prompt(int, "one", to=OPERATOR, on=root)
  await settle()
  engine.cancel(first)
  await settle()
  sand.script[root] = ["close(2)", "close(None)"]
  second = engine.prompt(int, "two", on=root)
  assert await second == 2
  (step,) = [a[1] for a in said(log, "rung") if a[2] == second]
  assert heads(sand.turns["reply1"]) == [
    f"#{root} root",
    rows(root)[0],
    f"#{first} one",
    f"#{first} cancelled",
    f"#{second} two",
    f"#{step} advance on {second}",
  ]


async def test_no_prompt_that_a_pause_is_over_asks_a_model() -> None:
  """No prompt that a pause is over asks a model."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  one = engine.prompt(int, "count", on=root)
  engine.pause(one)
  await settle()
  assert len(said(log, "reply")) == 1
  engine.wake(one)
  await settle()
  assert (await one) == 2 and len(said(log, "reply")) == 2


async def test_a_paused_chain_makes_no_new_reply_after_a_held_response() -> None:
  """A paused chain makes no new reply after a held response."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  one = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  assert len([a for a in said(log, "done") if a[1].startswith("reply")]) == 1 and len(said(log, "reply")) == 1
  await settle()
  assert len(said(log, "reply")) == 1 and engine.peek(one) is None


async def test_no_model_is_asked_for_a_rung_the_record_answered() -> None:
  """No model is asked for a rung the record answered, since a later life asks again for nothing it was answered once."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  later = Sand(stands=STANDS)
  _, over = await relived(later, plain(sand.record))
  assert [a for a in later.calls if a[0] == "reply"] == []
  assert engine.module(over)["a"] == 1


async def test_it_asks_for_no_rung_a_pause_stands_over() -> None:
  """It asks for no rung a pause stands over, whether the pause is over that rung or over the chain, so a paused chain asks no model until the wake."""
  sand = sown()
  log, root = life(sand)
  engine.pause(root)
  sand.script[root] = ["close(1)", "close(None)"]
  one = engine.prompt(int, "count", on=root)
  await settle()
  assert said(log, "reply") == [] and engine.peek(one) is None
  engine.wake(root)
  await settle()
  assert (await one) == 1 and len(said(log, "reply")) == 1


async def test_the_chain_asks_one_model_at_a_time() -> None:
  """The chain asks one model at a time, which it reads from its transcript, the rung that has waited the longest among those it heard on itself that nothing has been said of, handing it the turns as they stand, for the World to hand its provider as it likes, so that many chains ask many models at once."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)", "close(2)"]
  one = engine.prompt(int, "first", on=root)
  two = engine.prompt(int, "second", on=root)
  assert (await one, await two) == (1, 2)
  await settle()
  asks = said(log, "reply")
  assert [a[2] for a in asks] == [a[1] for a in said(log, "rung") if a[2] in (one, two)]
  assert [a[0] for a in log if a[1] in ("reply1", "reply2") and a[0] in ("reply", "done")] == [
    "reply",
    "done",
    "reply",
    "done",
  ]
  assert sand.turns["reply1"] == engine.turns(on=root)[:1] and sand.turns["reply2"] == engine.turns(on=root)[:3]
  side = engine.chain("side")
  mine = engine.prompt(int, "here", on=root)
  theirs = engine.prompt(int, "there", on=side)
  await settle()
  assert engine.peek(mine) is None and engine.peek(theirs) is None
  assert [a[3] for a in said(log, "reply")[2:]] == [root, side]
