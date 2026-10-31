"""bash, a command of the World."""

import os
import shlex
import signal
import sys
import time
from asyncio import CancelledError
from collections.abc import Generator
from pathlib import Path

import pytest

from conftest import (
  MANY,
  WORLD,
  Sand,
  Where,
  acts,
  born,
  chained,
  dones,
  life,
  machine,
  paragraphs,
  plain,
  relived,
  said,
  settle,
  world_says,
)
from furb import engine
from furb.engine import HEAD, HIDDEN, OPERATOR, TAIL, Act, Exit, Refused, Text, grep, span
from furb_monty import _monty


def door(one: str, part: str) -> str:
  """The door of one stream of a command, or of its stdin."""
  return f"{one}/{part}"


async def test_a_command_its_streams_as_they_come_its_exit_and_its_doors() -> None:
  """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to."""
  sand, _, root = born(auto=False)
  one = engine.bash("run", stdin=None, show_err=TAIL, on=root)
  assert engine.write(Text(door(one, "stdin"), "go"), on=root) == Text(door(one, "stdin"), "go")
  assert sand.fed == ["go"]
  world_says("out", one, "half\n", "stdout")
  world_says("out", one, "bad\n", "stderr")
  await settle()
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"
  assert engine.read(door(one, "stderr"), on=root).content == "bad\n"
  sand.exits(one, 2)
  assert await one == Exit(2, Text(door(one, "stdout"), "half\n"), Text(door(one, "stderr"), "bad\n"))


async def test_bash_is_given_one_show_for_each_stream_that_bash_tells() -> None:
  """bash is given one show for each stream that bash tells."""
  sand, _, root = born(auto=False)
  one = engine.bash("run", show=span(2, 2), show_err=grep("^b"), on=root)
  world_says("out", one, "one\ntwo\nthree\n", "stdout")
  world_says("out", one, "a\nb\n", "stderr")
  sand.exits(one, 0)
  await one
  assert paragraphs(engine.turns(on=root))[-1] == (f"#{one} exited 0\n{one}_stdout = 'two'\n{one}_stderr = 'b'")


async def test_the_commands_of_the_world_run_at_the_same_time() -> None:
  """The commands of the World run at the same time."""
  sand, _, root = born(auto=False)
  one, two = engine.bash("first", on=root), engine.bash("second", on=root)
  await settle()
  sand.exits(two, 1)
  await settle()
  done = engine.peek(two)
  assert isinstance(done, Exit) and done.code == 1 and engine.peek(one) is None
  sand.exits(one, 0)
  await settle()
  assert ((await one).code, (await two).code) == (0, 1)


async def test_bash_is_given_a_command_its_stdin_a_timeout_the_codes_that_it_accepts_and_a_show_for_each_stream() -> (
  None
):
  """bash is given a command, its stdin, a timeout, the codes that it accepts, and a show for each stream, the plain words first, so the journal makes it again."""
  sand, log, root = born(auto=False)
  engine.bash("echo hi", "go", 5.0, (0, 1), HEAD, TAIL, on=root)
  word = said(log, "bash")[0]
  assert word == ("bash", word[1], OPERATOR, root, "echo hi", "go", 5.0, (0, 1))
  assert [e[0] for e in sand.record if e[0][0] == "bash"] == [word]
  again, _ = await relived(Sand(auto=False), plain(sand.record))
  assert said(again, "bash") == [word]


async def test_bash_gives_the_command_which_is_awaited_for_its_exit_code_and_its_streams() -> None:
  """bash gives the command, which is awaited for its exit code and its streams."""
  _, _, root = born()
  one = engine.bash("echo hi", on=root)
  assert isinstance(one, Act) and one == "bash1"
  got = await one
  assert isinstance(got, Exit) and (got.code, got.stdout.content) == (0, "ran echo hi\n")


