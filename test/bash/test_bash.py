"""bash, a command of the World."""

from asyncio import CancelledError

import pytest

from conftest import (
  BASH,
  MANY,
  STANDS,
  Bound,
  Sand,
  Where,
  exited,
  life,
  made,
  paragraphs,
  plain,
  relived,
  said,
  settle,
  texted,
  verb,
  worded,
)
from furb import engine
from furb.engine import OPERATOR, WORLD, Act, Refused


def door(one: str, part: str) -> str:
  """The door of one stream of a command, or of its stdin."""
  return f"{one}/{part}"


def quiet() -> tuple[Sand, str]:
  """A life on a World whose commands tell nothing and end only when the test says so."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  _, root = life(sand)
  return sand, root


def fed(on: str, one: str, text: str | None = None) -> object:
  """A write to the stdin of a command, as the operator says it: the text, or nothing to close it."""
  return verb("write", on)(made(on, "Text", door(one, "stdin"), *([] if text is None else [text])))


async def test_a_command_its_streams_as_they_come_its_exit_and_its_doors() -> None:
  """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to."""
  sand, root = quiet()
  one = verb("bash", root)("run", fed=True, show_err=made(root, "span", -250, -1))
  assert isinstance(one, Act)
  assert texted(fed(root, one, "go")) == (door(one, "stdin"), "go")
  assert sand.fed == ["go"]
  engine.send("out", one, "half\n", "stdout", by=WORLD)
  engine.send("out", one, "bad\n", "stderr", by=WORLD)
  await settle()
  assert texted(verb("read", root)(door(one, "stdout"))) == (door(one, "stdout"), "half\n")
  assert texted(verb("read", root)(door(one, "stderr"))) == (door(one, "stderr"), "bad\n")
  engine.send("exited", one, 2, by=WORLD)
  assert exited(await one) == (2, (door(one, "stdout"), "half\n"), (door(one, "stderr"), "bad\n"))


async def test_bash_is_given_one_show_for_each_stream_that_bash_tells() -> None:
  """bash is given one show for each stream that bash tells."""
  _, root = quiet()
  one = verb("bash", root)("run", show=made(root, "span", 2, 2), show_err=made(root, "grep", "^b"))
  assert isinstance(one, Act)
  engine.send("out", one, "one\ntwo\nthree\n", "stdout", by=WORLD)
  engine.send("out", one, "a\nb\n", "stderr", by=WORLD)
  engine.send("exited", one, 0, by=WORLD)
  await one
  assert paragraphs(engine.turns(on=root))[-1] == (
    f"#{one} exited 0\n# {one}/stdout, 0 known\n# 2 two\n# {one}/stderr, 0 known\n# 2 b"
  )


async def test_a_feed_whose_text_is_none_closes_the_stdin_of_the_command() -> None:
  """A feed whose text is None closes the stdin of the command."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  one = verb("bash", root)("run", fed=True)
  assert isinstance(one, Act)
  fed(root, one, "go")
  fed(root, one)
  assert [a[3] for a in said(log, "feed")] == ["go", None]
  assert sand.fed == ["go", None]
  with pytest.raises(Refused, match="closed"):
    fed(root, one, "late")


async def test_the_commands_of_the_world_run_at_the_same_time() -> None:
  """The commands of the World run at the same time."""
  _, root = quiet()
  one, two = verb("bash", root)("first"), verb("bash", root)("second")
  assert isinstance(one, Act) and isinstance(two, Act)
  await settle()
  engine.send("exited", two, 1, by=WORLD)
  await settle()
  assert exited(engine.peek(two))[0] == 1 and exited(engine.peek(one))[0] is None
  engine.send("exited", one, 0, by=WORLD)
  await settle()
  assert (exited(await one)[0], exited(await two)[0]) == (0, 1)


