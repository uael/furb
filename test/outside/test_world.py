"""The World of this machine: the disk, the shell, the operator, a clock, chance, a record file, and an ask.

Each thing the World does has a law of its own here: a command in the directory it names, its streams as they
come, its death at a timeout and at a cancel, what it is fed, a text read and written, a clock and chance of this
machine, a roster bought once, an ask that carries the turns in their roles, the facts the World answers, the
record file, and the operator at its terminal.
"""

import ast
import asyncio
import json
import os
import subprocess
import sys
import time
from asyncio.subprocess import Process
from itertools import pairwise
from pathlib import Path

import pytest
from pydantic_ai.messages import ModelMessage, ModelResponse, TextPart
from pydantic_ai.models import Model

from furb import engine
from furb.engine import OPERATOR, TIMEOUT, Exit, Refused, Text
from furb.provider.claude import FAMILY, Claude, canon, limits
from furb.world import CAP, SYSTEM, Command, Live, kept, rendered, shown, truth, unwire, wire, worded
from outside.doubles import broken, life, mute, scripted, settle, speaking, tags, watched


def world(yard: Path, model: Model[object] | None = None, record: Path | None = None) -> Live:
  """The World of this machine, in the directory of the test, with the model it is asked to buy from."""
  return Live(str(yard), record=record, model=model)


def attr(tag: tuple, name: str) -> object:
  """The value of the named attribute of a tag; a tag that holds no such attribute is a failed test."""
  found = [value for key, value in tag[1] if key == name]
  assert len(found) == 1, (tag[0], name, tag[1])
  return found[0]


def commanded(proc: Process | None = None) -> Command:
  """One command of the World, merged and unfed, with the process it is given or with none at all."""
  return Command("bash://operator.1", "chain://operator.1", "x", False, TIMEOUT, True, proc)


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
  got = tags(root, "opened")[1]
  assert attr(got, "directory") == str(yard)
  assert attr(got, "actor") == "opus/low"
  roster = attr(got, "roster")
  assert isinstance(roster, tuple)
  assert [name for name, _, _ in roster] == [OPERATOR, "fable", "opus", "sonnet", "haiku"]
  assert live.roster == roster
  assert [one[0] for one in live.calls] == ["stand"]


async def test_a_text_is_read_and_written_on_the_disk_it_names(yard: Path) -> None:
  """A read gives the text at a path against the directory the chain stands in, and a write gives it back as it lies."""
  live = world(yard)
  root = life(live)
  got = engine.read("a.txt", on=root)
  assert got.content == "one\ntwo\nthree\n"
  assert got.path == str(yard / "a.txt")
  assert engine.write(Text("b/c.txt", "kept"), on=root) == Text(str(yard / "b" / "c.txt"), "kept")
  assert (yard / "b" / "c.txt").read_text(encoding="utf-8") == "kept"
  assert engine.read(str(yard / "a.txt"), on=root).content == "one\ntwo\nthree\n"


async def test_a_reading_refuses_what_no_text_could_be(yard: Path) -> None:
  """A file that is not there, one past the cap, one that is no utf-8, and a door nobody serves are each refused."""
  live = world(yard)
  root = life(live)
  for path, why in (("nowhere.txt", "no file at"), ("mem://x", "the door of nothing that lives")):
    with pytest.raises(Refused, match=why):
      engine.read(path, on=root)
  with pytest.raises(Refused, match="the door of nothing that takes a word"):
    engine.write(Text("mem://x", "no"), on=root)
  (yard / "big.txt").write_bytes(b"x" * (CAP + 1))
  with pytest.raises(Refused, match=f"over the {CAP} the World reads"):
    engine.read("big.txt", on=root)
  (yard / "raw.txt").write_bytes(b"\xff\xfe\x00")
  with pytest.raises(Refused, match="is no text"):
    engine.read("raw.txt", on=root)


