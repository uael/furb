"""Turn, one item of what a model reads of a transcript."""

from conftest import STANDS, Sand, heads, life, paragraphs, said, settle
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
  """One turn of what a model reads: who said it, the python it holds, what the answer to it cost, and the blocks of the provider, which are its own, are read by nothing of the engine, and go back to it with the turn."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = await spoke(sand, ["a = 1", "close(1)"])
  got = engine.turns(on=root)
  assert got[1] == ("assistant", "a = 1", COST, ["signed 5"])
  assert said(log, "ask")[1][5][1] == said(log, "answer")[0][3]
  assert [type(one[1]).__name__ for one in got] == ["str"] * 5


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


async def test_a_turn_is_a_role_python_a_usage_and_blocks() -> None:
  """A turn is a role, python, a usage and blocks."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = await spoke(sand, ["close(1)"])
  got = engine.turns(on=root)
  assert [len(turn) for turn in got] == [4, 4, 4]
  role, py, usage, blocks = got[0]
  assert (role, usage, blocks) == ("user", None, None)
  assert heads(got[:1]) == ["#chain1 root", f"#chain1 stands {STANDS!r}", "#prompt1 count", "#rung1 advance on prompt1"]
  assert py == "\n\n".join(paragraphs(got[:1]))


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
  assert said(log, "ask")[1][5][1] == answered
  assert said(log, "ask")[1][5][1] == ("assistant", "a = 1", COST, ["signed 5"])


async def test_the_python_of_a_turn() -> None:
  """The python of an assistant turn is the word the model wrote, quotes and all, and the python of a user turn is the paragraphs told since the ask before it, and nothing else."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = await spoke(sand, ["<S1>\nhi\n</S1>\na = S1", "close(len(a) - 2)"])
  got = engine.turns(on=root)
  assert got[1][1] == "<S1>\nhi\n</S1>\na = S1"
  assert got[2][1] == "#rung1 closed\n\n#rung3 advance on prompt1"
  assert got[4][1] == "#prompt1 closed 1"
