"""The World of this machine: the disk, the shell, the operator, a clock, chance, a record file, and an ask.

Each thing the World does has a law of its own here: a command in the directory it names, its streams as they
come, its death at a timeout and at a cancel, what it is fed, a text read and written, a clock and chance of this
machine, a roster bought once, an ask that carries the turns in their roles, the facts the World answers, the
record file, and the operator at its terminal.
"""

import ast
import asyncio
import gc
import inspect
import json
import os
import subprocess
import sys
import time
import warnings
from asyncio.subprocess import Process
from collections.abc import Generator, Sequence
from dataclasses import dataclass
from itertools import pairwise
from pathlib import Path

import pytest
from pydantic_ai.messages import ModelMessage, ModelResponse, TextPart, UserPromptPart
from pydantic_ai.models import Model
from pydantic_ai.models.function import AgentInfo, FunctionModel

from furb import engine
from furb.engine import OPERATOR, Refused
from furb.kernel import Native
from furb.provider.claude import ACTOR, FAMILY, Claude, canon, limits
from furb.world import CAP, SYSTEM, Command, Live, kept, truth, unwire, wire, worded
from outside.doubles import (
  BUILTIN,
  bash,
  broken,
  exited,
  heads,
  life,
  mute,
  read,
  scripted,
  settle,
  speaking,
  verb,
  watched,
  write,
)

TIMEOUT = 600.0
"""TIMEOUT is the timeout of a command that says none, which the bash extension names."""


def world(yard: Path, model: Model[object] | None = None, record: Path | None = None) -> Live:
  """The World of this machine, in the directory of the test, with the model it is asked to buy from, which plays
  the words of the builtins."""
  return Live(str(yard), record=record, model=model, words=BUILTIN)


def commanded(proc: Process | None = None) -> Command:
  """One command of the World, merged and unfed, with the process it is given or with none at all."""
  return Command("bash1", "x", False, TIMEOUT, True, proc)


def runs() -> list[asyncio.Task]:
  """The run of every command the World holds up, which the loop of a life cancels when it ends."""
  return [
    one
    for one in asyncio.all_tasks()
    if one is not asyncio.current_task()
    and inspect.iscoroutine(held := one.get_coro())
    and held.__qualname__ == "Bash.ran"
  ]


async def drained() -> None:
  """Room for the tasks the World still holds to end, since the machine takes a time of its own to grow a process."""
  for _ in range(2000):
    if len(asyncio.all_tasks()) == 1:
      return
    await asyncio.sleep(0.001)


async def test_the_world_answers_what_a_chain_stands_on(yard: Path) -> None:
  """A chain asks what it stands on as it opens, and the World answers with its roster, its directory and its actor."""
  live = world(yard)
  root = life(live)
  await settle()
  standing = engine.ask("stand", root)[1]
  assert isinstance(standing, list)
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


async def test_a_text_is_read_and_written_on_the_disk_it_names(yard: Path) -> None:
  """A read gives the text at a path against the directory the chain stands in, and a write gives it back as it lies."""
  live = world(yard)
  root = life(live)
  assert read(root, "a.txt") == (str(yard / "a.txt"), "one\ntwo\nthree\n")
  assert write(root, "b/c.txt", "kept") == (str(yard / "b" / "c.txt"), "kept")
  assert (yard / "b" / "c.txt").read_text(encoding="utf-8") == "kept"
  assert read(root, str(yard / "a.txt"))[1] == "one\ntwo\nthree\n"


async def test_a_reading_refuses_what_no_text_could_be(yard: Path) -> None:
  """A file that is not there, one past the cap and one that is no utf-8 are each refused, and a door of no act of the
  life the World says nothing of, so nobody answers it and the read gives nothing."""
  live = world(yard)
  root = life(live)
  with pytest.raises(Refused, match="no file at"):
    read(root, "nowhere.txt")
  assert verb("read", root)("mem://x") is None
  assert write(root, "mem://x", "no") == (None, None)
  (yard / "big.txt").write_bytes(b"x" * (CAP + 1))
  with pytest.raises(Refused, match=f"over the {CAP} the World reads"):
    read(root, "big.txt")
  (yard / "raw.txt").write_bytes(b"\xff\xfe\x00")
  with pytest.raises(Refused, match="is no text"):
    read(root, "raw.txt")


