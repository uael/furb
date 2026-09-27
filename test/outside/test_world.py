"""The World of this machine: the models of the provider and the operator at its terminal.

Each thing the World does has a law of its own here: the standing it answers, a roster bought once, a reply that
carries the turns in their roles, a fault of a model that pauses a chain, and the operator at its terminal. The ears
of the crate, the files, the commands, time and the store, are proved where the crate writes them.
"""

import asyncio
import os
import subprocess
import sys
from collections.abc import Sequence
from itertools import pairwise
from pathlib import Path

import pytest
from pydantic_ai.messages import ModelMessage, ModelResponse, TextPart, UserPromptPart
from pydantic_ai.models import Model
from pydantic_ai.models.function import AgentInfo, FunctionModel

from furb import engine
from furb.engine import OPERATOR, Refused
from furb.provider.claude import ACTOR, FAMILY, Claude, canon, limits
from furb.world import SYSTEM, Live, answered, entries, kept, truth, worded
from furb_monty import _monty
from outside.doubles import booted, broken, heads, life, mute, scripted, settle, speaking, watched


def world(yard: Path, model: Model[object] | None = None) -> Live:
  """The World of this machine, in the directory of the test, with the model it is asked to buy from."""
  return Live(str(yard), model=model)


def mutes(root: str) -> list[str]:
  """What each reply of a chain that the World could not answer was done with, in the order of the transcript."""
  return [
    str(a[3])
    for a in engine.transcript(root)
    if a[0] == "done" and a[1].startswith("reply") and isinstance(a[3], Refused)
  ]


async def test_the_world_answers_what_a_chain_stands_on(yard: Path) -> None:
  """A chain asks what it stands on as it opens, and the World answers with its roster, its directory and its actor."""
  live = world(yard)
  root = life(live)
  await settle()
  standing = engine.standing()
  roster, directory, actor = standing
  assert f"#{root} roster {roster!r}" in heads(root)
  told = "\n\n".join(py for role, py, _, _ in engine.turns(on=root) if role == "user")
  assert f"\n\n#{root} roster {roster!r}\n#{root} cwd {directory}\n#{root} actor {actor}\n\n" in f"{told}\n\n"
  assert directory == str(yard)
  assert actor == "opus/low"
  assert isinstance(roster, list)
  assert [name for name, _, _ in roster] == [OPERATOR, "fable", "opus", "sonnet", "haiku"]
  assert live.roster == roster
  assert [one[0] for one in live.calls] == ["stand"]


def test_the_roster_offers_the_claude_family_and_a_name_is_bought_once(yard: Path) -> None:
  """The roster names each model of the family with its window, and a name buys one model of the provider, once."""
  live = world(yard)
  assert [name for name, _, _ in live.roster] == [OPERATOR, *FAMILY]
  assert [window for _, _, window in live.roster[1:]] == [limits(name)[0] for name in FAMILY]
  assert isinstance(live.buys("opus"), Claude)
  assert live.buys("opus") is live.buys("opus")
  assert live.buys("opus") is not live.buys("haiku")
  told = scripted([])
  assert world(yard, told).buys("opus") is told


def test_the_engine_the_model_reads_is_the_engine_of_this_interpreter_whatever_the_switch_says() -> None:
  """A World of a process whose engine runs in monty gives the models the same system prompt, the engine file."""
  said = subprocess.run(
    [sys.executable, "-c", "from furb.world import SYSTEM; print(SYSTEM, end='')"],
    env={**os.environ, "FURB_ENGINE": "monty"},
    capture_output=True,
    text=True,
    check=True,
  ).stdout
  assert said == SYSTEM


async def test_the_world_hands_the_provider_the_python_of_a_user_turn_as_it_is(yard: Path) -> None:
  """The engine phrases everything a model reads, so the World renders nothing: a user turn goes to the provider as
  the python the engine wrote, byte for byte, and a user turn that holds nothing goes not at all."""
  seen: list[list[ModelMessage]] = []
  live = world(yard, watched(seen, ["a = 1", "close(2)"]))
  root = life(live)
  assert await engine.prompt(int, 'say "hi" <b> && \\n', on=root) == 2
  got = engine.turns(on=root)
  said = seen[1]
  assert [part.content for one in said[1::2] if isinstance(part := one.parts[0], UserPromptPart)] == [
    got[0][1],
    got[2][1],
  ]
  assert '#prompt1 say "hi" <b> && \\n' in got[0][1]


def test_the_word_of_a_rung_is_all_the_text_the_model_wrote() -> None:
  """A model speaks python alone, so a fence and the prose around it stay in the word for the gate to refuse."""
  assert worded(ModelResponse(parts=[TextPart("close(1)")])) == "close(1)"
  assert (
    worded(ModelResponse(parts=[TextPart("here:\n```python\nclose(1)\n```\n")])) == "here:\n```python\nclose(1)\n```"
  )
  assert worded(ModelResponse(parts=[TextPart("```\nclose(1)\n```")])) == "```\nclose(1)\n```"


