"""Note, one thing a tell says."""

import re
from collections.abc import Sequence

from conftest import BAD, acknowledged, born, heads, opened, paragraphs, said, settle, takes, world_says
from furb import engine
from furb.engine import OPERATOR, span

EVERY = (
  "x = bash('echo hi')\n"
  "read('a.txt')\n"
  "def shut(id):\n"
  "  yield 'started', id\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('write', q, _, _, t) if t.path == 'x://b.txt':\n"
  "        yield 'done', q, len(t.content)\n"
  "act('shut', '', shut)\n"
  "write(Text('x://b.txt', 'x'))\n"
  "peek(__name__)\n"
  "turns()\n"
  "clock()\n"
  "chance()\n"
  "gate('k = 9')\n"
  "cd('/x')\n"
  "wait(on=chain('far'))\n"
  "cwd()\n"
  "debug(t'{1}')\n"
  "close(1)\n"
)
EVENTS = {
  "closed",
  "done",
  "exited",
  "raised",
  "debugged",
  "refused",
  "ledger",
  "standing",
  "advance",
  "paused",
  "woke",
  "cancelled",
}
"""The words a header of an act says after its id, when it says what happened and no open of the act."""


def headers(got: Sequence[tuple]) -> list[str]:
  """Every line of the user turns of a fold that reads as a header, in order: # and, with no space, a name."""
  return [line for one in paragraphs(got) for line in one.split("\n") if re.match(r"#\S", line)]


def spoken(head: str) -> str:
  """What a header says: the word after the id of an act, or the open of an act, which says nothing after it."""
  rest = head[1:].split(" ", 2)[1:]
  return rest[0] if rest and rest[0] in EVENTS else "open"


async def test_one_thing_a_tell_says() -> None:
  """One thing a tell says: a paragraph, or the binding of an act."""
  _, log, root = born("read('n.txt', span(1, 1))\nclose(1)", files={"/w/n.txt": "one\ntwo\n"})
  assert await engine.thread(int, "read it", on=root) == 1
  read = [a[3] for a in said(log, "tell") if a[3][0].startswith("#read1")]
  assert read == [["#read1\nread1_path = 'n.txt'\nread1_text = 'one'"]]
  opened = [a[3] for a in said(log, "tell") if a[1] == "thread1"]
  assert opened == [["#thread1\nthread1_markdown = 'read it'", "thread1: Act[int] = Act('thread1')"]]
  assert all(isinstance(note, str) for a in said(log, "tell") for note in a[3])


async def test_a_paragraph_is_what_one_fact_that_tells_stands_as_in_a_turn() -> None:
  """A paragraph is what one fact that tells stands as in a turn: its notes, one after the other, and a blank line between two paragraphs."""
  _, log, root = born()
  act = engine.rung("k = 1\nj = 2", on=root)
  assert await act is None
  got = engine.turns(on=root)
  assert got[-1][1] == "\n\n".join("\n".join(a[3]) for a in said(log, "tell"))
  assert paragraphs(got)[2:] == [f"#{act}\n<s:{act}_word>\nk = 1\nj = 2</s:{act}_word>"]


async def test_the_first_line_of_a_paragraph_is_its_header() -> None:
  """The first line of a paragraph is its header: # and, with no space, the id of the act it is of, then what happened to the act, as #bash1 exited 0 or #thread1, and no text that the act tells."""
  _, _, root = born("x = bash('echo hi')\nread('a.txt')\nclose((await x).code)")
  assert await engine.thread(int, "run it", on=root) == 0
  await settle()
  got = heads(engine.turns(on=root))
  assert got[2:8] == ["#thread1", "#rung1 advance on thread1", "#bash1", "#read1", "#bash1 exited 0", "#thread1 closed"]
  assert [head for head in got if head[1:2] in ("", " ") or "hi" in head or "a.txt" in head] == []