async def test_the_world_runs_the_command_in_the_working_directory_of_the_chain_which_it_reads() -> None:
  """The World runs the command in the working directory of the chain, which it reads."""
  sand = Where()
  _, root = life(sand)
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/w"]
  engine.cd("/deep", on=root)
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/w", "/deep"]


async def test_the_command_runs_until_it_ends_until_its_timeout_or_until_a_cancel() -> None:
  """The command runs until it ends, until its timeout, or until a cancel."""
  sand, _, root = born()
  assert (await engine.bash("echo hi", on=root)).code == 0
  sand.auto = False
  assert (await engine.bash("forever", timeout=0, on=root)).code is None
  gone = engine.bash("slow", on=root)
  await settle()
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.peek(gone), CancelledError)


async def test_the_world_ends_the_command_at_its_timeout() -> None:
  """The World ends the command at its timeout."""
  _, log, root = born(auto=False)
  one = engine.bash("forever", timeout=0, on=root)
  await settle()
  assert [(a[1], a[2]) for a in dones(log, "bash")] == [(one, WORLD)]
  assert engine.peek(one) == Exit(None, Text(door(one, "stdout")), Text(door(one, "stderr")))


async def test_the_command_completes_with_its_exit_code_and_its_streams() -> None:
  """The command completes with its exit code and its streams."""
  _, _, root = born()
  one = engine.bash("echo hi", on=root)
  assert await one == Exit(0, Text(door(one, "stdout"), "ran echo hi\n"), Text(door(one, "stderr"), ""))


async def test_the_exit_code_of_the_command_is_none_after_a_timeout() -> None:
  """The exit code of the command is None after a timeout."""
  _, _, root = born(auto=False)
  assert (await engine.bash("forever", timeout=0, on=root)).code is None


async def test_a_signal_that_ends_the_command_gives_the_code_128_and_the_number_of_the_signal(tmp_path: Path) -> None:
  """A signal that ends the command gives the code 128 and the number of the signal, as a shell gives, so the code None says that the timeout ended it."""
  live, root = machine(tmp_path)
  got = await engine.bash("kill -KILL $$", on=root)
  late = await engine.bash("echo before; sleep 30", timeout=0.5, on=root)
  live.end()
  assert (got.code, late.code, late.stdout.content) == (137, None, "before\n")


async def test_the_command_ends_once_its_shell_exited_and_its_streams_closed(tmp_path: Path) -> None:
  """The command ends once its shell exited and its streams closed, or once its shell exited and a short drain passed with a stream open, as when a process that the command started holds a stream, with the code of the shell and what its streams wrote."""
  live, root = machine(tmp_path)
  begun = time.monotonic()
  held = await engine.bash("echo before; sleep 20 & echo $!", on=root)
  took = time.monotonic() - begun
  if sys.platform != "win32":
    os.kill(int(held.stdout.content.splitlines()[-1]), signal.SIGKILL)
  assert (held.code, held.stdout.content.startswith("before\n")) == (0, True)
  assert took < 10
  if sys.platform == "win32":
    live.end()
    return
  begun = time.monotonic()
  left = await engine.bash("perl -e 'use POSIX; setsid(); sleep 120' & echo $!", on=root)
  took = time.monotonic() - begun
  os.kill(int(left.stdout.content), signal.SIGKILL)
  live.end()
  assert left.code == 0
  assert took < 10


async def test_a_command_that_its_timeout_ended_tells_timed_out_and_its_timeout_in_its_header() -> None:
  """A command that its timeout ended tells timed out and its timeout in its header, as #bash1 timed out after 5.0s, with what it wrote before."""
  _, _, root = born(auto=False)
  one = engine.bash("forever", timeout=0.0, on=root)
  world_says("out", one, "before\n", "stdout")
  assert (await one).code is None
  assert paragraphs(engine.turns(on=root))[-1] == f"#{one} timed out after 0.0s\n{one}_stdout = 'before'"


