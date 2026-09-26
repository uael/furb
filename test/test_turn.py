"""Turn, one item of what a model reads of a transcript."""

from conftest import Sand, born, heads, paragraphs, rows, said, settle
from furb import engine

USAGE = (8000, 30, 0, 0, 1.5)
"""The usage of one answer of the model of these tests."""


async def spoke(*words: str) -> tuple[Sand, list[tuple], str]:
  """A life whose model answers with each word in turn at the usage of these tests, the last of them a close."""
  sand, log, root = born(*words, cost=USAGE)
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  return sand, log, root


async def test_one_turn_of_what_a_model_reads() -> None:
  """One turn of what a model reads: who said it, the python it holds, what the answer to it cost, and the blocks of the provider, which are its own, are read by nothing of the engine, and go back to it with the turn."""
  sand, _, root = await spoke("a = 1", "close(1)")
  got = engine.turns(on=root)
  assert got[1] == ("assistant", "a = 1", USAGE, ["signed 5"])
  assert sand.turns["reply2"][1] == engine.peek("reply1")
  assert [type(one[1]).__name__ for one in got] == ["str"] * 5


async def test_the_role_of_a_turn_is_assistant_for_a_response_and_user_for_everything_else() -> None:
  """The role of a turn is assistant for a response, and user for everything else."""
  _, _, root = await spoke("a = 1", "close(1)")
  assert [role for role, *_ in engine.turns(on=root)] == ["user", "assistant", "user", "assistant", "user"]


async def test_the_blocks_of_an_assistant_turn_are_the_response_as_the_provider_returned_it() -> None:
  """The blocks of an assistant turn are the response as the provider returned it."""
  _, log, root = await spoke("close(1)")
  assert [blocks for *_, blocks in engine.turns(on=root)] == [None, ["signed 8"], None]
  assert [a[3][3] for a in said(log, "done") if a[1] == "reply1"] == [["signed 8"]]


async def test_a_turn_is_a_role_python_a_usage_and_blocks() -> None:
  """A turn is a role, python, a usage and blocks."""
  _, _, root = await spoke("close(1)")
  got = engine.turns(on=root)
  assert [len(turn) for turn in got] == [4, 4, 4]
  role, py, usage, blocks = got[0]
  assert (role, usage, blocks) == ("user", None, None)
  assert heads(got[:1]) == ["#chain1 root", rows("chain1")[0], "#prompt1 count", "#rung1 advance on prompt1"]
  assert py == "\n\n".join(paragraphs(got[:1]))


async def test_an_assistant_turn_carries_the_usage_of_the_response() -> None:
  """An assistant turn carries the usage of the response."""
  _, _, root = await spoke("close(1)")
  assert [usage for _, _, usage, _ in engine.turns(on=root)] == [None, USAGE, None]
  assert engine.turns(on=root)[1][2] == (8000, 30, 0, 0, 1.5)


async def test_an_assistant_turn_keeps_the_role_assistant_for_every_model_that_reads_the_turns() -> None:
  """An assistant turn keeps the role assistant for every model that reads the turns."""
  sand, _, root = await spoke("a = 1", "close(1)")
  assert [role for role, *_ in sand.turns["reply2"]] == ["user", "assistant", "user"]
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert [role for role, *_ in engine.turns(on=twin)] == ["user", "assistant", "user", "assistant", "user"]


async def test_the_engine_sends_the_response_to_the_provider_again_whole() -> None:
  """The engine sends the response to the provider again whole."""
  sand, log, _ = await spoke("a = 1", "close(1)")
  answered = [a[3] for a in said(log, "done") if a[1] == "reply1"]
  assert [sand.turns["reply2"][1]] == answered
  assert sand.turns["reply2"][1] == ("assistant", "a = 1", USAGE, ["signed 5"])


async def test_the_python_of_a_turn() -> None:
  """The python of an assistant turn is the word the model wrote, quotes and all, and the python of a user turn is the paragraphs told since the reply before it, and nothing else."""
  _, _, root = await spoke("<S1>\nhi\n</S1>\na = S1", "close(len(a) - 2)")
  got = engine.turns(on=root)
  assert got[1][1] == "<S1>\nhi\n</S1>\na = S1"
  assert got[2][1] == "#rung3 advance on prompt1"
  assert got[4][1] == "#prompt1 closed 1"
