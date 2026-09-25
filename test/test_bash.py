"""bash, a command of the World."""

from asyncio import CancelledError

import pytest

from conftest import MANY, STANDS, Sand, Where, acts, dones, life, paragraphs, plain, relived, said, settle, world_says
from furb import engine
from furb.engine import HEAD, HIDDEN, OPERATOR, TAIL, WORLD, Act, Exit, Refused, Text, grep, span


def door(one: str, part: str) -> str:
  """The door of one stream of a command, or of its stdin."""
  return f"{one}/{part}"


async def test_a_command_its_streams_as_they_come_its_exit_and_its_doors() -> None:
  """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", fed=True, show_err=TAIL, on=root)
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
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", show=span(2, 2), show_err=grep("^b"), on=root)
  world_says("out", one, "one\ntwo\nthree\n", "stdout")
  world_says("out", one, "a\nb\n", "stderr")
  sand.exits(one, 0)
  await one
  assert paragraphs(engine.turns(on=root))[-1] == (
    f"#{one} exited 0\n# {one}/stdout, 0 known\n# 2 two\n# {one}/stderr, 0 known\n# 2 b"
  )


async def test_a_feed_whose_text_is_none_closes_the_stdin_of_the_command() -> None:
  """A feed whose text is None closes the stdin of the command."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  one = engine.bash("run", fed=True, on=root)
  engine.write(Text(door(one, "stdin"), "go"), on=root)
  engine.write(Text(door(one, "stdin")), on=root)
  assert [a[3] for a in said(log, "feed")] == ["go", None]
  assert sand.fed == ["go", None]
  with pytest.raises(Refused, match="closed"):
    engine.write(Text(door(one, "stdin"), "late"), on=root)


async def test_the_commands_of_the_world_run_at_the_same_time() -> None:
  """The commands of the World run at the same time."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one, two = engine.bash("first", on=root), engine.bash("second", on=root)
  await settle()
  sand.exits(two, 1)
  await settle()
  done = engine.peek(two)
  assert isinstance(done, Exit) and done.code == 1 and engine.peek(one) is None
  sand.exits(one, 0)
  await settle()
  assert ((await one).code, (await two).code) == (0, 1)


async def test_bash_is_given_a_command_a_fed_flag_a_timeout_and_a_show_for_each_stream() -> None:
  """bash is given a command, a fed flag, a timeout, and a show for each stream, the plain words first, so the record replays it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  engine.bash("echo hi", True, 5.0, HEAD, TAIL, on=root)
  word = said(log, "bash")[0]
  assert word == ("bash", word[1], OPERATOR, root, "echo hi", True, 5.0)
  assert [e[0] for e in sand.record if e[0][0] == "bash"] == [word]


async def test_bash_gives_the_command_which_is_awaited_for_its_exit_code_and_its_streams() -> None:
  """bash gives the command, which is awaited for its exit code and its streams."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert isinstance(one, Act) and one == "bash1"
  got = await one
  assert isinstance(got, Exit) and (got.code, got.stdout.content) == (0, "ran echo hi\n")


async def test_the_world_runs_the_command_in_the_working_directory_of_the_chain_which_it_reads() -> None:
  """The World runs the command in the working directory of the chain, which it reads."""
  sand = Where(stands=STANDS)
  _, root = life(sand)
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/w"]
  engine.cd("/deep", on=root)
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/w", "/deep"]


async def test_the_command_runs_until_it_ends_until_its_timeout_or_until_a_cancel() -> None:
  """The command runs until it ends, until its timeout, or until a cancel."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
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
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  one = engine.bash("forever", timeout=0, on=root)
  await settle()
  assert [(a[1], a[2]) for a in dones(log, "bash")] == [(one, WORLD)]
  assert engine.peek(one) == Exit(None, Text(door(one, "stdout")), Text(door(one, "stderr")))


async def test_the_command_completes_with_its_exit_code_and_its_streams() -> None:
  """The command completes with its exit code and its streams."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert await one == Exit(0, Text(door(one, "stdout"), "ran echo hi\n"), Text(door(one, "stderr"), ""))


async def test_the_exit_code_of_the_command_is_none_after_a_timeout() -> None:
  """The exit code of the command is None after a timeout."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  assert (await engine.bash("forever", timeout=0, on=root)).code is None