async def test_a_stdin_of_text_is_fed_whole_to_the_command_at_its_start_and_then_it_closes() -> None:
  """A stdin of text is fed whole to the command at its start, and then it closes, so the default, an empty text, gives a stdin that is empty and closed."""
  # The World feeds the text and closes the stdin, which the test of the crate
  # a_command_is_fed_a_stdin_of_text_whole_at_its_start_and_then_its_stdin_closes proves; the engine carries the text
  # to the World in the words of the act and says no feed of it.
  _, log, root = born(auto=False)
  engine.bash("git commit -F -", stdin="a message\n", on=root)
  engine.bash("second", on=root)
  await settle()
  assert [a[5] for a in said(log, "bash")] == ["a message\n", ""]
  assert said(log, "feed") == []


async def test_without_a_stderr_show_the_stderr_of_the_command_flows_into_its_stdout() -> None:
  """Without a stderr show, the stderr of the command flows into its stdout."""
  # The engine answers the World that asks whether the stderr of a command flows into its stdout, and the World runs
  # the command so, which the test of the crate the_world_writes_the_stderr_of_a_merged_command_into_its_stdout proves.
  _, log, root = born(auto=False)
  one, two = engine.bash("run", on=root), engine.bash("run", show_err=TAIL, on=root)
  await settle()
  assert {a[4]: engine.peek(a[1]) for a in acts(log).values() if a[0] == "merged"} == {one: True, two: False}


async def test_a_command_without_a_timeout_has_the_timeout_timeout_that_the_file_names() -> None:
  """A command without a timeout has the timeout TIMEOUT that the file names."""
  _, log, root = born(auto=False)
  engine.bash("run", on=root)
  assert engine.TIMEOUT == 120.0
  assert [a[6] for a in said(log, "bash")] == [engine.TIMEOUT]


async def test_a_stdin_of_none_stays_open_and_the_stdin_door_is_writable_while_the_command_runs() -> None:
  """A stdin of None stays open, and the door bash1/stdin is writable while the command runs."""
  sand, _, root = born(auto=False)
  one = engine.bash("run", stdin=None, on=root)
  assert one == "bash1"
  await settle()
  assert engine.write(Text("bash1/stdin", "go"), on=root) == Text("bash1/stdin", "go")
  assert engine.write(Text("bash1/stdin", "more"), on=root) == Text("bash1/stdin", "more")
  assert sand.fed == ["go", "more"]


async def test_the_world_feeds_a_command_the_text_of_each_write_of_its_stdin_that_it_answers() -> None:
  """The World feeds a command the text of each write of its stdin that it answers, and a write of nothing closes that stdin."""
  sand, _, root = born(auto=False)
  one = engine.bash("cat", stdin=None, on=root)
  await settle()
  assert engine.write(Text(door(one, "stdin"), "a"), on=root) == Text(door(one, "stdin"), "a")
  engine.write(Text(door(one, "stdin")), on=root)
  assert sand.fed == ["a", None] and sand.shut == {one}


async def test_the_stdout_and_the_stderr_doors_are_readable_while_the_command_runs_and_after_it() -> None:
  """The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it: a read gives that stream of the Exit once the command exited, and otherwise the out facts of that stream, so the doors of a command that a crash cut give what it wrote before the crash."""
  _, _, top = born(auto=False)
  shown = engine.bash("run", show_err=TAIL, on=top)
  assert shown == "bash1"
  world_says("out", shown, "one\n", "stdout")
  world_says("out", shown, "bad\n", "stderr")
  await settle()
  assert engine.read("bash1/stdout", on=top) == Text("bash1/stdout", "one\n")
  assert engine.read("bash1/stderr", on=top) == Text("bash1/stderr", "bad\n")
  engine.cancel(shown)
  await settle()
  assert isinstance(engine.peek(shown), CancelledError)
  assert engine.read("bash1/stdout", on=top) == Text("bash1/stdout", "one\n")
  sand, _, root = born(auto=False)
  one = engine.bash("run", on=root)
  await settle()
  sand.writes(one, "first\n", "stdout")
  await settle()
  _, over = await relived(Sand(auto=False), list(sand.record))
  engine.wake(over)
  await settle()
  assert isinstance(engine.peek(one), Refused)
  assert engine.read(door(one, "stdout"), on=over) == Text(door(one, "stdout"), "first\n")