async def test_a_relative_directory_stands_against_the_directory_of_the_life(yard: Path) -> None:
  """A pin: a chain holds the path a cd was given, which may name no directory of its own, and the World stands
  such a path against the directory every chain of the life started in, and never against this process."""
  (yard / "sub").mkdir()
  live = world(yard)
  root = life(live)
  assert engine.cd("sub", on=root) == "sub"
  assert engine.write(Text("b.txt", "kept"), on=root) == Text(str(yard / "sub" / "b.txt"), "kept")
  assert (yard / "sub" / "b.txt").read_text(encoding="utf-8") == "kept"
  assert live.at("sub", "b.txt") == yard / "sub" / "b.txt"
  assert live.at("/elsewhere", "b.txt") == Path("/elsewhere/b.txt")


async def test_a_command_runs_in_the_directory_it_names_and_ends_at_its_timeout(yard: Path) -> None:
  """A command runs where its chain stands, gives its code and its streams, and the World ends it at its timeout."""
  live = world(yard)
  root = life(live)
  got = await engine.bash("echo hi; pwd -P", on=root)
  assert got.code == 0
  assert got.stdout.content == f"hi\n{yard}\n"
  assert (await engine.bash("exit 3", on=root)).code == 3
  slow = await engine.bash("echo said this; sleep 5", timeout=0.3, on=root)
  assert (slow.code, slow.stdout.content) == (None, "said this\n")
  assert [one[0] for one in live.calls] == ["stand", "start", "start", "start"]


async def test_a_command_the_machine_will_not_start_is_closed_with_the_refusal(yard: Path) -> None:
  """A command the machine will not start never runs, so the World closes it with why, and whoever waits hears it."""
  live = world(yard)
  root = life(live)
  engine.cd(str(yard / "nowhere"), on=root)
  with pytest.raises(Refused, match="did not start"):
    await engine.bash("echo hi", on=root)


async def test_stderr_runs_into_stdout_unless_the_command_is_given_a_show_for_it(yard: Path) -> None:
  """Without a show of its own the stderr of a command is its stdout, in the order the command wrote them."""
  live = world(yard)
  root = life(live)
  merged = await engine.bash("echo out; echo err >&2", on=root)
  assert (merged.stdout.content, merged.stderr.content) == ("out\nerr\n", "")
  apart = await engine.bash("echo out; echo err >&2", show_err=engine.HEAD, on=root)
  assert (apart.stdout.content, apart.stderr.content) == ("out\n", "err\n")


async def test_what_a_command_says_enters_the_record_while_it_runs(yard: Path) -> None:
  """Each part of a stream is an out fact of the World as it arrives, so the door of a command answers while it runs."""
  live = world(yard)
  root = life(live)
  waits = engine.bash("echo one; sleep 0.3; echo two", on=root)
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if engine.read(f"{waits}/stdout", on=root).content:
      break
  assert engine.read(f"{waits}/stdout", on=root).content == "one\n"
  assert waits not in engine.outcomes
  got = await waits
  assert got.stdout.content == "one\ntwo\n"
  assert engine.read(f"{waits}/stdout", on=root).content == "one\ntwo\n"


async def test_a_cancelled_command_dies_instead_of_running_on(yard: Path) -> None:
  """A cancel ends the command, and the World kills the whole group it grew rather than leave it running."""
  live = world(yard)
  root = life(live)
  waits = engine.bash("sleep 30 & echo up; wait", timeout=60.0, on=root)
  # The shell grows the child before it says the word, so what the command said is the one word that the machine has
  # the whole group up, which is what the cancel must kill. A word said before the fork proves nothing: a cancel that
  # lands in that window kills the shell alone, and the child it grows after it holds the stdout of the command open.
  for _ in range(2000):
    await asyncio.sleep(0.001)
    if engine.read(f"{waits}/stdout", on=root).content:
      break
  engine.cancel(waits)
  with pytest.raises(asyncio.CancelledError):
    await waits
  # A child the kill missed holds the stdout of the command open, so the stream never ends and the drain of the
  # World never returns: the tasks of the life fall to the test alone only when the whole group is dead.
  await drained()
  assert asyncio.all_tasks() == {asyncio.current_task()}