async def test_a_relative_directory_stands_against_the_directory_of_the_life(yard: Path) -> None:
  """A pin: a chain holds the path a cd was given, which may name no directory of its own, and the World stands
  such a path against the directory every chain of the life started in, and never against this process."""
  (yard / "sub").mkdir()
  live = world(yard)
  root = life(live)
  assert verb("cd", root)("sub") == "sub"
  assert write(root, "b.txt", "kept") == (str(yard / "sub" / "b.txt"), "kept")
  assert (yard / "sub" / "b.txt").read_text(encoding="utf-8") == "kept"
  assert live.at("sub", "b.txt") == yard / "sub" / "b.txt"
  assert live.at("/elsewhere", "b.txt") == Path("/elsewhere/b.txt")


async def test_a_command_runs_in_the_directory_it_names_and_ends_at_its_timeout(yard: Path) -> None:
  """A command runs where its chain stands, gives its code and its streams, and the World ends it at its timeout."""
  live = world(yard)
  root = life(live)
  assert exited(await bash(root, "echo hi; pwd -P"))[:2] == (0, f"hi\n{yard}\n")
  assert exited(await bash(root, "exit 3"))[0] == 3
  assert exited(await bash(root, "echo said this; sleep 5", timeout=0.3))[:2] == (None, "said this\n")
  assert [one[0] for one in live.calls] == ["stand", "start", "start", "start"]


async def test_a_command_the_machine_will_not_start_is_closed_with_the_refusal(yard: Path) -> None:
  """A command the machine will not start never runs, so the World closes it with why, and whoever waits hears it."""
  live = world(yard)
  root = life(live)
  verb("cd", root)(str(yard / "nowhere"))
  with pytest.raises(Refused, match="did not start"):
    await bash(root, "echo hi")


async def test_stderr_runs_into_stdout_unless_the_command_is_given_a_show_for_it(yard: Path) -> None:
  """Without a show of its own the stderr of a command is its stdout, in the order the command wrote them."""
  live = world(yard)
  root = life(live)
  assert exited(await bash(root, "echo out; echo err >&2"))[1:] == ("out\nerr\n", "")
  apart = await bash(root, "echo out; echo err >&2", show_err=engine.modules[root]["HEAD"])
  assert exited(apart)[1:] == ("out\n", "err\n")


async def test_what_a_command_says_enters_the_record_while_it_runs(yard: Path) -> None:
  """Each part of a stream is an out fact of the World as it arrives, so the door of a command answers while it runs."""
  live = world(yard)
  root = life(live)
  waits = bash(root, "echo one; sleep 0.3; echo two")
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if read(root, f"{waits}/stdout")[1]:
      break
  assert read(root, f"{waits}/stdout")[1] == "one\n"
  assert waits not in engine.outcomes
  assert exited(await waits)[1] == "one\ntwo\n"
  assert read(root, f"{waits}/stdout")[1] == "one\ntwo\n"


async def test_a_cancelled_command_dies_instead_of_running_on(yard: Path) -> None:
  """A cancel ends the command, and the World kills the whole group it grew rather than leave it running."""
  live = world(yard)
  root = life(live)
  waits = bash(root, "sleep 30 & echo up; wait", timeout=60.0)
  # The shell grows the child before it says the word, so what the command said is the one word that the machine has
  # the whole group up, which is what the cancel must kill. A word said before the fork proves nothing: a cancel that
  # lands in that window kills the shell alone, and the child it grows after it holds the stdout of the command open.
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if read(root, f"{waits}/stdout")[1]:
      break
  engine.cancel(waits)
  with pytest.raises(asyncio.CancelledError):
    await waits
  # A child the kill missed holds the stdout of the command open, so the stream never ends and the drain of the
  # World never returns: the tasks of the life fall to the test alone only when the whole group is dead.
  await drained()
  assert asyncio.all_tasks() == {asyncio.current_task()}