async def test_the_stdin_of_the_command_is_closed_unless_bash_opened_the_command_fed() -> None:
  """The stdin of the command is closed unless bash opened the command fed."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  shut, open_ = engine.bash("first", on=root), engine.bash("second", fed=True, on=root)
  await settle()
  with pytest.raises(Refused, match="not fed"):
    engine.write(Text(door(shut, "stdin"), "x"), on=root)
  assert engine.write(Text(door(open_, "stdin"), "x"), on=root) == Text(door(open_, "stdin"), "x")
  assert sand.fed == ["x"]


async def test_without_a_stderr_show_the_stderr_of_the_command_flows_into_its_stdout() -> None:
  """Without a stderr show, the stderr of the command flows into its stdout."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", on=root)
  world_says("out", one, "half\n", "stdout")
  world_says("out", one, "oops\n", "stderr")
  await settle()
  assert engine.read(door(one, "stdout"), on=root).content == "half\noops\n"


async def test_the_stderr_door_of_a_merged_command_stays_empty() -> None:
  """The stderr door of a merged command stays empty."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", on=root)
  world_says("out", one, "oops\n", "stderr")
  await settle()
  assert engine.read(door(one, "stderr"), on=root) == Text(door(one, "stderr"), "")


async def test_a_command_without_a_timeout_has_the_timeout_timeout_that_the_file_names() -> None:
  """A command without a timeout has the timeout TIMEOUT that the file names."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  engine.bash("run", on=root)
  assert engine.TIMEOUT == 600.0
  assert [a[6] for a in said(log, "bash")] == [engine.TIMEOUT]


async def test_the_stdout_and_the_stderr_doors_are_readable_while_the_command_runs_and_after_it() -> None:
  """The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", show_err=TAIL, on=root)
  assert one == "bash1"
  world_says("out", one, "one\n", "stdout")
  world_says("out", one, "bad\n", "stderr")
  await settle()
  assert engine.read("bash1/stdout", on=root) == Text("bash1/stdout", "one\n")
  assert engine.read("bash1/stderr", on=root) == Text("bash1/stderr", "bad\n")
  sand.exits(one, 0)
  await one
  assert engine.read("bash1/stdout", on=root) == Text("bash1/stdout", "one\n")
  assert engine.read("bash1/stderr", on=root) == Text("bash1/stderr", "bad\n")


async def test_the_stdin_door_is_writable_while_a_fed_command_runs() -> None:
  """The door bash1/stdin is writable while a fed command runs."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", fed=True, on=root)
  assert one == "bash1"
  await settle()
  assert engine.write(Text("bash1/stdin", "go"), on=root) == Text("bash1/stdin", "go")
  assert engine.write(Text("bash1/stdin", "more"), on=root) == Text("bash1/stdin", "more")
  assert sand.fed == ["go", "more"]


async def test_a_write_of_nothing_to_the_stdin_door_closes_the_stdin() -> None:
  """A write of nothing to bash1/stdin closes the stdin."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", fed=True, on=root)
  assert one == "bash1"
  assert engine.write(Text("bash1/stdin", "go"), on=root) == Text("bash1/stdin", "go")
  assert engine.write(Text("bash1/stdin"), on=root) == Text("bash1/stdin")
  assert sand.fed == ["go", None]
  with pytest.raises(Refused, match="closed"):
    engine.write(Text("bash1/stdin", "late"), on=root)


async def test_a_read_of_the_stdout_door_gives_the_lines_of_the_command_while_the_command_runs() -> None:
  """A read of bash1/stdout gives the lines of the command while the command runs."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", on=root)
  assert one == "bash1"
  world_says("out", one, "one\ntwo\n", "stdout")
  await settle()
  assert engine.read("bash1/stdout", on=root).lines == ["one", "two"]
  assert engine.peek(one) is None