async def test_a_reply_carries_the_system_prompt_and_the_turns_in_their_roles(yard: Path) -> None:
  """The system prompt stands first and is the engine alone, then every turn of the chain in the role it has."""
  seen: list[list[ModelMessage]] = []
  live = world(yard, watched(seen, ["close(1)"]))
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 1
  assert len(seen) == 1
  said = seen[0]
  assert [type(one).__name__ for one in said] == ["ModelRequest", "ModelRequest"]
  assert said[0].parts[0].part_kind == "system-prompt"
  assert said[0].parts[0].content == SYSTEM
  assert said[1].parts[0].part_kind == "user-prompt"
  assert str(said[1].parts[0].content).endswith(
    "\n\n#prompt1 count\nprompt1: Act[int] = Act('prompt1')\n\n#rung1 advance on prompt1"
  )


async def test_an_assistant_turn_is_resent_as_the_parts_the_provider_gave(yard: Path) -> None:
  """An answer goes back to the provider as its own parts, and one that carries none goes back as its text."""
  seen: list[list[ModelMessage]] = []
  live = world(yard, watched(seen, ["a = 1", "close(2)"]))
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 2
  assert len(seen) == 2
  said = seen[1]
  assert [type(one).__name__ for one in said] == ["ModelRequest", "ModelRequest", "ModelResponse", "ModelRequest"]
  assert [one.part_kind for one in said[2].parts] == ["text"]
  first = said[2].parts[0]
  assert isinstance(first, TextPart)
  assert first.content == "a = 1"


async def test_every_turn_a_reply_sent_stands_unchanged_at_every_later_reply(yard: Path) -> None:
  """A pin: the ledger of a ceiling is told at the answer it counts, so no turn a reply already sent grows a line
  after it, and the prefix the provider holds stands from one reply of a chain to the next."""
  seen: list[list[ModelMessage]] = []
  live = world(yard, watched(seen, ["a = 1", "b = 2", "close(3)"]))
  root = life(live)
  engine.grant(usd=9.0, on=root)
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  assert [len(one) for one in seen] == [2, 4, 6]
  for before, after in pairwise(seen):
    assert [canon(one) for one in after[: len(before)]] == [canon(one) for one in before]


async def test_a_reply_the_world_cannot_answer_is_done_with_the_refusal_and_asked_again(yard: Path) -> None:
  """A fault of the moment is no pause: the World says the reply done with the refusal, the prompt asks again, and
  the answer it gets then is the answer of the prompt."""
  live = world(yard, mute())
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  assert [head for head in heads(root) if head.endswith(" paused")] == []
  assert mutes(root) == ["opus/low answered nothing: RuntimeError: the model was not there"]


async def test_a_reply_that_another_ear_ends_asks_its_model_for_nothing_more(yard: Path) -> None:
  """A reply that a cancel ends wants no turn, so the World stops asking the model and says nothing of it after."""
  stopped: list[str] = []

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    try:
      await asyncio.sleep(30)
    except asyncio.CancelledError:
      stopped.append("cancelled")
      raise
    return ModelResponse(parts=[TextPart("close(1)")])

  live = world(yard, FunctionModel(turn))
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  await settle()
  engine.cancel(act)
  await settle()
  dones = [a for a in engine.transcript(root) if a[0] == "done" and engine.question(("reply", a[1]))]
  assert stopped == ["cancelled"] and [(a[2], type(a[3])) for a in dones] == [(dones[0][1], asyncio.CancelledError)]


async def test_a_fault_that_stands_pauses_the_chain_so_no_rung_of_it_asks_again(yard: Path) -> None:
  """The same actor mute twice answers the same way twice: the World pauses the chain, so no rung of it asks again
  until a wake, and the prompt waits there."""
  live = world(yard, broken())
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert [head for head in heads(root) if head.endswith(" paused")] == [f"#{root} paused"]
  assert len(mutes(root)) == 2
  assert engine.peek(act, ...) is ...


def faltering(words: Sequence[str | None]) -> FunctionModel:
  """A model that answers each reply with the next word of a script, and answers nothing where the script holds None."""
  said = list(words)

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    word = said.pop(0) if said else "close(None)"
    if word is None:
      raise RuntimeError("the model was not there")
    return ModelResponse(parts=[TextPart(word)])

  return FunctionModel(turn)


