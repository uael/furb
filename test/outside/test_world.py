"""The World of this machine: the provider of the crate it stands a life on, and the operator at its terminal.

Each thing the World does has a law of its own here: the models it hands the provider, the questions it hears, why
it tells the operator a chain went quiet, the answers a record holds, and the operator at its terminal. The ears of
the crate, the provider and its catalog among them, are proved where the crate writes them.
"""

import sys
from collections.abc import Callable, Sequence
from pathlib import Path

import pytest

from conftest import tilded
from furb import engine
from furb.engine import OPERATOR, Refused
from furb.world import Answer, answered, kept
from furb_monty import _monty
from outside.doubles import broken, life, scripted, settle, speaking, turn, until, watched, world


async def test_the_world_stands_a_life_on_the_provider_of_the_crate_and_its_model_answers_each_reply(
  yard: Path,
) -> None:
  """The World hands the provider of the crate its roster and its actor, and the model it is given answers each
  reply of the life with the request the provider made of it."""
  seen: list[dict] = []
  live = world(yard, watched(seen, ["close(3)"]))
  live.actor = "opus/xhigh"
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 3
  roster, directory, actor, *_ = engine.standing()
  assert [entry[0] for entry in roster] == ["claude-cli:opus", OPERATOR]
  assert (directory, actor) == (tilded(yard), "claude-cli:opus/xhigh")
  assert [one[0] for one in live.calls] == ["stand", "stand", "reply"]
  (request,) = seen
  assert (request["actor"], request["chain"], request["settings"]["effort"]) == ("claude-cli:opus/xhigh", root, "xhigh")
  assert [message["role"] for message in request["messages"]] == ["user", "assistant", "user", "assistant", "user"]
  assert request["messages"][1]["content"][0]["text"].startswith("read(") and "DOCTRINE.md" in str(
    request["messages"][1]
  )
  opened = "\n\n#prompt1\nprompt1_message = 'count'\nprompt1: Act[int] = Act('prompt1')\n\n"
  assert request["messages"][2]["content"][0]["text"].startswith("#read1\n")
  assert request["messages"][3]["content"][0]["text"].startswith("stand()\ncd(")
  first, last = request["messages"][4]["content"]
  assert first["text"].startswith("#stand2\n") and last["text"].startswith(f"#{root}\n")
  assert opened in last["text"]


async def test_the_world_hands_the_provider_a_stream_that_hears_what_a_model_writes(yard: Path) -> None:
  """The World hands the provider its stream, which is told what a model writes as it writes it, under the rung the
  model writes for and the chain of that rung."""
  heard: list[tuple[str, ...]] = []
  live = world(yard, faltering(["close(3)"]))
  live.stream = lambda *said: heard.append(said)
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 3
  (step,) = [a[1] for a in engine.transcript(root) if a[0] == "rung" and a[2] == "prompt1"]
  assert heard == [(step, root, "a part", "")]


def test_the_world_refuses_a_model_that_the_catalog_knows_no_model_by(yard: Path) -> None:
  """A name that the catalog of the crate knows no model by is no model, and the World cannot offer it."""
  live = world(yard, scripted([]))
  live.roster = ["nothing"]
  with pytest.raises(Refused, match=r"No model is nothing\. Name one as provider:id\."):
    live.opened()