async def test_a_command_ends_at_a_cancel_of_its_prompt_and_runs_on_after_a_close_of_it(yard: Path) -> None:
  """The World ends a command only at a control that covers puts over it: a cancel is over everything under the
  prompt it names, so it ends the command that a rung of the prompt made, and a close is over the act it names and
  the words running under it, so the command runs to its end."""
  live = world(yard, scripted(["c = bash('sleep 30')\nawait wait(60)", "c = bash('sleep 0.3; echo late')\nclose(1)"]))
  root = life(live)
  cancelled = engine.prompt(int, "go", on=root)
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if "bash1" in engine.acts:
      break
  engine.cancel(cancelled)
  with pytest.raises(asyncio.CancelledError):
    await engine.Act("bash1")
  assert await engine.prompt(int, "go", on=root) == 1
  assert exited(await engine.Act("bash2"))[:2] == (0, "late\n")


async def test_a_command_still_up_when_its_run_is_cancelled_is_reaped(yard: Path) -> None:
  """The loop of a life that ends cancels the run of a command still up: the World kills the command and reads its
  streams to their end, so no pipe of it outlives the loop and nothing of it warns of itself at the next garbage
  collection."""
  live = world(yard)
  root = life(live)
  waits = bash(root, "sleep 30 & echo up; wait", timeout=60.0)
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if read(root, f"{waits}/stdout")[1]:
      break
  up = runs()
  assert len(up) == 1
  up[0].cancel()
  await asyncio.gather(*up, return_exceptions=True)
  with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    gc.collect()
  assert [str(one.message) for one in caught if issubclass(one.category, ResourceWarning)] == []
  engine.cancel(waits)
  with pytest.raises(asyncio.CancelledError):
    await waits
  await drained()
  assert asyncio.all_tasks() == {asyncio.current_task()}


async def test_a_later_life_runs_a_command_an_earlier_world_left_not_ended_only_at_a_wake(yard: Path) -> None:
  """A command that an earlier World started and that never ended, whether it told anything or not, runs in no later
  life until a wake that life says, and then once; what the command told stands in the door of its command."""
  record = yard / "record.jsonl"
  live = world(yard, scripted([]), record)
  root = life(live)
  # Each command grows its child before it says its word, so the end of the World kills the whole group of it.
  await engine.rung("a = bash('sleep 30 & printf once >> count; wait')\nb = bash('sleep 30 & echo up; wait')", on=root)
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if (yard / "count").is_file() and read(root, "bash2/stdout")[1]:
      break
  up = runs()
  for one in up:
    one.cancel()
  await asyncio.gather(*up, return_exceptions=True)
  for _ in ("the life after the World ended", "the life after that one"):
    live = world(yard, scripted([]), record)
    root = life(live, kept(record))
    await settle()
    assert [one for one in live.calls if one[0] == "start"] == []
    assert "bash1" not in engine.outcomes and "bash2" not in engine.outcomes
    assert read(root, "bash2/stdout")[1] == "up\n"
    assert (yard / "count").read_text(encoding="utf-8") == "once"
  engine.wake(root)
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if (yard / "count").read_text(encoding="utf-8") == "onceonce":
      break
  assert [one[1] for one in live.calls if one[0] == "start"] == ["bash1", "bash2"]
  engine.wake(root)
  await settle()
  assert [one[1] for one in live.calls if one[0] == "start"] == ["bash1", "bash2"]
  assert (yard / "count").read_text(encoding="utf-8") == "onceonce"
  up = runs()
  for one in up:
    one.cancel()
  await asyncio.gather(*up, return_exceptions=True)


async def test_a_command_cancelled_before_its_process_stood_dies_as_soon_as_it_stands(yard: Path) -> None:
  """A cancel that lands before the machine has the command up kills the command where it stands, since the World
  grows the group after the word that ended it."""
  live = world(yard)
  root = life(live)
  # The start of a command is said as the command is made, and the World grows the group in a task after it, so a
  # cancel with no turn of the loop between the two always lands first.
  waits = bash(root, "sleep 30", timeout=60.0)
  engine.cancel(waits)
  with pytest.raises(asyncio.CancelledError):
    await waits
  await drained()
  assert [one[0] for one in live.calls] == ["stand", "start"]