async def test_the_world_counts_a_row_of_mute_replies_by_what_it_was_answered_and_never_by_a_text(yard: Path) -> None:
  """A text that says an actor answered nothing is no nothing of that actor, and an answer between two nothings ends
  the row, so neither nothing of these replies pauses the chain and the prompt gets its answer."""
  (yard / "notes.txt").write_text(f"Last run: {ACTOR} answered nothing: Error: 529 overloaded\n", encoding="utf-8")
  live = world(yard, faltering(["t = read('notes.txt')", None, "x = 1", None, "close(3)"]))
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert [head for head in heads(root) if head.endswith(" paused")] == []
  assert len(mutes(root)) == 2
  assert engine.peek(act) == 3


async def test_the_answers_a_record_holds_are_the_turns_its_replies_came_to(yard: Path) -> None:
  """The answers a record holds are the done of each reply that came to a turn, and none of a reply that came to a
  refusal."""
  record = yard / "record.jsonl"
  stored, store = _monty.store(str(record))
  root = booted(world(yard, faltering([None, "close(3)"])).hears(), entries(stored), gated=False, store=store)
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  store.dispose()
  said = kept(record)
  replies = [fact[1] for (fact,) in said if fact[0] == "reply"]
  assert len(replies) == 2
  assert [(one[1], one[3][1]) for one in answered(said)] == [(replies[1], "close(3)")]


def test_the_operator_answers_the_shapes_the_world_puts_to_it() -> None:
  """A line of the operator becomes a value of the shape the prompt wants, and a line that is neither yes nor no is none."""
  assert truth("YES") is True
  assert truth("n") is False
  with pytest.raises(ValueError, match="neither yes nor no"):
    truth("perhaps")


async def test_the_world_shows_a_prompt_to_the_operator_and_closes_it_with_the_line_it_read(yard: Path) -> None:
  """A pin: the World shows a prompt of the operator on its terminal and reads one line back, on the loop and never
  on a thread, so no read of the operator outlives the life that made it."""
  read, wrote = os.pipe()
  os.write(wrote, b"a word of the operator\n")
  os.close(wrote)
  held, sys.stdin = sys.stdin, os.fdopen(read)
  try:
    live = world(yard)
    root = life(live)
    got = engine.prompt(str, "say a word", OPERATOR, on=root)
    for _ in range(2000):
      await asyncio.sleep(0.001)
      if engine.peek(got, ...) is not ...:
        break
    assert await got == "a word of the operator"
  finally:
    sys.stdin.close()
    sys.stdin = held


async def test_two_prompts_of_the_operator_are_shown_and_answered_one_at_a_time(yard: Path) -> None:
  """There is one terminal and one operator, so prompts of the operator are shown and answered one at a time, in
  the order they asked, and never two at once on one stream."""
  read, wrote = os.pipe()
  os.write(wrote, b"first\nsecond\n")
  os.close(wrote)
  held, sys.stdin = sys.stdin, os.fdopen(read)
  try:
    live = world(yard)
    root = life(live)
    one = engine.prompt(str, "the first", OPERATOR, on=root)
    two = engine.prompt(str, "the second", OPERATOR, on=root)
    for _ in range(2000):
      await asyncio.sleep(0.001)
      if engine.peek(one, ...) is not ... and engine.peek(two, ...) is not ...:
        break
    assert (await one, await two) == ("first", "second")
  finally:
    sys.stdin.close()
    sys.stdin = held


async def test_the_world_refuses_a_shape_the_operator_does_not_answer(yard: Path) -> None:
  """Which shapes the operator answers is the law of the World, and one it cannot put to the operator it closes."""
  live = world(yard)
  root = life(live)
  got = engine.prompt(list, "a list please", OPERATOR, on=root)
  await settle()
  assert engine.peek(got, ...) is not ...
  with pytest.raises(Refused, match="the operator answers no list"):
    await got


async def test_the_operator_that_cannot_be_read_closes_the_prompt_with_a_refusal(yard: Path) -> None:
  """A terminal the World cannot read is no operator, and the prompt is closed with what went wrong rather than
  left standing for an answer that can never come."""
  live = world(yard)
  root = life(live)
  got = engine.prompt(str, "say a word", OPERATOR, on=root)
  await settle()
  assert engine.peek(got, ...) is not ...
  with pytest.raises(Refused, match="the operator cannot be read"):
    await got


async def test_a_line_that_is_no_value_of_the_shape_closes_the_prompt_with_a_refusal(yard: Path) -> None:
  """Which shapes the operator answers is the law of the World, and a line that is none of the shape is refused."""
  with speaking("not a number\n"):
    live = world(yard)
    root = life(live)
    got = engine.prompt(int, "a number please", OPERATOR, on=root)
    for _ in range(2000):
      await asyncio.sleep(0.001)
      if engine.peek(got, ...) is not ...:
        break
    with pytest.raises(Refused, match="is no int"):
      await got