async def test_a_fault_that_stands_pauses_the_chain_and_the_world_tells_the_operator_why(
  yard: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  """The provider pauses a chain at the second reply in a row that its actor answered nothing to, and the World
  tells the operator on stderr why the chain went quiet."""
  live = world(yard, broken())
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  told: list[str] = []

  def said() -> bool:
    told.append(capsys.readouterr().err)
    return "is paused" in "".join(told)

  assert await until(said)
  why = "claude-cli:opus/high answered nothing: ProviderError: RuntimeError: the model was not there"
  assert "".join(told) == f"{root} is paused: {why}\n"
  assert engine.paused(root)
  assert engine.peek(act, ...) is ...


def faltering(words: Sequence[str | None]) -> Answer:
  """A model that answers each request with the next word of a script, and answers nothing where the script holds
  None."""
  said = list(words)

  def answer(request: dict, write: Callable[..., None]) -> tuple:
    del request
    write(text="a part")
    word = said.pop(0) if said else "close(None)"
    if word is None:
      why = "the model was not there"
      raise RuntimeError(why)
    return turn(word)

  return answer


async def test_the_answers_a_record_holds_are_the_turns_its_replies_came_to(yard: Path) -> None:
  """The answers a record holds are the done of each reply that came to a turn, and none of a reply that came to a
  refusal."""
  record = yard / "record.jsonl"
  live = world(yard, faltering([None, "close(3)"]))
  root = life(live, record)
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  live.end()
  said = kept(record)
  replies = [fact[1] for (fact,) in said if fact[0] == "reply"]
  assert len(replies) == 2
  assert [(one[1], one[3][1]) for one in answered(said)] == [(replies[1], "close(3)")]


async def test_a_terminal_whose_pipe_the_loop_reads_not_is_read_on_a_thread_that_hands_each_line_to_the_loop(
  yard: Path,
) -> None:
  """A terminal whose pipe the loop reads not, as a file, or every terminal on Windows, is read on a thread that reads
  the lines of the operator alone and hands each to the loop."""
  (at := yard / "answers.txt").write_text("first\n", encoding="utf-8", newline="")
  held, sys.stdin = sys.stdin, at.open(encoding="utf-8")
  try:
    root = life(world(yard))
    got = engine.prompt(str, "the first", OPERATOR, on=root)
    assert await until(lambda: engine.peek(got, ...) is not ...)
    assert await got == "first"
  finally:
    sys.stdin.close()
    sys.stdin = held


async def test_two_prompts_of_the_operator_are_shown_and_answered_one_at_a_time(yard: Path) -> None:
  """There is one terminal and one operator, so prompts of the operator are shown and answered one at a time, in
  the order they asked, and never two at once on one stream."""
  with speaking("first\nsecond\n"):
    root = life(world(yard))
    one = engine.prompt(str, "the first", OPERATOR, on=root)
    two = engine.prompt(str, "the second", OPERATOR, on=root)
    assert await until(lambda: engine.peek(one, ...) is not ... and engine.peek(two, ...) is not ...)
    assert (await one, await two) == ("first", "second")


async def test_a_prompt_of_none_is_closed_as_it_is_taken_and_shown_as_a_notice(
  yard: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  """A prompt of None asks nothing: the World closes it with None as it takes it, reads no line of the operator for
  it, and the terminal shows its message as a notice."""
  root = life(world(yard))
  got = engine.prompt(None, "the build is green", OPERATOR, on=root)
  assert engine.peek(got, ...) is None
  assert await until(lambda: "prompt1 tells: the build is green\n" in capsys.readouterr().out)


async def test_the_world_refuses_a_shape_the_operator_does_not_answer(yard: Path) -> None:
  """Which shapes the operator answers is the law of the World, and one it cannot put to the operator it closes."""
  root = life(world(yard))
  got = engine.prompt(set, "a set please", OPERATOR, on=root)
  await settle()
  assert engine.peek(got, ...) is not ...
  with pytest.raises(Refused, match="the operator answers no set"):
    await got


async def test_the_operator_that_cannot_be_read_closes_the_prompt_with_a_refusal(yard: Path) -> None:
  """A terminal the World cannot read is no operator, and the prompt is closed with what went wrong rather than
  left standing for an answer that can never come."""
  root = life(world(yard))
  got = engine.prompt(str, "say a word", OPERATOR, on=root)
  await settle()
  assert engine.peek(got, ...) is not ...
  with pytest.raises(Refused, match="the operator cannot be read"):
    await got


async def test_a_line_that_is_no_value_of_the_shape_closes_the_prompt_with_a_refusal(yard: Path) -> None:
  """Which shapes the operator answers is the law of the World, and a line that is none of the shape is refused."""
  with speaking("not a number\n"):
    root = life(world(yard))
    got = engine.prompt(int, "a number please", OPERATOR, on=root)
    assert await until(lambda: engine.peek(got, ...) is not ...)
    with pytest.raises(Refused, match="is no int"):
      await got


def test_the_ears_of_the_crate_let_go_at_the_end_of_the_world(yard: Path) -> None:
  """The World lets the ears of the crate go at its end, so the store lets its record go and a later life owns it."""
  live = world(yard)
  record = str(yard / "record.jsonl")
  _, live.ears["store"] = _monty.store(record)
  with pytest.raises(Refused, match="Another process owns"):
    _monty.store(record)
  live.end()
  _, store = _monty.store(record)
  store.dispose()