async def test_a_wake_on_a_chain_with_a_source_starts_no_inherited_command_again() -> None:
  """A wake on a chain with a source starts no inherited command again, since only the owner starts an act."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  assert await engine.prompt(int, "start one", on=root) == 1
  command = said(log, "bash")[0][1]
  twin = engine.chain("twin", source=root)
  await settle()
  engine.pause(twin)
  engine.wake(twin)
  await settle()
  assert [a[1] for a in sand.calls if a[0] == "bash"] == [command]
  assert [a[1] for a in said(log, "bash")] == [command]


async def test_the_world_takes_it_and_a_later_life_has_it_take_it_again_only_when_the_record_shows_it_not_ended() -> (
  None
):
  """The World takes it, and a later life has the World take it again only when the record shows it started and not ended, and then only at a wake."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  await one
  await settle()
  assert [a[1] for a in sand.calls if a[0] == "bash"] == [one]
  later = Sand(stands=STANDS)
  _, over = await relived(later, plain(sand.record))
  engine.wake(over)
  await settle()
  assert [a for a in later.calls if a[0] == "bash"] == [] and over == root
  got = engine.peek(one)
  assert isinstance(got, Exit) and got.code == 0
  quiet = Sand(stands=STANDS, auto=False)
  _, root = life(quiet)
  two = engine.bash("sleep 9", on=root)
  await settle()
  third = Sand(stands=STANDS)
  _, over = await relived(third, plain(quiet.record))
  assert said(third.calls, "bash") == [] and engine.peek(two) is None
  engine.wake(over)
  await settle()
  got = engine.peek(two)
  assert [a[1] for a in said(third.calls, "bash")] == [two] and isinstance(got, Exit) and got.code == 0


async def test_its_stdin_is_written_while_it_runs_and_it_is_fed() -> None:
  """Its stdin is written while it runs and it is fed, which it says to the World as a feed and answers with the text that landed, and a write of nothing closes it; a write of it takes no word once the command ended, and none at all when the command was not opened fed."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", fed=True, on=root)
  bare = engine.bash("other", on=root)
  await settle()
  assert engine.write(Text(door(one, "stdin"), "go"), on=root) == Text(door(one, "stdin"), "go")
  assert sand.fed == ["go"]
  engine.write(Text(door(one, "stdin")), on=root)
  assert sand.fed == ["go", None]
  with pytest.raises(Refused, match="not fed"):
    engine.write(Text(door(bare, "stdin"), "x"), on=root)
  sand.exits(one, 0)
  await one
  with pytest.raises(Refused, match="ended"):
    engine.write(Text(door(one, "stdin"), "late"), on=root)


async def test_without_a_show_of_its_own_the_stderr_of_it_flows_into_its_stdout() -> None:
  """Without a show of its own, the stderr of it flows into its stdout, and the door of its stderr stays empty; with a hidden show it tells its header and its binding alone, and not its command and not its close."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", on=root)
  world_says("out", one, "half\n", "stdout")
  world_says("out", one, "oops\n", "stderr")
  await settle()
  assert engine.read(door(one, "stdout"), on=root).content == "half\noops\n"
  assert engine.read(door(one, "stderr"), on=root).content == ""
  quiet = engine.bash("quiet", show=HIDDEN, on=root)
  sand.exits(quiet, 0)
  await quiet
  assert paragraphs(engine.turns(on=root))[2:] == [
    f"#{one} run\n{one}: Act[Exit] = Act({one!r})",
    f"#{quiet}\n{quiet}: Act[Exit] = Act({quiet!r})",
  ]


async def test_it_answers_a_read_of_a_stream_while_it_runs() -> None:
  """It answers a read of a stream while it runs, and once it has ended it lives on to answer a read of its streams and to refuse a write of its stdin, and nothing else reaches it, so a cancel does not end it, as it ends every other act."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", on=root)
  world_says("out", one, "half\n", "stdout")
  await settle()
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"
  sand.exits(one, 0)
  got = await one
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"
  with pytest.raises(Refused, match="ended"):
    engine.write(Text(door(one, "stdin"), "late"), on=root)
  engine.cancel(one)
  await settle()
  assert engine.peek(one) == got == engine.peek(one)
  assert engine.read(door(one, "stdout"), on=root).content == "half\n"


async def test_it_runs_until_it_ends_until_its_timeout_or_until_a_cancel() -> None:
  """It runs until it ends, until its timeout or until a cancel: the World is the one that ends it, at the timeout it reads off the act as at a cancel, since the World is the one running it, and the engine says nothing to make it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
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
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  one = engine.bash("run", on=root)
  await settle()
  engine.pause(root)
  world_says("out", one, "half\n", "stdout")
  sand.exits(one, 0)
  await settle()
  assert [a[2] for a in dones(log, "bash")] == [WORLD] and paragraphs(engine.turns(on=root))[-1] == f"#{root} paused"
  engine.wake(root)
  await settle()
  assert paragraphs(engine.turns(on=root))[-1] == f"#{one} exited 0\n# {one}/stdout, 0 known\n# 1 half"