async def test_a_command_cancelled_before_its_process_stood_dies_as_soon_as_it_stands(yard: Path) -> None:
  """A cancel that lands before the machine has the command up kills the command where it stands, since the World
  grows the group after the word that ended it."""
  live = world(yard)
  root = life(live)
  # The start of a command is said as the command is made, and the World grows the group in a task after it, so a
  # cancel with no turn of the loop between the two always lands first.
  waits = engine.bash("sleep 30", timeout=60.0, on=root)
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
  waits = engine.bash("cat", fed=True, on=root)
  assert engine.write(Text(f"{waits}/stdin", "one\n"), on=root) == Text(f"{waits}/stdin", "one\n")
  assert engine.write(Text(f"{waits}/stdin"), on=root) == Text(f"{waits}/stdin")
  got = await waits
  assert (got.code, got.stdout.content) == (0, "one\n")
  with pytest.raises(Refused, match="takes no word"):
    engine.write(Text(f"{waits}/stdin", "more"), on=root)
  deaf = engine.bash("echo hi", on=root)
  with pytest.raises(Refused, match="is not fed"):
    engine.write(Text(f"{deaf}/stdin", "x"), on=root)
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


def test_what_the_model_reads_of_a_tag_is_its_name_its_attributes_and_its_body() -> None:
  """A tag crosses as a block named by its name, its short attributes beside the name, and everything else inside.

  Nothing is escaped, so a text crosses to the model byte for byte, and a value that holds a line break or a
  quotation mark stands in the body rather than beside the name.
  """
  assert shown(("cwd", [("path", "/w")], None)) == '<cwd path="/w"/>'
  assert shown(("closed", [("id", "rung://x")], "3")) == '<closed id="rung://x">\n3\n</closed>'
  assert shown(("ledger", [("spent", 1.5)], None)) == '<ledger spent="1.5"/>'
  assert shown(("read", [("path", 'say "hi"')], None)) == '<read>\n<path>\nsay "hi"\n</path>\n</read>'
  assert shown(("out", [("text", "a > b && c\n")], None)) == "<out>\n<text>\na > b && c\n\n</text>\n</out>"
  assert shown(("read", [], [("shown", [("path", "f")], "1 a")])) == (
    '<read>\n<shown path="f">\n1 a\n</shown>\n</read>'
  )
  assert shown(("read", [], [(Text("f", "a"), engine.HEAD)])).startswith("<read>\n(Text(")
  # A body is any value a tag was told with, and a tag of an extension may hold one of a kind this World has no
  # reading of, which stands as python shows it rather than going missing.
  assert shown(("found", [], Text("a.txt", "hi"))) == "<found>\nText(path='a.txt', content='hi', before=None)\n</found>"
  assert shown(("found", [], 3)) == "<found>\n3\n</found>"
  assert rendered([("cwd", [("path", "/w")], None), "said"]) == '<cwd path="/w"/>\nsaid'
  assert rendered([]) == ""


def test_the_word_of_a_rung_is_the_code_of_the_answer_or_the_whole_of_it() -> None:
  """A model that fences its code says the block alone; one that fences nothing says everything it wrote."""
  assert worded(ModelResponse(parts=[TextPart("close(1)")])) == "close(1)"
  assert worded(ModelResponse(parts=[TextPart("here:\n```python\nclose(1)\n```\n")])) == "close(1)"
  assert worded(ModelResponse(parts=[TextPart("```\nclose(1)\n```")])) == "close(1)"
  assert worded(ModelResponse(parts=[TextPart("```\na\n```\n```\nb\n```")])) == "```\na\n```\n```\nb\n```"


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
  assert "opened" in str(said[1].parts[0].content)


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
  """A pin: the ledger of a ceiling is told at the answer it counts, so no turn an ask already sent grows a tag
  after it, and the prefix the provider holds stands from one ask of a chain to the next."""
  seen: list[list[ModelMessage]] = []
  live = world(yard, watched(seen, ["a = 1", "b = 2", "close(3)"]))
  root = life(live)
  engine.grant(usd=9.0, on=root)
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
  assert tags(root, "paused") == []
  assert [str(one[2]) for one in tags(root, "closed") if "answered nothing" in str(one[2])] != []