async def test_a_wake_on_a_chain_with_a_source_starts_no_inherited_command_again() -> None:
  """A wake on a chain with a source starts no inherited command again, since only the owner starts an act."""
  sand, log, root = born("x = bash('slow')\nclose(1)", auto=False)
  assert await engine.prompt(int, "start one", on=root) == 1
  command = said(log, "bash")[0][1]
  twin = await chained("twin", root)
  engine.pause(twin)
  engine.wake(twin)
  await settle()
  assert [a[1] for a in sand.calls if a[0] == "bash"] == [command]
  assert [a[1] for a in said(log, "bash")] == [command]


async def test_the_world_takes_it_once_and_closes_a_command_that_a_crash_cut_as_interrupted_at_a_wake() -> None:
  """The World takes it once: a later life never runs it again, and at a wake the World closes a command that the record shows started and not ended with a refusal that says the command was interrupted, so the model sees that outcome and runs the command again if it wants."""
  sand, _, root = born()
  one = engine.bash("echo hi", on=root)
  await one
  await settle()
  assert [a[1] for a in sand.calls if a[0] == "bash"] == [one]
  later = Sand()
  _, over = await relived(later, plain(sand.record))
  engine.wake(over)
  await settle()
  assert [a for a in later.calls if a[0] == "bash"] == [] and over == root
  got = engine.peek(one)
  assert isinstance(got, Exit) and got.code == 0
  quiet, _, root = born(auto=False)
  two = engine.bash("sleep 9", on=root)
  await settle()
  third = Sand()
  heard, over = await relived(third, plain(quiet.record))
  assert said(third.calls, "bash") == [] and engine.peek(two) is None
  engine.wake(over)
  await settle()
  got, why = engine.peek(two), "'sleep 9' was interrupted: the life that ran it ended before it did"
  assert isinstance(got, Refused) and str(got) == why
  assert third.outs == {} and [a[1] for a in heard if a[0] == "close"] == [two]
  assert f"#{two} closed by world\n{two}_value = Refused({why!r})" in paragraphs(engine.turns(on=over))


async def test_with_a_hidden_show_it_tells_its_header_and_its_binding_alone() -> None:
  """With a hidden show it tells its header and its binding alone, and not its command and not its close."""
  sand, _, root = born(auto=False)
  one = engine.bash("run", on=root)
  quiet = engine.bash("quiet", show=HIDDEN, on=root)
  sand.exits(quiet, 0)
  await quiet
  assert paragraphs(engine.turns(on=root))[1:] == [
    f"#{one}\n{one}_command = 'run'\n{one}: Act[Exit] = Act({one!r})",
    f"#{quiet}\n{quiet}: Act[Exit] = Act({quiet!r})",
  ]


