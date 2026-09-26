"""The command line: one life for one command of the operator, on a loop of its own.

The three commands are furb's own, so every law here is written for them: a life opened on a record and resumed
from it, a prompt the record already holds taken up rather than asked again, the turns of a root as a model read
them, and a word its caller wrote run on the root.
"""

import asyncio
from pathlib import Path

import pytest

from furb import engine
from furb.cli import SHAPES, again, lived, main, parser, prompted, running, say, turned
from furb.engine import OPERATOR, Act
from furb.world import kept
from outside.doubles import settle, speaking


def held(yard: Path) -> Path:
  """The record of a life of these tests."""
  return yard / "record.jsonl"


async def answered(yard: Path, message: str = "say a word") -> Path:
  """A record that holds one prompt of the operator and the answer the operator gave it at its terminal."""
  record = held(yard)
  with speaking("7\n"):
    world, root, said = lived(record, yard, "opus/low", keeps=True)
    assert said == []
    waits = engine.prompt(int, message, OPERATOR, on=root)
    for _ in range(2000):
      await asyncio.sleep(0.001)
      if engine.peek(waits, ...) is not ...:
        break
    assert await waits == 7
    await settle()
    assert "store" in world.ears
    world.end()
  return record


def test_the_command_line_takes_three_commands_of_the_operator() -> None:
  """A prompt, the turns of a record, and a word its caller wrote, each with the record and the directory it names."""
  whole = parser()
  got = whole.parse_args(["prompt", "count", "--to", "opus/low", "--shape", "int", "--cwd", "/w"])
  assert (got.verb, got.message, got.to, got.shape, str(got.cwd)) == ("prompt", "count", "opus/low", "int", "/w")
  assert got.record is None
  assert whole.parse_args(["turns", "--record", "r.jsonl"]).verb == "turns"
  assert whole.parse_args(["run", "k = 1"]).word == "k = 1"
  assert sorted(SHAPES) == ["bool", "float", "int", "list", "none", "str"]
  assert SHAPES["none"] is None
  assert SHAPES["int"] is int
  with pytest.raises(SystemExit):
    whole.parse_args(["prompt", "count", "--shape", "nothing"])


def test_one_line_to_the_operator(capsys: pytest.CaptureFixture[str]) -> None:
  """The command line says what it has to say on the terminal of the operator, one line at a time."""
  say("a word")
  assert capsys.readouterr().out == "a word\n"


async def test_a_life_is_opened_on_the_record_it_is_given_and_resumed_from_it(yard: Path) -> None:
  """One command is one life: it is opened from the record it is given, and what that record holds it is given."""
  record = await answered(yard)
  world, root, said = lived(record, yard, "opus/low", keeps=True)
  await settle()
  assert root == "chain1"
  assert world.directory == str(yard)
  assert [fact[0] for fact, *_ in kept(record)] == [*[fact[0] for fact, *_ in said], "stand", "done"]
  assert any(fact[0] == "prompt" for fact, *_ in said)
  world.end()


async def test_a_prompt_the_record_already_holds_is_taken_up_and_never_asked_again(yard: Path) -> None:
  """A pin: the engine matches nothing the operator says again, so a life stood up on its own record would open a
  second prompt beside the one that record stands on, and ask a model for what it was answered once."""
  record = await answered(yard)
  world, root, said = lived(record, yard, "opus/low", keeps=True)
  name = again(said, root, int, "say a word", OPERATOR)
  assert name == "prompt1"
  assert await Act(name) == 7
  assert again(said, root, int, "another word", OPERATOR) == ""
  assert again(said, root, str, "say a word", OPERATOR) == ""
  assert again([], root, int, "say a word", OPERATOR) == ""
  world.end()


async def test_a_prompt_of_the_operator_gives_what_the_record_answered(
  yard: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  """The command awaits the prompt on the loop of the operator and gives what it came to, asking no model for a
  prompt the record already holds."""
  record = await answered(yard)
  capsys.readouterr()
  assert await prompted(record, yard, int, "say a word", OPERATOR) == 7


async def test_a_word_its_caller_wrote_runs_on_the_root(yard: Path) -> None:
  """rung is given a word and runs it on a chain in the globals of that chain, and gives back what the word gave."""
  assert await running(None, yard, "k = 1\nclose(k + 1)") == 2


async def test_the_turns_of_a_root_are_printed_as_a_model_read_them(
  yard: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  """The turns of the root of a life made again from its record, each as the python a model reads of it."""
  record = await answered(yard)
  capsys.readouterr()
  await turned(record, yard)
  said = capsys.readouterr().out
  assert said.startswith("[user] ")
  assert "#chain1 root\n" in said
  assert "\n\n#prompt1 say a word\nprompt1: Act[int] = Act('prompt1')\n\n#prompt1 closed 7" in said


async def test_the_turns_of_a_record_run_every_word_again_and_keep_nothing(
  yard: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  """A life that only reads a record keeps nothing, since a World given the record it reads appends to it, and it
  stands whole when boot returns, since the Kernel runs every word of the record again."""
  record = held(yard)
  assert await running(record, yard, "k = 3") is None
  assert await running(record, yard, "close(k + 1)") == 4
  before = record.read_bytes()
  elsewhere = yard / "elsewhere"
  elsewhere.mkdir()
  capsys.readouterr()
  await turned(record, elsewhere)
  assert record.read_bytes() == before
  said = capsys.readouterr().out
  assert "#rung2 closed 4" in said


def test_the_console_script_runs_one_command_of_the_operator(
  yard: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
  """The console script parses one command, opens one life for it on a loop of its own, and prints what it gave."""
  monkeypatch.setattr("sys.argv", ["furb", "run", "close(1 + 1)", "--cwd", str(yard)])
  assert main() is None
  assert capsys.readouterr().out.strip() == "2"
  record = yard / "kept.jsonl"
  monkeypatch.setattr("sys.argv", ["furb", "run", "k = 3", "--record", str(record), "--cwd", str(yard)])
  assert main() is None
  assert record.is_file()
  monkeypatch.setattr("sys.argv", ["furb", "turns", "--record", str(record), "--cwd", str(yard)])
  capsys.readouterr()
  assert main() is None
  assert "[user]" in capsys.readouterr().out


async def test_a_prompt_the_record_does_not_hold_is_put_to_the_actor_it_names(yard: Path) -> None:
  """A prompt of any shape goes to the actor the command names, and the operator is an actor like another."""
  with speaking("said\n"):
    assert await prompted(None, yard, str, "say a word", OPERATOR) == "said"
  with speaking("\n"):
    assert await prompted(None, yard, None, "say nothing", OPERATOR) is None


def test_the_console_script_prompts_the_actor_it_is_given(
  yard: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
  """The console script prompts on the root of the life it opens, and prints what the prompt came to."""
  monkeypatch.setattr(
    "sys.argv", ["furb", "prompt", "say a word", "--to", OPERATOR, "--shape", "str", "--cwd", str(yard)]
  )
  with speaking("a word\n"):
    assert main() is None
  assert capsys.readouterr().out.strip().endswith("'a word'")