async def test_slaying_what_already_died_harms_nobody(yard: Path) -> None:
  """A command whose process is gone is killed without complaint, and one that never stood is killed too."""
  assert yard.is_dir()
  proc = await asyncio.create_subprocess_shell("exit 0")
  await proc.wait()
  commanded(proc).slay()
  commanded().slay()


async def test_a_fed_command_takes_what_was_written_before_its_process_stood(yard: Path) -> None:
  """A pin: a rung writes the stdin of a command in the step that made it, which is before the World has the process
  up, so what was fed waits and goes in the order it was said once the process stands; a write of nothing closes it."""
  live = world(yard)
  root = life(live)
  waits = bash(root, "cat", fed=True)
  assert write(root, f"{waits}/stdin", "one\n") == (f"{waits}/stdin", "one\n")
  assert write(root, f"{waits}/stdin") == (f"{waits}/stdin", "")
  assert exited(await waits)[:2] == (0, "one\n")
  with pytest.raises(Refused, match="ended"):
    write(root, f"{waits}/stdin", "more")
  deaf = bash(root, "echo hi")
  with pytest.raises(Refused, match="not fed"):
    write(root, f"{deaf}/stdin", "x")
  await deaf


async def test_a_clock_and_a_chance_are_of_this_machine(yard: Path) -> None:
  """The clock of the World is the wall clock, and its chance is a number at least zero and under one."""
  live = world(yard)
  root = life(live)
  was = time.time()
  read = engine.clock(on=root)
  assert was <= read <= time.time()
  drawn = engine.chance(on=root)
  assert 0 <= drawn < 1
  assert [one[0] for one in live.calls] == ["stand", "clock", "chance"]


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


def test_the_engine_the_model_reads_is_the_engine_that_runs() -> None:
  """The system prompt is the engine, minified in layout alone, and nothing else at all."""
  source = Path(engine.__file__).read_text(encoding="utf-8")
  assert ast.dump(ast.parse(SYSTEM)) == ast.dump(ast.parse(source))
  assert "def boot(" in SYSTEM
  assert len(SYSTEM) < len(source)


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


async def test_an_ask_carries_the_system_prompt_and_the_turns_in_their_roles(yard: Path) -> None:
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
    "\n\n#prompt1 count\nprompt1: Act[int] = Act('prompt1')\n\n#rung4 advance on prompt1"
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


async def test_every_turn_an_ask_sent_stands_unchanged_at_every_later_ask(yard: Path) -> None:
  """A pin: the ledger of a ceiling is told at the answer it counts, so no turn an ask already sent grows a line
  after it, and the prefix the provider holds stands from one ask of a chain to the next."""
  seen: list[list[ModelMessage]] = []
  live = world(yard, watched(seen, ["a = 1", "b = 2", "close(3)"]))
  root = life(live)
  verb("grant", root)(usd=9.0)
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  assert [len(one) for one in seen] == [2, 4, 6]
  for before, after in pairwise(seen):
    assert [canon(one) for one in after[: len(before)]] == [canon(one) for one in before]


async def test_an_ask_the_world_cannot_answer_is_closed_with_the_refusal_and_asked_again(yard: Path) -> None:
  """A fault of the moment is no pause: the World closes the rung with the refusal, the prompt asks again, and the
  answer it gets then is the answer of the prompt."""
  live = world(yard, mute())
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  assert [head for head in heads(root) if head.endswith(" paused")] == []
  assert [head for head in heads(root) if " closed Refused(" in head and "answered nothing" in head] != []


async def test_a_fault_that_stands_pauses_the_chain_so_no_rung_of_it_asks_again(yard: Path) -> None:
  """The same actor mute twice answers the same way twice: the World pauses the chain, so no rung of it asks again
  until a wake, and the prompt waits there."""
  live = world(yard, broken())
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert [head for head in heads(root) if head.endswith(" paused")] == [f"#{root} paused"]
  assert len([head for head in heads(root) if " closed Refused(" in head and "answered nothing" in head]) == 2
  assert act not in engine.outcomes