async def test_the_chain_that_a_read_of_a_door_of_the_command_is_on_answers_it() -> None:
  """The chain that a read of a door of the command is on answers that read, while the command runs and after it, and the World answers a write of a door of it, so the ear of the command lives only until it hears the done of the command, and a cancel ends the command as it ends every other act."""
  sand, log, root = born(auto=False)
  other = engine.chain("other")
  one = engine.bash("run", stdin=None, on=root)
  world_says("out", one, "half\n", "stdout")
  await settle()
  assert engine.read(door(one, "stdout"), on=other).content == "half\n"
  assert engine.write(Text(door(one, "stdin"), "go"), on=other) == Text(door(one, "stdin"), "go")
  sand.exits(one, 0)
  got = await one
  await settle()
  engine.drive(engine.idle(one), one)
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"
  with pytest.raises(Refused, match="takes no write"):
    engine.write(Text(door(one, "stdin"), "late"), on=other)
  two = engine.bash("slow", on=root)
  world_says("out", two, "part\n", "stdout")
  engine.cancel(two)
  await settle()
  engine.drive(engine.idle(two), two)
  assert engine.read(door(two, "stdout"), on=other).content == "part\n"
  assert (engine.peek(one), type(engine.peek(two))) == (got, CancelledError)
  doors = {a[1]: a[3] for a in said(log, "read") if "/std" in str(a[4])}
  assert {(doors[a[1]], a[2]) for a in said(log, "done") if a[1] in doors} == {(root, root), (other, other)}
  writes = [a[1] for a in said(log, "write")]
  assert {a[2] for a in said(log, "done") if a[1] in writes} == {WORLD} and sand.fed == ["go"]


async def test_a_read_of_a_door_of_a_command_that_an_ear_makes_while_the_chain_speaks_finds_no_ear() -> None:
  """A read of a door of a command that an ear makes while the chain of that read speaks finds no ear of the engine that answers it, as a read of the door of a prompt finds none, since an ear that speaks hears nothing until it is done."""
  _, _, root = born(auto=False)
  one = engine.bash("run", on=root)
  world_says("out", one, "half\n", "stdout")
  await settle()
  got: list[object] = []

  def watch() -> Generator[None, tuple]:
    while True:
      a = yield
      if a[0] == "run" and not got:
        for path in (door(one, "stdout"), "prompt1"):
          try:
            got.append(engine.read(path, on=root))
          except Refused as no:
            got.append(str(no))

  engine.drive(watch(), "watch")
  asked = engine.prompt(str, "go", on=root)
  await engine.rung("x = 1", on=root)
  assert got == ["nothing takes read", "nothing takes read"]
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"
  assert engine.read(asked, on=root).content == ""


async def test_it_runs_until_it_ends_until_its_timeout_or_until_a_cancel() -> None:
  """It runs until it ends, until its timeout or until a cancel: the World is the one that ends it, at the timeout it reads off the act as at a cancel, since the World is the one running it, and the engine says nothing to make it."""
  sand, log, root = born(auto=False)
  late = engine.bash("forever", timeout=0, on=root)
  await settle()
  gone = engine.bash("slow", on=root)
  await settle()
  engine.cancel(gone)
  await settle()
  assert {a[1]: (a[2], type(a[3]).__name__) for a in dones(log, "bash")} == {
    late: (WORLD, "Exit"),
    gone: (gone, "CancelledError"),
  }
  assert engine.peek(late) == Exit(None, Text(door(late, "stdout")), Text(door(late, "stderr"))) and sand.outs == {}
  assert said(log, "wait") == []


async def test_a_pause_stops_no_command() -> None:
  """A pause stops no command: it runs on, the World says it done when it ends, and its ear tells its exit at the wake."""
  sand, log, root = born(auto=False)
  one = engine.bash("run", on=root)
  await settle()
  engine.pause(root)
  world_says("out", one, "half\n", "stdout")
  sand.exits(one, 0)
  await settle()
  assert [a[2] for a in dones(log, "bash")] == [WORLD] and paragraphs(engine.turns(on=root))[
    -1
  ] == f"#{root} paused by operator"
  engine.wake(root)
  await settle()
  assert paragraphs(engine.turns(on=root))[-1] == f"#{one} exited 0\n{one}_stdout = 'half'"


