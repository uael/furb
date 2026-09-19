"""Turn, one item of what a model reads of a transcript."""

from conftest import STANDS, Sand, life, said, settle, text_of
from furb import engine

COST = (8000, 30, 0, 0, 1.5)
"""The usage of one answer of the model of these tests."""


async def spoke(sand: Sand, words: list[str]) -> tuple[list[tuple], str]:
  """A life whose model answers with each word in turn, the last of them a close."""
  log, root = life(sand)
  sand.script[root] = words
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  return log, root


async def test_one_turn_of_what_a_model_reads() -> None:
  """One turn of what a model reads: who said it, what it holds, what the answer to it cost, and the blocks of the provider, which are its own, are read by nothing of the engine, and go back to it with the turn."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = await spoke(sand, ["a = 1", "close(1)"])
  got = engine.turns(on=root)
  assert got[1] == ("assistant", ["a = 1"], COST, ["signed 5"])
  assert said(log, "ask")[1][5][1] == said(log, "answer")[0][3]
  assert [type(one).__name__ for one in got[0][1]] == ["tuple"] * len(got[0][1])


async def test_the_role_of_a_turn_is_assistant_for_a_response_and_user_for_everything_else() -> None:
  """The role of a turn is assistant for a response, and user for everything else."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = await spoke(sand, ["a = 1", "close(1)"])
  assert [role for role, *_ in engine.turns(on=root)] == ["user", "assistant", "user", "assistant", "user"]


async def test_the_blocks_of_an_assistant_turn_are_the_response_as_the_provider_returned_it() -> None:
  """The blocks of an assistant turn are the response as the provider returned it."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = await spoke(sand, ["close(1)"])
  assert [blocks for *_, blocks in engine.turns(on=root)] == [None, ["signed 8"], None]
  assert said(log, "answer")[0][3][3] == ["signed 8"]


async def test_a_turn_is_a_role_a_content_a_usage_and_blocks() -> None:
  """A turn is a role, a content, a usage and blocks."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = await spoke(sand, ["close(1)"])
  got = engine.turns(on=root)
  assert [len(turn) for turn in got] == [4, 4, 4]
  role, content, usage, blocks = got[0]
  assert (role, usage, blocks) == ("user", None, None)
  assert [tag[0] for tag in content] == ["opened", "opened", "opened", "opened"]


async def test_an_assistant_turn_carries_the_usage_of_the_response() -> None:
  """An assistant turn carries the usage of the response."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = await spoke(sand, ["close(1)"])
  assert [usage for _, _, usage, _ in engine.turns(on=root)] == [None, COST, None]
  assert engine.turns(on=root)[1][2] == (8000, 30, 0, 0, 1.5)


async def test_an_assistant_turn_keeps_the_role_assistant_for_every_model_that_reads_the_turns() -> None:
  """An assistant turn keeps the role assistant for every model that reads the turns."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = await spoke(sand, ["a = 1", "close(1)"])
  assert [role for role, *_ in said(log, "ask")[1][5]] == ["user", "assistant", "user"]
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert [role for role, *_ in engine.turns(on=twin)] == ["user", "assistant", "user", "assistant", "user"]


async def test_the_engine_sends_the_response_to_the_provider_again_whole() -> None:
  """The engine sends the response to the provider again whole."""
  sand = Sand(stands=STANDS, cost=COST)
  log, _ = await spoke(sand, ["a = 1", "close(1)"])
  answered = said(log, "answer")[0][3]
  assert said(log, "ask")[1][5][1] is answered
  assert said(log, "ask")[1][5][1] == ("assistant", ["a = 1"], COST, ["signed 5"])


async def test_the_text_of_a_turn_is_everything_it_holds_that_is_text() -> None:
  """The text of a turn is everything it holds that is text, which is the whole of what a model says when what it says is python."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = await spoke(sand, ["a = 1\nb = 2", "close(1)"])
  got = engine.turns(on=root)
  assert text_of(got[1]) == "a = 1\nb = 2" and got[1][1] == ["a = 1\nb = 2"]
  assert text_of(got[0]) == ""