async def test_bash_is_given_a_command_a_fed_flag_a_timeout_and_a_show_for_each_stream() -> None:
  """bash is given a command, a fed flag, a timeout, and a show for each stream, the plain words first, so the record replays it."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  verb("bash", root)("echo hi", True, 5.0, made(root, "span", 1, 9), made(root, "span", -9, -1))
  word = said(log, "bash")[0]
  assert word == ("bash", word[1], OPERATOR, root, "echo hi", True, 5.0)
  assert [e[0] for e in sand.record if e[0][0] == "bash"] == [word]


async def test_bash_gives_the_command_which_is_awaited_for_its_exit_code_and_its_streams() -> None:
  """bash gives the command, which is awaited for its exit code and its streams."""
  sand = Sand(stands=STANDS, words=BASH)
  _, root = life(sand)
  one = verb("bash", root)("echo hi")
  assert isinstance(one, Act) and one == "bash1"
  assert exited(await one) == (0, ("bash1/stdout", "ran echo hi\n"), ("bash1/stderr", ""))
  word = "x = await bash('echo there')\nclose([type(x) is Exit, x.code, x.stdout.content])"
  assert await engine.rung(word, on=root) == [True, 0, "ran echo there\n"]


async def test_the_world_runs_the_command_in_the_working_directory_that_cwd_gives_on_its_chain() -> None:
  """The World runs the command in the working directory that cwd gives on its chain."""
  sand = Where(stands=STANDS, words=BASH)
  _, root = life(sand)
  one = verb("bash", root)("echo hi")
  assert isinstance(one, Act)
  await one
  assert sand.where == ["/w"]
  verb("cd", root)("/deep")
  two = verb("bash", root)("echo hi")
  assert isinstance(two, Act)
  await two
  assert sand.where == ["/w", "/deep"]


async def test_the_command_runs_until_it_ends_until_its_timeout_or_until_a_cancel() -> None:
  """The command runs until it ends, until its timeout, or until a cancel."""
  sand = Sand(stands=STANDS, words=BASH)
  _, root = life(sand)
  one = verb("bash", root)("echo hi")
  assert isinstance(one, Act) and exited(await one)[0] == 0
  sand.auto = False
  late = verb("bash", root)("forever", timeout=0)
  assert isinstance(late, Act) and exited(await late)[0] is None
  gone = verb("bash", root)("slow")
  assert isinstance(gone, Act)
  await settle()
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.peek(gone), CancelledError)


async def test_the_world_ends_the_command_at_its_timeout() -> None:
  """The World ends the command at its timeout."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  one = verb("bash", root)("forever", timeout=0)
  assert isinstance(one, Act)
  await settle()
  assert [(a[2], a[3]) for a in said(log, "exited") if a[1] == one] == [(WORLD, None)]
  assert exited(engine.peek(one)) == (None, (door(one, "stdout"), ""), (door(one, "stderr"), ""))


async def test_the_command_completes_with_its_exit_code_and_its_streams() -> None:
  """The command completes with its exit code and its streams."""
  sand = Sand(stands=STANDS, words=BASH)
  _, root = life(sand)
  one = verb("bash", root)("echo hi")
  assert isinstance(one, Act)
  assert exited(await one) == (0, (door(one, "stdout"), "ran echo hi\n"), (door(one, "stderr"), ""))


async def test_the_exit_code_of_the_command_is_none_after_a_timeout() -> None:
  """The exit code of the command is None after a timeout."""
  _, root = quiet()
  one = verb("bash", root)("forever", timeout=0)
  assert isinstance(one, Act) and exited(await one)[0] is None


async def test_the_stdin_of_the_command_is_closed_unless_bash_opened_the_command_fed() -> None:
  """The stdin of the command is closed unless bash opened the command fed."""
  sand, root = quiet()
  shut, opened = verb("bash", root)("first"), verb("bash", root)("second", fed=True)
  assert isinstance(shut, Act) and isinstance(opened, Act)
  await settle()
  with pytest.raises(Refused, match="not fed"):
    fed(root, shut, "x")
  assert texted(fed(root, opened, "x")) == (door(opened, "stdin"), "x")
  assert sand.fed == ["x"]