async def test_the_stdout_of_a_command_without_a_show_is_told_as_tail() -> None:
  """The stdout of a command without a show is told as TAIL, which is the span of its last 250 lines."""
  lines = MANY.splitlines()
  assert engine.TAIL(lines) == span(-250, -1)(lines) == list(range(51, 301))
  sand, _, root = born(auto=False)
  one = engine.bash("run", on=root)
  world_says("out", one, MANY, "stdout")
  sand.exits(one, 0)
  await one
  told = "\n".join(f"line {i}" for i in range(51, 301))
  assert paragraphs(engine.turns(on=root))[-1] == f"#{one} exited 0\n<s:{one}_stdout>\n{told}</s:{one}_stdout>"


async def test_the_world_refuses_a_write_to_a_door_of_a_command_that_takes_no_write() -> None:
  """The World refuses a write to the door bash1/stdout or bash1/stderr, and a write to bash1/stdin once the command ended or was cancelled, once its stdin is closed, as a stdin of text is from the start, or once a crash cut the command."""
  sand, _, root = born()
  one = engine.bash("echo hi", stdin=None, on=root)
  assert one == "bash1"
  for path in ("bash1/stdout", "bash1/stderr"):
    with pytest.raises(Refused, match="takes no write"):
      engine.write(Text(path, "x"), on=root)
  await one
  with pytest.raises(Refused, match="takes no write"):
    engine.write(Text("bash1/stdin", "late"), on=root)
  sand.auto = False
  gone, shut = engine.bash("sleep", stdin=None, on=root), engine.bash("sleep", stdin=None, on=root)
  given, bare = engine.bash("cat", stdin="x", on=root), engine.bash("cat", on=root)
  engine.cancel(gone)
  await settle()
  with pytest.raises(Refused, match="takes no write"):
    engine.write(Text(door(gone, "stdin"), "late"), on=root)
  engine.write(Text(door(shut, "stdin")), on=root)
  for one in (shut, given, bare):
    with pytest.raises(Refused, match="takes no write"):
      engine.write(Text(door(one, "stdin"), "late"), on=root)
  assert len(said(sand.calls, "write")) == 8 and sand.fed == ["x", None, None]
  held = engine.bash("sleep", stdin=None, on=root)
  await settle()
  later = Sand(auto=False)
  _, over = await relived(later, plain(sand.record))
  with pytest.raises(Refused, match="takes no write"):
    engine.write(Text(door(held, "stdin"), "late"), on=over)
  assert later.fed == []


async def test_its_close_tells_what_its_stdout_shows() -> None:
  """Its close tells what its stdout shows, and what its stderr shows when that stream has a show of its own that is no hidden one, so a merged command tells no stderr."""
  sand, _, root = born(auto=False)
  merged = engine.bash("one", on=root)
  apart = engine.bash("two", show_err=TAIL, on=root)
  quiet = engine.bash("three", show_err=HIDDEN, on=root)
  await settle()
  for one in (merged, apart, quiet):
    world_says("out", one, "bad\n", "stderr")
    sand.exits(one, 0)
  await settle()
  assert paragraphs(engine.turns(on=root))[-3:] == [
    f"#{merged} exited 0",
    f"#{apart} exited 0\n{apart}_stderr = 'bad'",
    f"#{quiet} exited 0",
  ]


async def test_the_world_asks_a_command_whether_its_stderr_flows_into_its_stdout() -> None:
  """The World asks a command whether its stderr flows into its stdout, and the command answers from what its verb was given."""
  _, log, root = born(auto=False)
  merged = engine.bash("one", on=root)
  apart = engine.bash("two", show_err=TAIL, on=root)
  await settle()
  asked = [a for a in acts(log).values() if a[0] == "merged"]
  assert asked == [("merged", "merged1", WORLD, root, merged), ("merged", "merged2", WORLD, root, apart)]
  assert [engine.peek(a[1]) for a in asked] == [True, False]


async def test_the_world_hears_the_bash_itself() -> None:
  """The World hears the bash itself, with the command, the stdin and the timeout, and no working directory and no show."""
  _, log, root = born()
  engine.cd("/deep", on=root)
  one = engine.bash("echo hi", stdin=None, timeout=5.0, show=HIDDEN, show_err=TAIL, on=root)
  assert (await one).stdout.content == "ran echo hi\n"
  assert said(log, "bash") == [("bash", one, OPERATOR, root, "echo hi", None, 5.0, (0,))]


