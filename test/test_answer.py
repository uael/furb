"""Answer, the result of an ask."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD


async def test_the_result_of_an_ask_is_the_turn_of_the_model_which_stands_as_the_turn_it_is() -> None:
  """The result of an ask is the turn of the model, which stands as the turn it is."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  _, _, _, got = said(log, "answer")[0]
  assert got == ("assistant", "close(1)", (0, 0, 0, 0, 0.0), ["signed 8"])
  assert [turn for turn in engine.turns(on=root) if turn[0] == "assistant"] == [got]


async def test_the_world_answers_an_ask_with_an_answer_that_carries_the_response_of_the_provider() -> None:
  """The World answers an ask with an answer that carries the response of the provider."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  (answer,) = said(log, "answer")
  _, about, by, got = answer
  assert by == WORLD and about == said(log, "ask")[0][1]
  assert got[1] == "close(1)"


async def test_the_world_answers_with_the_turn_which_carries_its_usage_and_the_blocks_of_the_provider() -> None:
  """The World answers with the turn, which carries its usage and the blocks of the provider."""
  sand = Sand(stands=STANDS, cost=(80000, 7, 0, 0, 1.5))
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  _, _, _, got = said(log, "answer")[0]
  assert got[2] == (80000, 7, 0, 0, 1.5)
  assert got[3] == ["signed 8"]