async def test_without_a_stderr_show_the_stderr_of_the_command_flows_into_its_stdout() -> None:
  """Without a stderr show, the stderr of the command flows into its stdout."""
  _, root = quiet()
  one = verb("bash", root)("run")
  assert isinstance(one, Act)
  engine.send("out", one, "half\n", "stdout", by=WORLD)
  engine.send("out", one, "oops\n", "stderr", by=WORLD)
  await settle()
  assert texted(verb("read", root)(door(one, "stdout")))[1] == "half\noops\n"


async def test_the_stderr_door_of_a_merged_command_stays_empty() -> None:
  """The stderr door of a merged command stays empty."""
  _, root = quiet()
  one = verb("bash", root)("run")
  assert isinstance(one, Act)
  engine.send("out", one, "oops\n", "stderr", by=WORLD)
  await settle()
  assert texted(verb("read", root)(door(one, "stderr"))) == (door(one, "stderr"), "")


async def test_a_command_without_a_timeout_has_the_timeout_timeout_that_the_file_names() -> None:
  """A command without a timeout has the timeout TIMEOUT that the file names."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  verb("bash", root)("run")
  assert await worded("close(TIMEOUT)", BASH) == 600.0
  assert [a[6] for a in said(log, "bash")] == [600.0]


async def test_the_stdout_and_the_stderr_doors_are_readable_while_the_command_runs_and_after_it() -> None:
  """The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it."""
  _, root = quiet()
  one = verb("bash", root)("run", show_err=made(root, "span", -250, -1))
  assert isinstance(one, Act) and one == "bash1"
  engine.send("out", one, "one\n", "stdout", by=WORLD)
  engine.send("out", one, "bad\n", "stderr", by=WORLD)
  await settle()
  assert texted(verb("read", root)("bash1/stdout")) == ("bash1/stdout", "one\n")
  assert texted(verb("read", root)("bash1/stderr")) == ("bash1/stderr", "bad\n")
  engine.send("exited", one, 0, by=WORLD)
  await one
  assert texted(verb("read", root)("bash1/stdout")) == ("bash1/stdout", "one\n")
  assert texted(verb("read", root)("bash1/stderr")) == ("bash1/stderr", "bad\n")


async def test_the_stdin_door_is_writable_while_a_fed_command_runs() -> None:
  """The door bash1/stdin is writable while a fed command runs."""
  sand, root = quiet()
  one = verb("bash", root)("run", fed=True)
  assert one == "bash1"
  await settle()
  assert texted(fed(root, "bash1", "go")) == ("bash1/stdin", "go")
  assert texted(fed(root, "bash1", "more")) == ("bash1/stdin", "more")
  assert sand.fed == ["go", "more"]


async def test_a_write_of_nothing_to_the_stdin_door_closes_the_stdin() -> None:
  """A write of nothing to bash1/stdin closes the stdin."""
  sand, root = quiet()
  one = verb("bash", root)("run", fed=True)
  assert one == "bash1"
  assert texted(fed(root, "bash1", "go")) == ("bash1/stdin", "go")
  assert texted(fed(root, "bash1")) == ("bash1/stdin", "")
  assert sand.fed == ["go", None]
  with pytest.raises(Refused, match="closed"):
    fed(root, "bash1", "late")


async def test_a_read_of_the_stdout_door_gives_the_lines_of_the_command_while_the_command_runs() -> None:
  """A read of bash1/stdout gives the lines of the command while the command runs."""
  _, root = quiet()
  one = verb("bash", root)("run")
  assert isinstance(one, Act) and one == "bash1"
  engine.send("out", one, "one\ntwo\n", "stdout", by=WORLD)
  await settle()
  assert await engine.rung("close(read('bash1/stdout').lines)", on=root) == ["one", "two"]
  assert exited(engine.peek(str(one)))[0] is None


async def test_a_wake_on_a_chain_with_a_source_starts_no_inherited_command_again() -> None:
  """A wake on a chain with a source starts no inherited command again, since only the owner starts an act."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  assert await engine.prompt(int, "start one", on=root) == 1
  command = said(log, "bash")[0][1]
  twin = engine.chain("twin", source=root)
  await settle()
  engine.pause(twin)
  engine.wake(twin)
  await settle()
  assert [a[1] for a in sand.calls if a[0] == "start"] == [command]
  assert [a[1] for a in said(log, "bash")] == [command]