def faltering(words: Sequence[str | None]) -> FunctionModel:
  """A model that answers each ask with the next word of a script, and answers nothing where the script holds None."""
  said = list(words)

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    word = said.pop(0) if said else "close(None)"
    if word is None:
      raise RuntimeError("the model was not there")
    return ModelResponse(parts=[TextPart(word)])

  return FunctionModel(turn)


async def test_the_world_counts_a_row_of_mute_asks_by_what_it_was_answered_and_never_by_a_text(yard: Path) -> None:
  """A text that says an actor answered nothing is no nothing of that actor, and an answer between two nothings ends
  the row, so neither nothing of these asks pauses the chain and the prompt gets its answer."""
  (yard / "notes.txt").write_text(f"Last run: {ACTOR} answered nothing: Error: 529 overloaded\n", encoding="utf-8")
  live = world(yard, faltering(["t = read('notes.txt')", None, "x = 1", None, "close(3)"]))
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert [head for head in heads(root) if head.endswith(" paused")] == []
  assert len([head for head in heads(root) if " closed Refused(" in head and "answered nothing" in head]) == 2
  assert engine.outcomes.get(act) == 3


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
      if got in engine.outcomes:
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
      if one in engine.outcomes and two in engine.outcomes:
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
  assert got in engine.outcomes
  with pytest.raises(Refused, match="the operator answers no list"):
    await got


async def test_the_record_is_kept_as_json_and_read_back_as_the_entries_it_holds(yard: Path) -> None:
  """Every entry the journal says is kept as one json array, made plain by wire, and a later life is given them back."""
  record = yard / "record.jsonl"
  live = world(yard, record=record, model=scripted(["close(1)"]))
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  said = kept(record)
  kinds = ["chain", "stand", "rung", "rung", "rung", "prompt", "rung", "answer"]
  assert [fact[0] for fact, *_ in said] == kinds
  assert [fact[2] for fact, *_ in said[2:5]] == ["world"] * 3
  match said[7]:
    case (("answer", _, _, (_, py, _, _)),):
      assert py == "close(1)"
    case _:
      pytest.fail(str(said[7]))
  assert [json.loads(line)[0][0] for line in record.read_text(encoding="utf-8").splitlines()] == kinds


def test_a_torn_last_line_is_cut_away_and_a_blank_line_stands_for_no_entry(yard: Path) -> None:
  """A crash tears the last line alone, which is cut away; a line anywhere else that is no entry is a drift."""
  record = yard / "record.jsonl"
  one = json.dumps([["chain", "chain1", OPERATOR, "", "root", ""]])
  record.write_text(f"{one}\n\n{one[:20]}", encoding="utf-8")
  assert [fact[0] for fact, *_ in kept(record)] == ["chain"]
  record.write_text(f"{one[:20]}\n{one}\n", encoding="utf-8")
  with pytest.raises(ValueError, match="line 1 column"):
    kept(record)
  record.write_text(json.dumps([1, 2]) + "\n", encoding="utf-8")
  with pytest.raises(engine.Drift, match="is no entry of the record"):
    kept(record)


async def test_a_wait_is_done_when_the_seconds_it_carries_have_passed(yard: Path) -> None:
  """The World says a wait is done when its seconds have passed, and it is over then."""
  live = world(yard)
  root = life(live)
  waits = engine.wait(0.05, on=root)
  assert waits not in engine.outcomes
  assert await waits is None


async def test_the_world_keeps_nothing_when_it_was_given_no_record(yard: Path) -> None:
  """A World that is durable keeps what it is told, and one that is not keeps nothing, and either way the life runs."""
  live = world(yard, scripted(["close(1)"]))
  root = life(live)
  assert await engine.prompt(int, "count", on=root) == 1
  assert live.record is None
  assert not list(yard.glob("*.jsonl"))