async def test_the_stdout_of_a_command_without_a_show_is_told_as_tail() -> None:
  """The stdout of a command without a show is told as TAIL, which is the span of its last 250 lines."""
  lines = MANY.splitlines()
  assert engine.TAIL(lines) == span(-250, -1)(lines) == list(range(51, 301))
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("run", on=root)
  world_says("out", one, MANY, "stdout")
  sand.exits(one, 0)
  await one
  told = "".join(f"\n# {i} line {i}" for i in range(51, 301))
  assert paragraphs(engine.turns(on=root))[-1] == f"#{one} exited 0\n# {one}/stdout, 0 known{told}"


async def test_the_engine_refuses_a_write_to_the_stdin_door_after_the_command_ended() -> None:
  """The engine refuses a write to bash1/stdin once the command ended or was cancelled."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", fed=True, on=root)
  assert one == "bash1"
  await one
  with pytest.raises(Refused, match="ended"):
    engine.write(Text("bash1/stdin", "late"), on=root)
  sand.auto = False
  gone = engine.bash("sleep", fed=True, on=root)
  engine.cancel(gone)
  await settle()
  with pytest.raises(Refused, match="ended"):
    engine.write(Text(door(gone, "stdin"), "late"), on=root)


async def test_its_close_tells_what_its_stdout_shows() -> None:
  """Its close tells what its stdout shows, and what its stderr shows when that stream has a show of its own that is no hidden one, so a merged command tells no stderr."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  merged = engine.bash("one", on=root)
  apart = engine.bash("two", show_err=TAIL, on=root)
  quiet = engine.bash("three", show_err=HIDDEN, on=root)
  await settle()
  for one in (merged, apart, quiet):
    world_says("out", one, "bad\n", "stderr")
    sand.exits(one, 0)
  await settle()
  assert paragraphs(engine.turns(on=root))[-3:] == [
    f"#{merged} exited 0\n# {merged}/stdout, 0 known\n# 1 bad",
    f"#{apart} exited 0\n# {apart}/stdout, 0 known\n# {apart}/stderr, 0 known\n# 1 bad",
    f"#{quiet} exited 0\n# {quiet}/stdout, 0 known",
  ]


async def test_the_world_asks_a_command_whether_its_stderr_flows_into_its_stdout() -> None:
  """The World asks a command whether its stderr flows into its stdout, and the command answers from what its verb was given."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  merged = engine.bash("one", on=root)
  apart = engine.bash("two", show_err=TAIL, on=root)
  await settle()
  asked = [a for a in acts(log).values() if a[0] == "merged"]
  assert asked == [("merged", "merged1", WORLD, root, merged), ("merged", "merged2", WORLD, root, apart)]
  assert [engine.peek(a[1]) for a in asked] == [True, False]


async def test_the_world_hears_the_bash_itself() -> None:
  """The World hears the bash itself, with the command, the fed flag and the timeout, and no working directory and no show."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  engine.cd("/deep", on=root)
  one = engine.bash("echo hi", fed=True, timeout=5.0, show=HIDDEN, show_err=TAIL, on=root)
  assert (await one).stdout.content == "ran echo hi\n"
  assert said(log, "bash") == [("bash", one, OPERATOR, root, "echo hi", True, 5.0)]
  sand.auto = False
  two = engine.bash("forever", timeout=0, on=root)
  await settle()
  assert (await two).code is None


async def test_the_world_that_takes_a_command_answers_it() -> None:
  """The World that takes a command answers it: it says it done with its Exit, the code and the streams it kept, since the owner of an act is the one that answers it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("run", show_err=TAIL, on=root)
  world_says("out", act, "out\n", "stdout")
  world_says("out", act, "err\n", "stderr")
  sand.exits(act, 7)
  assert [(a[0], a[2]) for a in log if a[1] == act and a[0] in ("started", "done")] == [
    ("started", WORLD),
    ("done", WORLD),
  ]
  assert await act == Exit(7, Text(f"{act}/stdout", "out\n"), Text(f"{act}/stderr", "err\n"))