async def test_the_world_starts_it_and_a_later_life_starts_it_again_only_when_the_record_shows_it_not_ended() -> None:
  """The World starts it, and a later life starts it again only when the record shows it started and not ended, and then only at a wake."""
  sand = Sand(stands=STANDS, words=BASH)
  _, root = life(sand)
  one = verb("bash", root)("echo hi")
  assert isinstance(one, Act)
  await one
  await settle()
  assert [a[1] for a in sand.calls if a[0] == "start"] == [one]
  later = Sand(stands=STANDS, words=BASH)
  _, over = await relived(later, plain(sand.record))
  engine.wake(over)
  await settle()
  assert [a for a in later.calls if a[0] == "start"] == [] and over == root
  assert exited(engine.peek(one))[0] == 0
  silent = Sand(stands=STANDS, auto=False, words=BASH)
  _, root = life(silent)
  two = verb("bash", root)("sleep 9")
  await settle()
  third = Sand(stands=STANDS, words=BASH)
  _, over = await relived(third, plain(silent.record))
  assert said(third.calls, "start") == []
  assert exited(engine.peek(str(two))) == (None, (f"{two}/stdout", ""), (f"{two}/stderr", ""))
  engine.wake(over)
  await settle()
  assert [a[1] for a in said(third.calls, "start")] == [two] and exited(engine.outcomes[str(two)])[0] == 0


async def test_its_stdin_is_written_while_it_runs_and_it_is_fed() -> None:
  """Its stdin is written while it runs and it is fed, which it says to the World as a feed and answers with the text that landed, and a write of nothing closes it; a write of it takes no word once the command ended, and none at all when the command was not opened fed."""
  sand, root = quiet()
  one, bare = verb("bash", root)("run", fed=True), verb("bash", root)("other")
  assert isinstance(one, Act) and isinstance(bare, Act)
  await settle()
  assert texted(fed(root, one, "go")) == (door(one, "stdin"), "go")
  assert sand.fed == ["go"]
  fed(root, one)
  assert sand.fed == ["go", None]
  with pytest.raises(Refused, match="not fed"):
    fed(root, bare, "x")
  engine.send("exited", one, 0, by=WORLD)
  await one
  with pytest.raises(Refused, match="ended"):
    fed(root, one, "late")


async def test_without_a_show_of_its_own_the_stderr_of_it_flows_into_its_stdout() -> None:
  """Without a show of its own, the stderr of it flows into its stdout, and the door of its stderr stays empty; with a hidden show it tells its header and its binding alone, and not its command and not its close."""
  _, root = quiet()
  one = verb("bash", root)("run")
  assert isinstance(one, Act)
  engine.send("out", one, "half\n", "stdout", by=WORLD)
  engine.send("out", one, "oops\n", "stderr", by=WORLD)
  await settle()
  assert texted(verb("read", root)(door(one, "stdout")))[1] == "half\noops\n"
  assert texted(verb("read", root)(door(one, "stderr")))[1] == ""
  hidden = verb("bash", root)("quiet", show=Bound(root).HIDDEN)
  assert isinstance(hidden, Act)
  engine.send("exited", hidden, 0, by=WORLD)
  await hidden
  assert [part for part in paragraphs(engine.turns(on=root)) if part.startswith("#bash")] == [
    f"#{one} run\n{one}: Act[Exit] = Act({one!r})",
    f"#{hidden}\n{hidden}: Act[Exit] = Act({hidden!r})",
  ]