async def test_a_stream_that_ends_in_the_middle_of_a_letter_still_tells_what_it_held(yard: Path) -> None:
  """Each stream is decoded as it arrives, so what a command says crosses whole, and a part of a letter at the end
  of it is told as the letter that stands for what could not be read."""
  live = world(yard)
  root = life(live)
  # The octal escape is the one printf of every shell reads, where \xc3 is bash's alone.
  assert exited(await bash(root, r"printf 'a\303'"))[1] == "a�"


async def test_a_command_takes_no_word_at_a_stdin_that_is_not_open() -> None:
  """What is fed to a command before its process stands waits for it, and what is fed to one that was not opened
  fed goes nowhere at all, since the World opened no stdin to write it to."""
  one = commanded()
  one.feed("waits")
  assert one.waiting == ["waits"]
  proc = await asyncio.create_subprocess_shell("exit 0", stdin=subprocess.DEVNULL)
  await proc.wait()
  assert proc.stdin is None
  deaf = commanded(proc)
  deaf.feed("nowhere")
  assert deaf.waiting == []


async def test_the_operator_that_cannot_be_read_closes_the_prompt_with_a_refusal(yard: Path) -> None:
  """A terminal the World cannot read is no operator, and the prompt is closed with what went wrong rather than
  left standing for an answer that can never come."""
  live = world(yard)
  root = life(live)
  got = engine.prompt(str, "say a word", OPERATOR, on=root)
  await settle()
  assert got in engine.outcomes
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
      if got in engine.outcomes:
        break
    with pytest.raises(Refused, match="is no int"):
      await got


@dataclass
class Point:
  """A shape of the suite, which a word may define as it likes."""

  x: int
  y: int | None = None


def test_the_plain_form_of_a_value_leaves_as_json_and_comes_back_whole() -> None:
  """An exception, a shape and plain data leave a life as json through wire, and unwire makes each again by the name
  it is known by, of the engine or of the interpreter; the mark of a shape neither knows comes back as its plain
  fields, as a record of 0.1.0 holds a Text, which the word of an extension makes its value of."""
  held = ("done", "bash1", "world", Point(1), Refused("no"), ValueError("x", 1), {"k": (1, None)})
  plain = json.loads(json.dumps(wire(held)))
  assert plain == [
    "done",
    "bash1",
    "world",
    {"is": "Point", "x": 1, "y": None},
    {"is": "Refused", "args": ["no"]},
    {"is": "ValueError", "args": ["x", 1]},
    {"k": [1, None]},
  ]
  back = unwire(plain)
  assert isinstance(back, list)
  assert back[:3] == ["done", "bash1", "world"]
  assert back[3] == {"is": "Point", "x": 1, "y": None}
  assert isinstance(back[4], Refused) and back[4].args == ("no",)
  assert isinstance(back[5], ValueError) and back[5].args == ("x", 1)
  assert back[6] == {"k": [1, None]}
  old = {"is": "Text", "path": "a", "content": "b", "before": None}
  assert unwire(old) == old


def test_a_map_that_holds_the_key_is_leaves_as_its_pairs_and_comes_back_as_the_map_it_is() -> None:
  """A map that holds the key is leaves a life as its pairs, so unwire makes the map again and never makes a value of
  the name the map holds."""
  held = {"is": "Refused", "args": ["x"], "point": Point(1, 2)}
  plain = json.loads(json.dumps(wire(held)))
  point = {"is": "Point", "x": 1, "y": 2}
  assert plain == {"is": "dict", "args": [[["is", "Refused"], ["args", ["x"]], ["point", point]]]}
  assert unwire(plain) == {**held, "point": point}


async def test_the_plain_form_of_a_whole_record_is_a_fixed_point_of_json(yard: Path) -> None:
  """Every entry a life kept leaves as json through wire and comes back equal, so a record holds nothing that json
  changes: no key that is no string, and no value that is not plain."""
  record = yard / "record.jsonl"
  live = world(yard, record=record, model=scripted(["x = bash('echo hi')\nclose((await x).code)"]))
  root = life(live)
  assert await engine.prompt(int, "run", on=root) == 0
  await settle()
  plain = wire(kept(record))
  assert {"chain", "prompt", "rung", "bash", "answer", "out", "exited"} <= {fact[0] for fact, *_ in kept(record)}
  assert json.loads(json.dumps(plain)) == plain