async def test_a_fault_that_stands_pauses_the_chain_so_no_rung_of_it_asks_again(yard: Path) -> None:
  """The same actor mute twice answers the same way twice: the World pauses the chain, so no rung of it asks again
  until a wake, and the prompt waits there."""
  live = world(yard, broken())
  root = life(live)
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert [one[0] for one in tags(root, "paused")] == ["paused"]
  assert len([one for one in tags(root, "closed") if "answered nothing" in str(one[2])]) == 2
  assert act not in engine.outcomes


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
  assert [fact[0] for _, fact, *_ in said] == ["chain", "prompt", "rung", "answer"]
  match said[3]:
    case (_, ("answer", _, _, (_, content, _, _))):
      assert [one for one in content if isinstance(one, str)] == ["close(1)"]
    case _:
      pytest.fail(str(said[3]))
  assert [json.loads(line)[1][0] for line in record.read_text(encoding="utf-8").splitlines()] == [
    "chain",
    "prompt",
    "rung",
    "answer",
  ]


def test_a_torn_last_line_is_cut_away_and_a_blank_line_stands_for_no_entry(yard: Path) -> None:
  """A crash tears the last line alone, which is cut away; a line anywhere else that is no entry is a drift."""
  record = yard / "record.jsonl"
  one = json.dumps(["", ["chain", "chain://operator.1", OPERATOR, "", "root", ""]])
  record.write_text(f"{one}\n\n{one[:20]}", encoding="utf-8")
  assert [fact[0] for _, fact, *_ in kept(record)] == ["chain"]
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
  got = await engine.bash(r"printf 'a\303'", on=root)
  assert got.stdout.content == "a�"


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


def test_the_plain_form_of_a_value_leaves_as_json_and_comes_back_whole() -> None:
  """An exception, a text, a shape and plain data leave a life as json through wire, and unwire makes each again by
  the name it is known by, of the engine or of the interpreter."""
  exit_ = Exit(0, Text("bash://operator.1/stdout", "hi\n"), Text("bash://operator.1/stderr"))
  held = ("done", "bash://operator.1", "world", exit_, Refused("no"), ValueError("x", 1), {"k": (1, None)})
  plain = json.loads(json.dumps(wire(held)))
  assert plain == [
    "done",
    "bash://operator.1",
    "world",
    {
      "is": "Exit",
      "code": 0,
      "stdout": {"is": "Text", "path": "bash://operator.1/stdout", "content": "hi\n"},
      "stderr": {"is": "Text", "path": "bash://operator.1/stderr", "content": ""},
    },
    {"is": "Refused", "args": ["no"]},
    {"is": "ValueError", "args": ["x", 1]},
    {"k": [1, None]},
  ]
  back = unwire(plain)
  assert isinstance(back, list)
  assert back[:3] == ["done", "bash://operator.1", "world"]
  assert back[3] == Exit(0, Text("bash://operator.1/stdout", "hi\n"), Text("bash://operator.1/stderr"))
  assert isinstance(back[4], Refused) and back[4].args == ("no",)
  assert isinstance(back[5], ValueError) and back[5].args == ("x", 1)
  assert back[6] == {"k": [1, None]}
  assert wire(Text("a", "b", Text("a", "c"))) == {"is": "Text", "path": "a", "content": "b"}