async def test_the_world_that_takes_a_command_answers_it() -> None:
  """The World that takes a command answers it: it says it done with its Exit, the code and the streams it kept, since the owner of an act is the one that answers it."""
  sand, log, root = born(auto=False)
  act = engine.bash("run", show_err=TAIL, on=root)
  world_says("out", act, "out\n", "stdout")
  world_says("out", act, "err\n", "stderr")
  sand.exits(act, 7)
  assert [(a[0], a[2]) for a in log if a[1] == act and a[0] in ("started", "done")] == [
    ("started", WORLD),
    ("done", WORLD),
  ]
  assert await act == Exit(7, Text(f"{act}/stdout", "out\n"), Text(f"{act}/stderr", "err\n"))


async def test_the_world_runs_each_command_with_tmpdir_set_to_the_scratch_directory(tmp_path: Path) -> None:
  """The World runs each command with TMPDIR set to the scratch directory, and TMP and TEMP too on Windows, which it makes first, so the command finds the files of scratch:// in $TMPDIR, and a program that reads TMPDIR, as tempfile of python does, makes its files there."""
  live, root = machine(tmp_path)
  first = await engine.bash('test -d "$TMPDIR" && echo made', on=root)
  engine.write(Text("scratch://plan.md", "plan\n"), on=root)
  made = f'{shlex.quote(sys.executable)} -c "import tempfile; print(tempfile.mkstemp()[1])"'
  got = await engine.bash(f'cat "$TMPDIR/plan.md" && {made}', on=root)
  listed = engine.read("scratch://", on=root).content
  live.end()
  plan, one = got.stdout.content.splitlines()
  assert (first.stdout.content, plan) == ("made\n", "plan")
  assert listed == f"plan.md\n{Path(one).name}"


async def test_the_world_runs_the_command_in_a_posix_sh(tmp_path: Path) -> None:
  """The World runs the command in a POSIX sh: /bin/sh, and on Windows the sh of Git for Windows, which it finds on PATH, beside the git on PATH, or in the folder of the programs of the machine."""
  live, root = machine(tmp_path)
  got = await engine.bash('n=6; echo "$((n * 7))" "${n#6}x"', on=root)
  live.end()
  assert (got.code, got.stdout.content, Path(_monty.shell()).stem) == (0, "42 x\n", "sh")


async def test_a_command_accepts_the_code_0_alone_and_ok_names_the_codes_that_it_accepts_in_its_place() -> None:
  """A command accepts the code 0 alone, and ok names the codes that it accepts in its place, as ok=(0, 1) for a grep or a diff, which exits 1 for no match or for a difference."""
  sand, log, root = born(
    "one = bash('grep', ok=(0, 1))\ngot = await one\ntwo = bash('false')\ntry:\n  await two\nexcept Failed as no:\n  close([got.code, str(no)])",
    auto=False,
  )
  asked = engine.prompt(list, "run them", on=root)
  await settle()
  one = said(log, "bash")[0][1]
  sand.exits(one, 1)
  await settle()
  two = said(log, "bash")[1][1]
  sand.exits(two, 1)
  assert await asked == [1, f"{two} exited 1"]
  assert [a[7] for a in said(log, "bash")] == [(0, 1), (0,)]


async def test_a_command_with_ok_none_accepts_every_code_the_none_of_a_timeout_among_them() -> None:
  """A command with ok=None accepts every code, the None of a timeout among them."""
  sand, log, root = born("one = bash('slow', ok=None)\ngot = await one\nclose(str(got.code))", auto=False)
  asked = engine.prompt(str, "run it", on=root)
  await settle()
  sand.exits(said(log, "bash")[0][1], None)
  assert await asked == "None"