def note(kept_: list[tuple]) -> Generator[tuple | None, tuple]:
  """An ear of the outside that answers a read of a door of its own, and keeps every fact it hears."""
  while True:
    match a := (yield):
      case ("read", qid, _, _, path) if path.startswith("note://"):
        kept_.append(a)
        yield "done", qid, {"path": path, "content": "kept"}
      case (_, _, _, *_):
        kept_.append(a)


async def test_the_world_says_nothing_of_a_door_of_no_act_so_an_ear_of_the_outside_answers_its_own(yard: Path) -> None:
  """The World refuses a door of an act of the life that lives no more, and says nothing of a path of a scheme, so
  an ear of the outside answers a door of its own whatever the order the ears were given in."""
  live = world(yard)
  heard: list[tuple] = []
  root = engine.boot((), kernel=Native().kernel(), world=live.hears(), note=note(heard))
  live.play()
  assert read(root, "note://one") == ("note://one", "kept")
  assert [a[4] for a in heard if a[0] == "read"] == ["note://one"]
  over = engine.wait(0.0, on=root)
  await over
  with pytest.raises(Refused, match="nothing that lives"):
    read(root, f"{over}/stdout")
  with pytest.raises(Refused, match="nothing that takes a word"):
    write(root, f"{over}/stdin", "late")


async def test_the_world_closes_the_start_of_an_act_that_no_part_does_with_a_refusal(yard: Path) -> None:
  """A start of an act of a kind that neither the World nor any of its parts does is closed with a refusal, so no
  word waits for it."""
  live = world(yard)
  root = life(live)
  job = await engine.rung("close(act('job', __name__, started(ending(idle))))", on=root)
  assert isinstance(job, str)
  with pytest.raises(Refused, match=r"^the World does no job$"):
    await engine.Act(job)
  assert [one for one in live.calls if one[0] == "start"] == [("start", job, job)]


async def test_the_world_plays_its_words_on_a_chain_born_without_a_source_and_on_no_other(yard: Path) -> None:
  """Once the life stands on its record, the World plays its words as the World on each chain born without a source,
  and a chain with a source takes the rungs of its origin instead."""
  live = world(yard)
  root = life(live)
  two = engine.chain("two")
  twin = engine.chain("twin", source=root)
  await settle()
  words = {
    on: [engine.acts[one] for one in engine.acts if one.startswith("rung") and engine.acts[one][3] == on]
    for on in (root, two, twin)
  }
  assert [(a[2], a[4]) for a in words[root]] == [("world", one) for one in BUILTIN]
  assert [(a[2], a[4]) for a in words[two]] == [("world", one) for one in BUILTIN]
  assert [a[2] for a in words[twin]] == [twin] * 3 and all(a[5] for a in words[twin])


async def test_a_record_of_0_1_0_opens(yard: Path) -> None:
  """A record of 0.1.0 holds a text as the mark of its class, which the reader keeps as its plain fields: the life
  opens on it with no drift, and the World plays its words after."""
  said = kept(Path(__file__).parent / "record-0.1.0.jsonl")
  assert said[3][1] == {"is": "Text", "path": "/w/a.txt", "content": "one\ntwo\n"}
  live = world(yard)
  root = life(live, said)
  await settle()
  assert root == "chain1"
  assert read(root, "a.txt") == (str(yard / "a.txt"), "one\ntwo\nthree\n")


async def test_a_path_of_a_chain_that_binds_no_cwd_resolves_in_the_directory_it_stands_on(yard: Path) -> None:
  """The World resolves a path where cwd says the chain stands, and where the standing of the chain says when the
  chain binds no cwd, as a chain that plays no files extension."""
  live = Live(str(yard), words=[])
  root = life(live)
  assert "cwd" not in engine.modules[root]
  _, got = engine.ask("read", root, "a.txt")
  assert got == {"path": str(yard / "a.txt"), "content": "one\ntwo\nthree\n"}
