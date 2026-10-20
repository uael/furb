"""bash, a command of the World."""

import shlex
import sys
from asyncio import CancelledError
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
  ended,
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


async def test_bash_is_given_a_command_its_stdin_a_timeout_and_a_show_for_each_stream() -> None:
  """bash is given a command, its stdin, a timeout, and a show for each stream, the plain words first, so the journal makes it again."""
  sand, log, root = born(auto=False)
  engine.bash("echo hi", "go", 5.0, HEAD, TAIL, on=root)
  word = said(log, "bash")[0]
  assert word == ("bash", word[1], OPERATOR, root, "echo hi", "go", 5.0)
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
  ended(live)
  assert (got.code, late.code, late.stdout.content) == (137, None, "before\n")


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
  assert engine.TIMEOUT == 600.0
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


async def test_the_stdout_and_the_stderr_doors_are_readable_while_the_command_runs_and_after_it() -> None:
  """The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it: a read gives that stream of the Exit once the command exited, and otherwise the out facts of that stream since the World last started it, so a command that runs again starts its streams again."""
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
  later = Sand(auto=False)
  _, over = await relived(later, list(sand.record))
  assert engine.read(door(one, "stdout"), on=over).content == "first\n"
  engine.wake(over)
  await settle()
  assert engine.read(door(one, "stdout"), on=over).content == ""
  later.writes(one, "second\n", "stdout")
  await settle()
  assert engine.read(door(one, "stdout"), on=over).content == "second\n"
  world_says("done", one, Exit(0, Text(door(one, "stdout"), "kept\n"), Text(door(one, "stderr"))))
  await settle()
  assert engine.read(door(one, "stdout"), on=over).content == "kept\n"


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


async def test_the_world_takes_it_and_a_later_life_has_it_take_it_again_only_when_the_record_shows_it_not_ended() -> (
  None
):
  """The World takes it, and a later life has the World take it again only when the record shows it started and not ended, and then only at a wake."""
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
  _, over = await relived(third, plain(quiet.record))
  assert said(third.calls, "bash") == [] and engine.peek(two) is None
  engine.wake(over)
  await settle()
  got = engine.peek(two)
  assert [a[1] for a in said(third.calls, "bash")] == [two] and isinstance(got, Exit) and got.code == 0


async def test_with_a_hidden_show_it_tells_its_header_and_its_binding_alone() -> None:
  """With a hidden show it tells its header and its binding alone, and not its command and not its close."""
  sand, _, root = born(auto=False)
  one = engine.bash("run", on=root)
  quiet = engine.bash("quiet", show=HIDDEN, on=root)
  sand.exits(quiet, 0)
  await quiet
  assert paragraphs(engine.turns(on=root))[2:] == [
    f"#{one}\n{one}_command = 'run'\n{one}: Act[Exit] = Act({one!r})",
    f"#{quiet}\n{quiet}: Act[Exit] = Act({quiet!r})",
  ]


async def test_once_it_has_ended_it_lives_on_to_answer_its_doors() -> None:
  """Once it has ended it lives on to answer its doors, and nothing else reaches it, so a cancel does not end it, as it ends every other act."""
  sand, _, root = born(auto=False)
  one = engine.bash("run", on=root)
  world_says("out", one, "half\n", "stdout")
  await settle()
  sand.exits(one, 0)
  got = await one
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"
  with pytest.raises(Refused, match="ended"):
    engine.write(Text(door(one, "stdin"), "late"), on=root)
  engine.cancel(one)
  await settle()
  assert engine.peek(one) == got
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"


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
  assert [a[2] for a in dones(log, "bash")] == [WORLD] and paragraphs(engine.turns(on=root))[-1] == f"#{root} paused"
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


async def test_the_engine_refuses_a_write_to_a_door_of_a_command_that_takes_no_write() -> None:
  """The engine refuses a write to the door bash1/stdout or bash1/stderr, and a write to bash1/stdin once the command ended or was cancelled, or once its stdin is closed, as a stdin of text is from the start."""
  sand, _, root = born()
  one = engine.bash("echo hi", stdin=None, on=root)
  assert one == "bash1"
  for path in ("bash1/stdout", "bash1/stderr"):
    with pytest.raises(Refused, match="takes no write"):
      engine.write(Text(path, "x"), on=root)
  await one
  with pytest.raises(Refused, match="ended"):
    engine.write(Text("bash1/stdin", "late"), on=root)
  sand.auto = False
  gone, shut = engine.bash("sleep", stdin=None, on=root), engine.bash("sleep", stdin=None, on=root)
  given, bare = engine.bash("cat", stdin="x", on=root), engine.bash("cat", on=root)
  engine.cancel(gone)
  await settle()
  with pytest.raises(Refused, match="ended"):
    engine.write(Text(door(gone, "stdin"), "late"), on=root)
  engine.write(Text(door(shut, "stdin")), on=root)
  for one in (shut, given, bare):
    with pytest.raises(Refused, match="closed"):
      engine.write(Text(door(one, "stdin"), "late"), on=root)
  assert [a for a in sand.calls if a[0] == "write"] == []


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
  assert said(log, "bash") == [("bash", one, OPERATOR, root, "echo hi", None, 5.0)]


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
  ended(live)
  plan, one = got.stdout.content.splitlines()
  assert (first.stdout.content, plan) == ("made\n", "plan")
  assert listed == f"plan.md\n{Path(one).name}"