async def test_it_answers_a_read_of_a_stream_and_a_peek_while_it_runs() -> None:
  """It answers a read of a stream and a peek while it runs, and once it has ended it lives on to answer a read of its streams and to refuse a write of its stdin, and nothing else reaches it, so a cancel does not end it, as it ends every other act, and a peek at it once it ended the life answers from its outcomes."""
  _, root = quiet()
  one = verb("bash", root)("run")
  assert isinstance(one, Act)
  engine.send("out", one, "half\n", "stdout", by=WORLD)
  await settle()
  assert texted(verb("read", root)(door(one, "stdout")))[1] == "half\n"
  assert exited(engine.peek(one))[0] is None
  engine.send("exited", one, 0, by=WORLD)
  got = exited(await one)
  assert texted(verb("read", root)(door(one, "stdout")))[1] == "half\n"
  with pytest.raises(Refused, match="ended"):
    fed(root, one, "late")
  engine.cancel(one)
  await settle()
  assert exited(engine.peek(one)) == got == exited(engine.outcomes[one])
  assert texted(verb("read", root)(door(one, "stdout")))[1] == "half\n"


async def test_it_runs_until_it_ends_until_its_timeout_or_until_a_cancel() -> None:
  """It runs until it ends, until its timeout or until a cancel: the World is the one that ends it, at the timeout it reads off the act as at a cancel, since the World is the one running it, and the engine says nothing to make it."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  late = verb("bash", root)("forever", timeout=0)
  await settle()
  gone = verb("bash", root)("slow")
  assert isinstance(gone, Act)
  await settle()
  engine.cancel(gone)
  await settle()
  assert {a[1]: (a[2], a[3]) for a in said(log, "exited")} == {late: (WORLD, None), gone: (WORLD, None)}
  assert said(log, "wait") == []


async def test_a_pause_stops_no_command() -> None:
  """A pause stops no command: it runs on, and its close waits for the wake."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  one = verb("bash", root)("run")
  assert isinstance(one, Act)
  await settle()
  engine.pause(root)
  engine.send("out", one, "half\n", "stdout", by=WORLD)
  engine.send("exited", one, 0, by=WORLD)
  await settle()
  assert one not in engine.outcomes
  assert [a for a in said(log, "done") if a[1] == one] == []
  engine.wake(root)
  await settle()
  assert exited(await one)[1][1] == "half\n"


async def test_its_close_tells_what_its_stdout_shows() -> None:
  """Its close tells what its stdout shows, and what its stderr shows when that stream has a show of its own that is no hidden one, so a merged command tells no stderr."""
  _, root = quiet()
  merged = verb("bash", root)("one")
  apart = verb("bash", root)("two", show_err=made(root, "span", -250, -1))
  hidden = verb("bash", root)("three", show_err=Bound(root).HIDDEN)
  await settle()
  for one in (merged, apart, hidden):
    engine.send("out", str(one), "bad\n", "stderr", by=WORLD)
    engine.send("exited", str(one), 0, by=WORLD)
  await settle()
  assert paragraphs(engine.turns(on=root))[-3:] == [
    f"#{merged} exited 0\n# {merged}/stdout, 0 known\n# 1 bad",
    f"#{apart} exited 0\n# {apart}/stdout, 0 known\n# {apart}/stderr, 0 known\n# 1 bad",
    f"#{hidden} exited 0\n# {hidden}/stdout, 0 known",
  ]


async def test_the_paragraph_of_an_exited_command_holds_one_showing_for_each_text_told() -> None:
  """The paragraph of an exited command holds one showing for each text told."""
  sand = Sand(stands=STANDS, words=BASH)
  log, root = life(sand)
  both = verb("bash", root)("echo hi", show_err=made(root, "span", -250, -1))
  merged = verb("bash", root)("echo two")
  assert isinstance(both, Act) and isinstance(merged, Act)
  await both
  await merged
  await settle()
  told = {a[1]: [note[:2] for note in a[3][1:]] for a in said(log, "tell") if a[3][0].endswith(" exited 0")}
  assert told == {
    both: [(f"{both}/stdout", "ran echo hi\n"), (f"{both}/stderr", "")],
    merged: [(f"{merged}/stdout", "ran echo two\n")],
  }
  assert [part for part in paragraphs(engine.turns(on=root)) if " exited " in part.split("\n", 1)[0]] == [
    f"#{both} exited 0\n# {both}/stdout, 0 known\n# 1 ran echo hi\n# {both}/stderr, 0 known",
    f"#{merged} exited 0\n# {merged}/stdout, 0 known\n# 1 ran echo two",
  ]


