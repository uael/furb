"""Ask, the request the World answers with a turn of a model."""

import asyncio
from dataclasses import dataclass
from functools import partial

from conftest import STANDS, Sand, World, heads, life, paragraphs, plain, relived, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, WORLD, Refused


@dataclass
class Busy(Sand):
  """A World that cannot answer the first ask.

  It pauses the chain, closes the rung of that ask with a refusal, and answers every ask after it from its script.
  """

  balked: bool = False

  def hears(self) -> World:
    """The World that closes one ask with a refusal, and answers the asks after it."""
    loop = asyncio.get_running_loop()
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          yield "done", qid, self.stands or ((), "", "")
        case ("ask", rung, _, on, _, _):
          self.calls.append(a)
          if not self.balked:
            self.balked = True
            engine.pause(on)
            engine.close(Refused("the World is busy"), rung)
          elif self.script.get(on):
            word = self.script[on].pop(0)
            turn = ("assistant", word, self.cost or (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
            loop.call_soon(partial(engine.send, "answer", rung, turn, by=WORLD))


async def test_the_request_of_an_ask_is_the_transcript_of_the_chain_as_turns() -> None:
  """The request of an ask is the transcript of the chain as turns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (asked,) = said(log, "ask")
  assert (
    asked[5]
    == engine.turns(on=root)[:1]
    == [
      (
        "user",
        f"#{root} root\n{root}: Act[object] = Act({root!r})\n\n#{root} stands {STANDS!r}\n\n"
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
  asked = said(log, "ask")
  assert [len(a[5]) for a in asked] == [1, 3]
  assert [turn[0] for turn in asked[1][5]] == ["user", "assistant", "user"]
  assert list(asked[1][5])[:1] == list(asked[0][5])


async def test_an_ask_carries_the_rung_it_asks_for_the_chain_that_asks_the_actor_and_the_turns() -> None:
  """An ask carries the rung it asks for, the chain that asks, the actor, and the turns of the chain, in order, and nothing else."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (word,) = said(log, "ask")
  assert word == ("ask", step, root, root, "m/low", engine.turns(on=root)[:1])


async def test_an_ask_says_the_chain_that_asks_as_a_word() -> None:
  """An ask says the chain that asks as a word, so the World keys its facts and its cache by chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  sand.script[root] = ["close(1)"]
  sand.script[two] = ["close(2)"]
  assert await engine.prompt(int, "one", on=root) == 1
  assert await engine.prompt(int, "two", on=two) == 2
  await settle()
  assert [a[3] for a in said(log, "ask")] == [root, two]
  assert [a[3] for a in said(sand.calls, "ask")] == [root, two]


async def test_an_ask_the_world_cannot_answer_is_the_worlds_to_close() -> None:
  """An ask the World cannot answer is the World's to close: it pauses the chain first when it wants a wake, and closes the rung with the refusal, which the prompt asks again after."""
  sand = Busy(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  one = engine.prompt(int, "count", on=root)
  await settle()
  (first,) = [a[1] for a in said(log, "rung") if a[2] == one]
  assert engine.peek(one) is None and [a[1] for a in said(log, "ask")] == [first]
  assert [(a[1], a[2]) for a in said(log, "pause")] == [(root, WORLD)]
  assert [type(a[3]).__name__ for a in said(log, "done") if a[1] == first] == ["Refused"]
  engine.wake(root)
  await settle()
  assert (await one) == 7
  assert (
    [a[1] for a in said(log, "ask")]
    == [a[1] for a in said(log, "rung") if a[2] == one]
    == [first, said(log, "ask")[1][1]]
  )


async def test_an_ask_hands_the_turns_of_the_chain_whole_folded_again_for_that_ask() -> None:
  """An ask hands the turns of the chain whole, folded again for that ask, and the record keeps the answer and no turns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["k = 1", "close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  asked = said(log, "ask")
  assert asked[0][5][0] == asked[1][5][0] and asked[0][5][0] is not asked[1][5][0]
  kinds = [e[1][0] for e in sand.record]
  assert "answer" in kinds and "ask" not in kinds


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


async def test_a_chain_has_at_most_one_ask_in_flight() -> None:
  """A chain has at most one ask in flight."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)", "close(2)", "close(None)", "close(None)"]
  first = engine.prompt(int, "first", on=root)
  second = engine.prompt(int, "second", on=root)
  await settle()
  assert ((await first), (await second)) == (1, 2)
  opens = {a[1]: i for i, a in enumerate(log) if a[0] == "ask"}
  ends = {a[1]: i for i, a in enumerate(log) if a[0] == "ready" and a[1] in opens}
  assert all(ends[one] < opens[other] for one, other in zip(opens, list(opens)[1:], strict=False))


async def test_across_chains_there_is_no_limit_on_the_asks_in_flight() -> None:
  """Across chains there is no limit on the asks in flight."""
  sand = sown()
  log, root = life(sand)
  two = engine.chain("two")
  here = engine.prompt(int, "here", on=root)
  there = engine.prompt(int, "there", on=two)
  await settle()
  assert [a[3] for a in said(log, "ask")] == [root, two]
  assert engine.peek(here) is None and engine.peek(there) is None
  assert sand.script.get(root, []) == []


async def test_every_model_asked_on_a_chain_reads_all_the_turns_of_the_chain_as_the_turns_grow() -> None:
  """Every model asked on a chain reads all the turns of the chain, as the turns grow."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  first, second = said(log, "ask")[0], said(log, "ask")[1]
  assert paragraphs(second[5])[: len(paragraphs(first[5]))] == paragraphs(first[5])
  assert second[5][:1] == first[5] and [turn[0] for turn in second[5]] == ["user", "assistant", "user"]


async def test_as_many_asks_as_there_are_chains_are_in_flight_together() -> None:
  """As many asks as there are chains are in flight together."""
  sand = sown()
  log, root = life(sand)
  two, three = engine.chain("two"), engine.chain("three")
  for one in (root, two, three):
    engine.prompt(int, "work", on=one)
  await settle()
  assert [a[3] for a in said(log, "ask")] == [root, two, three]


async def test_a_later_life_asks_no_model_and_does_no_act_whose_close_the_record_already_holds() -> None:
  """A later life asks no model, and does no act, whose close the record already holds."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  later = Sand(stands=STANDS)
  again, _ = await relived(later, plain(sand.record))
  assert said(again, "ask") == [] and said(again, "start") == []
  assert [a for a in later.calls if a[0] in ("ask", "start")] == []


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
  assert heads(said(log, "ask")[0][5]) == [
    f"#{root} root",
    f"#{root} stands {STANDS!r}",
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
  assert len(said(log, "ask")) == 1
  engine.wake(one)
  await settle()
  assert (await one) == 2 and len(said(log, "ask")) == 2


async def test_a_paused_chain_makes_no_new_ask_after_a_held_response() -> None:
  """A paused chain makes no new ask after a held response."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  one = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  assert len(said(log, "answer")) == 1 and len(said(log, "ask")) == 1
  await settle()
  assert len(said(log, "ask")) == 1 and engine.peek(one) is None


async def test_no_model_is_asked_for_a_rung_the_record_answered() -> None:
  """No model is asked for a rung the record answered, since a later life asks again for nothing it was answered once."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  later = Sand(stands=STANDS)
  again, over = await relived(later, plain(sand.record))
  assert said(again, "ask") == [] and [a for a in later.calls if a[0] == "ask"] == []
  assert engine.modules[over]["a"] == 1


async def test_it_asks_for_no_rung_a_pause_stands_over() -> None:
  """It asks for no rung a pause stands over, whether the pause is over that rung or over the chain, so a paused chain asks no model until the wake."""
  sand = sown()
  log, root = life(sand)
  engine.pause(root)
  sand.script[root] = ["close(1)", "close(None)"]
  one = engine.prompt(int, "count", on=root)
  await settle()
  assert said(log, "ask") == [] and engine.peek(one) is None
  engine.wake(root)
  await settle()
  assert (await one) == 1 and len(said(log, "ask")) == 1


async def test_the_chain_asks_one_model_at_a_time() -> None:
  """The chain asks one model at a time, which it reads from its transcript, the rung that has waited the longest among those it heard on itself that nothing has been said of, handing it the turns as they stand, for the World to hand its provider as it likes, so that many chains ask many models at once."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)", "close(2)"]
  one = engine.prompt(int, "first", on=root)
  two = engine.prompt(int, "second", on=root)
  assert (await one, await two) == (1, 2)
  await settle()
  asks = said(log, "ask")
  assert [a[1] for a in asks] == [a[1] for a in said(log, "rung") if a[2] in (one, two)]
  assert [a[0] for a in log if a[0] in ("ask", "answer")] == ["ask", "answer", "ask", "answer"]
  assert asks[0][5] == engine.turns(on=root)[:1] and asks[1][5] == engine.turns(on=root)[:3]
  side = engine.chain("side")
  mine = engine.prompt(int, "here", on=root)
  theirs = engine.prompt(int, "there", on=side)
  await settle()
  assert engine.peek(mine) is None and engine.peek(theirs) is None
  assert [a[3] for a in said(log, "ask")[2:]] == [root, side]