async def test_every_other_line_of_a_paragraph_is_python() -> None:
  """Every other line of a paragraph is python: a binding of each value that the act tells, and the binding of the act on its open."""
  _, _, root = born("read('n.txt')\nclose(1)", files={"/w/n.txt": "#bash1 exited 0\n\nend\n"})
  assert await engine.thread(int, "read it\nbash1 exited 0\n\nthen close", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  quote = "<s:thread1_markdown>\nread it\nbash1 exited 0\n\nthen close</s:thread1_markdown>"
  assert got[2] == f"#thread1\n{quote}\nthread1: Act[int] = Act('thread1')"
  assert got[4] == "#read1\nread1_path = 'n.txt'\n<s:read1_text>\n#bash1 exited 0\n\nend</s:read1_text>"
  assert got[1] == takes(root)
  for one in got:
    header, *rest = engine.unquoted(one).split("\n")
    assert re.match(r"#\S", header)
    compile("\n".join(rest), "paragraph", "exec")
    assert [line for line in rest if line.startswith("#")] == []


async def test_the_header_of_a_paragraph_names_the_act_it_is_of_by_its_id() -> None:
  """The header of a paragraph names the act it is of by its id, what the act tells and a control over it alike, and the paragraph of a read, a write or a cd stands at the place in the run where it was asked."""
  _, _, root = born("x = bash('echo hi')\nread('n.txt')\ncd('/x')\nclose(1)", files={"/w/n.txt": "one\n"}, auto=False)
  assert await engine.thread(int, "read it", on=root) == 1
  await settle()
  engine.pause("bash1")
  assert heads(engine.turns(on=root))[4:] == ["#bash1", "#read1", "#cd1", "#thread1 closed", "#bash1 paused"]


async def test_the_headers_of_the_file() -> None:
  """The headers of the file are the open of an act, closed, done, exited, raised, debugged, refused, ledger, standing, advance, paused, woke and cancelled."""
  sand, _, root = born()
  ceiling = engine.grant(usd=10.0, on=root)
  await settle()
  sand.script[root] = ["k = BAD", "raise ValueError('boom')", EVERY]
  assert await engine.thread(int, "everything", on=root) == 1
  await settle()
  engine.pause(root)
  engine.wake(root)
  engine.cancel(ceiling)
  step = engine.rung("k = 1", on=root)
  assert await step is None
  await settle()
  assert {spoken(head) for head in headers(engine.turns(on=root))} == EVENTS | {"open"}


async def test_a_statement_that_a_paragraph_shows_binds_the_name_of_an_act_in_the_chain() -> None:
  """A statement or a quote that a paragraph shows binds its name in the chain, and a comment binds nothing."""
  _, log, root = born("x = bash('echo hi')\nclose(1)", "close((await bash1).code)")
  assert await engine.thread(int, "run it\nnow", on=root) == 1
  ack = await acknowledged(log, "bash1")
  assert engine.peek(ack) == 0
  own = [a[5] for a in said(log, "run") if a[4].endswith("_told")]
  asked = "#thread1\n<s:thread1_markdown>\nrun it\nnow</s:thread1_markdown>\nthread1: Act[int] = Act('thread1')"
  told = [
    "#bash1\nbash1_command = 'echo hi'\nbash1: Act[Exit] = Act('bash1')",
    "#thread1 closed\nthread1_value = 1",
    "#bash1 exited 0\nbash1_stdout = 'ran echo hi'",
    f"#{ack} advance on bash1",
  ]
  words = ["\n\n".join([opened(root, "root"), takes(root), asked, "#rung1 advance on thread1"]), "\n\n".join(told)]
  assert own == [engine.unquoted(word) for word in words]
  shown = "\n\n".join(paragraphs(engine.turns(on=root)))
  assert shown == "\n\n".join([*words, f"#{ack} closed\n{ack}_value = 0"])
  assert engine.module(root)["bash1"] == "bash1"
  assert engine.module(root)["thread1_markdown"] == "run it\nnow"
  _, _, fixed = born(
    "k = 1",
    "ok = BAD",
    "mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))",
    "close((k, ok, rung2_findings))",
  )
  assert await engine.thread(object, "fix it", on=fixed) == (1, 2, BAD)


async def test_a_paragraph_binds_each_value_that_it_tells_under_a_name() -> None:
  """A paragraph binds each value that it tells under a name: the id of the act, an underscore, and the word that holds the value, as thread2_markdown, bash1_command, read3_text or thread2_value."""
  _, _, root = born("x = bash('echo hi')\nread('a.txt')\nclose('done')", "close(None)")
  assert await engine.thread(str, "run it", on=root) == "done"
  await settle()
  got = paragraphs(engine.turns(on=root))
  assert "#thread1\nthread1_markdown = 'run it'\nthread1: Act[str] = Act('thread1')" in got
  assert "#bash1\nbash1_command = 'echo hi'\nbash1: Act[Exit] = Act('bash1')" in got
  assert "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>" in got
  assert "#thread1 closed\nthread1_value = 'done'" in got
  bound = [engine.module(root)[name] for name in ("thread1_markdown", "bash1_command", "read1_text", "thread1_value")]
  assert bound == ["run it", "echo hi", "one\ntwo", "done"]


async def test_a_paragraph_binds_a_string_of_more_than_one_line_as_a_quote() -> None:
  """A paragraph binds a string of more than one line as a quote, and any other value as a statement of its repr."""
  sand, _, root = born("x = bash('echo a\\necho b')\nclose('one\\ntwo')", "close(None)")
  assert await engine.thread(str, "run\nthem", on=root) == "one\ntwo"
  sand.script[root] = ["close([1, 'b', Refused('no')])", "close(None)"]
  one = engine.thread(list, "which?", on=root)
  assert (await one)[:2] == [1, "b"]
  sand.script[root] = ["close(1)"]
  assert await engine.thread(int, "then bind it", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  assert "#thread1\n<s:thread1_markdown>\nrun\nthem</s:thread1_markdown>\nthread1: Act[str] = Act('thread1')" in got
  assert "#bash1\n<s:bash1_command>\necho a\necho b</s:bash1_command>\nbash1: Act[Exit] = Act('bash1')" in got
  assert "#thread1 closed\n<s:thread1_value>\none\ntwo</s:thread1_value>" in got
  assert f"#{one} closed\n{one}_value = [1, 'b', Refused('no')]" in got
  bound = [engine.module(root)[name] for name in ("thread1_markdown", "bash1_command", "thread1_value")]
  assert bound == ["run\nthem", "echo a\necho b", "one\ntwo"]
  assert engine.module(root)[f"{one}_value"][:2] == [1, "b"]


async def test_a_value_that_is_none_or_an_empty_string_is_not_told() -> None:
  """A value that is None or an empty string is not told."""
  _, _, root = born()
  bare = engine.thread(None, to=OPERATOR, on=root)
  engine.close(None, bare)
  empty = engine.thread(str, "say nothing", to=OPERATOR, on=root)
  engine.close("", empty)
  zero = engine.thread(int, "count nothing", to=OPERATOR, on=root)
  engine.close(0, zero)
  await settle()
  got = paragraphs(engine.turns(on=root))[2:]
  assert got == [
    f"#{bare}\n{bare}: Act[None] = Act({bare!r})",
    f"#{bare} closed",
    f"#{empty}\n{empty}_markdown = 'say nothing'\n{empty}: Act[str] = Act({empty!r})",
    f"#{empty} closed",
    f"#{zero}\n{zero}_markdown = 'count nothing'\n{zero}: Act[int] = Act({zero!r})",
    f"#{zero} closed\n{zero}_value = 0",
  ]


async def test_a_quote_that_a_paragraph_tells_takes_one_more_underscore_while_its_string_holds_its_close_mark() -> None:
  """A quote that a paragraph tells takes one more underscore in its name for as long as its string holds the close mark of that name at the end of a line, so the first close mark after it is its own."""
  first, second = "a</s:thread1_markdown>\nb</s:thread1_markdown_>\nc", "d</s:thread1_markdown__>\ne"
  _, _, root = born("close(None)")
  engine.thread(None, first, to=OPERATOR, on=root)
  engine.thread(None, second, to=OPERATOR, on=root)
  assert await engine.thread(None, "go", on=root) is None
  got = paragraphs(engine.turns(on=root))
  assert (
    got[2] == f"#thread1\n<s:thread1_markdown__>\n{first}</s:thread1_markdown__>\nthread1: Act[None] = Act('thread1')"
  )
  assert got[3] == f"#thread2\n<s:thread2_markdown>\n{second}</s:thread2_markdown>\nthread2: Act[None] = Act('thread2')"
  assert [engine.module(root)[name] for name in ("thread1_markdown__", "thread2_markdown")] == [first, second]


async def test_the_engine_applies_a_show_before_it_writes_a_line() -> None:
  """The engine applies a show before it writes a line, so a quote holds the lines that its show picked, as they are, and no number of a line."""
  _, _, root = born(
    "read('n.txt', span(2, 3))\nread('n.txt', grep('^o'))\nclose(1)", files={"/w/n.txt": "one\ntwo\nthree\n"}
  )
  assert await engine.thread(int, "read parts", on=root) == 1
  got = [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#read")]
  assert got == [
    "#read1\nread1_path = 'n.txt'\n<s:read1_text>\ntwo\nthree</s:read1_text>",
    "#read2\nread2_path = 'n.txt'\nread2_text = 'one'",
  ]


async def test_a_show_applies_to_a_text_or_to_a_stream() -> None:
  """A show applies to a text or to a stream."""
  sand, log, root = born("read('n.txt', span(1, 1))\nclose(1)", files={"/w/n.txt": "one\ntwo\nthree\n"}, auto=False)
  assert await engine.thread(int, "read it", on=root) == 1
  act = engine.bash("many", show=span(1, 1), on=root)
  await settle()
  command = said(log, "bash")[0][1]
  world_says("out", command, "a\nb\nc\n", "stdout")
  sand.exits(command, 0)
  assert (await act).code == 0
  got = paragraphs(engine.turns(on=root))
  assert "#read1\nread1_path = 'n.txt'\nread1_text = 'one'" in got
  assert f"#{command} exited 0\n{command}_stdout = 'a'" in got