async def test_a_show_applies_to_a_stream_as_it_applies_to_a_text() -> None:
  """A show applies to a stream as it applies to a text."""
  sand = Sand(files={"/w/n.txt": "a\nb\nc\n"}, stands=STANDS, auto=False, words=BASH)
  _, root = life(sand)
  assert await engine.rung("read('n.txt', span(2, 2))", on=root) is None
  one = verb("bash", root)("many", show=made(root, "span", 2, 2))
  assert isinstance(one, Act)
  engine.send("out", one, "a\nb\nc\n", "stdout", by=WORLD)
  engine.send("exited", one, 0, by=WORLD)
  await one
  told = [part for part in paragraphs(engine.turns(on=root)) if part.startswith(("#read", f"#{one} exited"))]
  assert told == ["#read n.txt\n# /w/n.txt, 0 known\n# 2 b", f"#{one} exited 0\n# {one}/stdout, 0 known\n# 2 b"]


async def test_the_stdout_of_a_command_without_a_show_is_told_as_tail() -> None:
  """The stdout of a command without a show is told as TAIL, which is the span of its last 250 lines."""
  lines = MANY.splitlines()
  assert await worded(f"close(TAIL({lines!r}) == span(-250, -1)({lines!r}) == list(range(51, 301)))", BASH) is True
  _, root = quiet()
  one = verb("bash", root)("run")
  assert isinstance(one, Act)
  engine.send("out", one, MANY, "stdout", by=WORLD)
  engine.send("exited", one, 0, by=WORLD)
  await one
  told = "".join(f"\n# {i} line {i}" for i in range(51, 301))
  assert paragraphs(engine.turns(on=root))[-1] == f"#{one} exited 0\n# {one}/stdout, 0 known{told}"


async def test_a_command_refuses_a_write_to_the_stdin_door_once_it_ended_or_was_cancelled() -> None:
  """A command refuses a write to bash1/stdin once it ended or was cancelled."""
  sand = Sand(stands=STANDS, words=BASH)
  _, root = life(sand)
  one = verb("bash", root)("echo hi", fed=True)
  assert isinstance(one, Act) and one == "bash1"
  await one
  with pytest.raises(Refused, match="ended"):
    fed(root, "bash1", "late")
  sand.auto = False
  gone = verb("bash", root)("sleep", fed=True)
  assert isinstance(gone, Act)
  engine.cancel(gone)
  await settle()
  with pytest.raises(Refused, match="ended"):
    fed(root, gone, "late")


async def test_the_world_asks_a_command_whether_its_stderr_flows_into_its_stdout() -> None:
  """The World asks a command whether its stderr flows into its stdout, as it asks cwd where its paths resolve, and the command answers from what its verb was given."""
  _, root = quiet()
  merged = verb("bash", root)("one")
  apart = verb("bash", root)("two", show_err=made(root, "span", -250, -1))
  await settle()
  asked = [a for a in engine.asked.values() if a[0] == "merged"]
  assert [a[2:] for a in asked] == [(WORLD, root, merged), (WORLD, root, apart)]
  assert [engine.outcomes[a[1]] for a in asked] == [True, False]


async def test_the_world_hears_the_bash_itself() -> None:
  """The World hears the bash itself, with the command, the fed flag and the timeout, and no working directory and no show."""
  sand = Sand(stands=STANDS, words=BASH)
  log, root = life(sand)
  verb("cd", root)("/deep")
  one = verb("bash", root)("echo hi", fed=True, timeout=5.0, show=Bound(root).HIDDEN)
  assert isinstance(one, Act)
  assert exited(await one)[1][1] == "ran echo hi\n"
  assert said(log, "bash") == [("bash", one, OPERATOR, root, "echo hi", True, 5.0)]
  (started,) = [a for a in sand.calls if a[0] == "start"]
  assert started == ("start", one, one)
