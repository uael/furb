"""Note, one thing a tell says."""

import re
from collections.abc import Sequence

from conftest import BAD, born, heads, opened, paragraphs, said, settle, takes, world_says
from furb import engine
from furb.engine import OPERATOR, Act, span

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
  "cwd()\n"
  "debug(t'{1}')\n"
  "close(1)\n"
)
EVENTS = {
  "closed",
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
  assert await engine.prompt(int, "read it", on=root) == 1
  read = [a[3] for a in said(log, "tell") if a[3][0].startswith("#read1")]
  assert read == [["#read1\nread1_path = 'n.txt'\nread1_text = 'one'"]]
  opened = [a[3] for a in said(log, "tell") if a[1] == "prompt1"]
  assert opened == [["#prompt1\nprompt1_message = 'read it'", "prompt1: Act[int] = Act('prompt1')"]]
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
  """The first line of a paragraph is its header: # and, with no space, the id of the act it is of, then what happened to the act, as #bash1 exited 0 or #prompt1, and no text that the act tells."""
  _, _, root = born("x = bash('echo hi')\nread('a.txt')\nclose((await x).code)")
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  got = heads(engine.turns(on=root))
  assert got[2:8] == ["#prompt1", "#rung1 advance on prompt1", "#bash1", "#read1", "#bash1 exited 0", "#prompt1 closed"]
  assert [head for head in got if head[1:2] in ("", " ") or "hi" in head or "a.txt" in head] == []


async def test_every_other_line_of_a_paragraph_is_python() -> None:
  """Every other line of a paragraph is python: a binding of each value that the act tells, and the binding of the act on its open."""
  _, _, root = born("read('n.txt')\nclose(1)", files={"/w/n.txt": "#bash1 exited 0\n\nend\n"})
  assert await engine.prompt(int, "read it\nbash1 exited 0\n\nthen close", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  quote = "<s:prompt1_message>\nread it\nbash1 exited 0\n\nthen close</s:prompt1_message>"
  assert got[2] == f"#prompt1\n{quote}\nprompt1: Act[int] = Act('prompt1')"
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
  assert await engine.prompt(int, "read it", on=root) == 1
  await settle()
  engine.pause("bash1")
  assert heads(engine.turns(on=root))[4:] == ["#bash1", "#read1", "#cd1", "#prompt1 closed", "#bash1 paused"]


async def test_the_headers_of_the_file() -> None:
  """The headers of the file are the open of an act, closed, exited, raised, debugged, refused, ledger, standing, advance, paused, woke and cancelled."""
  sand, _, root = born()
  ceiling = engine.grant(usd=10.0, on=root)
  await settle()
  sand.script[root] = ["k = BAD", "raise ValueError('boom')", EVERY]
  assert await engine.prompt(int, "everything", on=root) == 1
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
  assert await engine.prompt(int, "run it\nnow", on=root) == 1
  assert await Act("prompt2") == 0
  own = [a[4] for a in said(log, "rung") if a[2] == root]
  assert own == [
    "\n".join(
      [
        *[line for one in (opened(root, "root"), takes(root)) for line in one.split("\n")[1:]],
        "prompt1_message = 'run it\\nnow'\nprompt1: Act[int] = Act('prompt1')",
      ]
    ),
    "bash1_command = 'echo hi'\nbash1: Act[Exit] = Act('bash1')\nprompt1_value = 1\n"
    "prompt2_message = 'bash1 done'\nprompt2: Act[None] = Act('prompt2')\nbash1_stdout = 'ran echo hi'",
  ]
  shown = [
    line
    for one in paragraphs(engine.turns(on=root))
    for line in engine.unquoted(one).split("\n")
    if line and line[0] != "#"
  ]
  assert shown == [line for word in own for line in word.split("\n")] + ["prompt2_value = 0"]
  assert engine.module(root)["bash1"] == "bash1"
  assert engine.module(root)["prompt1_message"] == "run it\nnow"
  _, _, fixed = born(
    "k = 1",
    "ok = BAD",
    "mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))",
    "close((k, ok, rung3_findings))",
  )
  assert await engine.prompt(object, "fix it", on=fixed) == (1, 2, BAD)


async def test_a_paragraph_binds_each_value_that_it_tells_under_a_name() -> None:
  """A paragraph binds each value that it tells under a name: the id of the act, an underscore, and the word that holds the value, as prompt2_message, bash1_command, read3_text or prompt2_value."""
  _, _, root = born("x = bash('echo hi')\nread('a.txt')\nclose('done')", "close(None)")
  assert await engine.prompt(str, "run it", on=root) == "done"
  await settle()
  got = paragraphs(engine.turns(on=root))
  assert "#prompt1\nprompt1_message = 'run it'\nprompt1: Act[str] = Act('prompt1')" in got
  assert "#bash1\nbash1_command = 'echo hi'\nbash1: Act[Exit] = Act('bash1')" in got
  assert "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>" in got
  assert "#prompt1 closed\nprompt1_value = 'done'" in got
  bound = [engine.module(root)[name] for name in ("prompt1_message", "bash1_command", "read1_text", "prompt1_value")]
  assert bound == ["run it", "echo hi", "one\ntwo", "done"]


async def test_a_paragraph_binds_a_string_of_more_than_one_line_as_a_quote() -> None:
  """A paragraph binds a string of more than one line as a quote, and any other value as a statement of its repr."""
  sand, _, root = born("x = bash('echo a\\necho b')\nclose('one\\ntwo')", "close(None)")
  assert await engine.prompt(str, "run\nthem", on=root) == "one\ntwo"
  sand.script[root] = ["close([1, 'b', Refused('no')])", "close(None)"]
  one = engine.prompt(list, "which?", on=root)
  assert (await one)[:2] == [1, "b"]
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "then bind it", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  assert "#prompt1\n<s:prompt1_message>\nrun\nthem</s:prompt1_message>\nprompt1: Act[str] = Act('prompt1')" in got
  assert "#bash1\n<s:bash1_command>\necho a\necho b</s:bash1_command>\nbash1: Act[Exit] = Act('bash1')" in got
  assert "#prompt1 closed\n<s:prompt1_value>\none\ntwo</s:prompt1_value>" in got
  assert f"#{one} closed\n{one}_value = [1, 'b', Refused('no')]" in got
  bound = [engine.module(root)[name] for name in ("prompt1_message", "bash1_command", "prompt1_value")]
  assert bound == ["run\nthem", "echo a\necho b", "one\ntwo"]
  assert engine.module(root)[f"{one}_value"][:2] == [1, "b"]


async def test_a_value_that_is_none_or_an_empty_string_is_not_told() -> None:
  """A value that is None or an empty string is not told."""
  _, _, root = born()
  bare = engine.prompt(None, to=OPERATOR, on=root)
  engine.close(None, bare)
  empty = engine.prompt(str, "say nothing", to=OPERATOR, on=root)
  engine.close("", empty)
  zero = engine.prompt(int, "count nothing", to=OPERATOR, on=root)
  engine.close(0, zero)
  await settle()
  got = paragraphs(engine.turns(on=root))[2:]
  assert got == [
    f"#{bare}\n{bare}: Act[None] = Act({bare!r})",
    f"#{bare} closed",
    f"#{empty}\n{empty}_message = 'say nothing'\n{empty}: Act[str] = Act({empty!r})",
    f"#{empty} closed",
    f"#{zero}\n{zero}_message = 'count nothing'\n{zero}: Act[int] = Act({zero!r})",
    f"#{zero} closed\n{zero}_value = 0",
  ]


async def test_a_quote_that_a_paragraph_tells_takes_one_more_underscore_while_its_string_holds_its_close_mark() -> None:
  """A quote that a paragraph tells takes one more underscore in its name for as long as its string holds the close mark of that name at the end of a line, so the first close mark after it is its own."""
  first, second = "a</s:prompt1_message>\nb</s:prompt1_message_>\nc", "d</s:prompt1_message__>\ne"
  _, _, root = born("close(None)")
  engine.prompt(None, first, to=OPERATOR, on=root)
  engine.prompt(None, second, to=OPERATOR, on=root)
  assert await engine.prompt(None, "go", on=root) is None
  got = paragraphs(engine.turns(on=root))
  assert (
    got[2] == f"#prompt1\n<s:prompt1_message__>\n{first}</s:prompt1_message__>\nprompt1: Act[None] = Act('prompt1')"
  )
  assert got[3] == f"#prompt2\n<s:prompt2_message>\n{second}</s:prompt2_message>\nprompt2: Act[None] = Act('prompt2')"
  assert [engine.module(root)[name] for name in ("prompt1_message__", "prompt2_message")] == [first, second]


async def test_the_engine_applies_a_show_before_it_writes_a_line() -> None:
  """The engine applies a show before it writes a line, so a quote holds the lines that its show picked, as they are, and no number of a line."""
  _, _, root = born(
    "read('n.txt', span(2, 3))\nread('n.txt', grep('^o'))\nclose(1)", files={"/w/n.txt": "one\ntwo\nthree\n"}
  )
  assert await engine.prompt(int, "read parts", on=root) == 1
  got = [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#read")]
  assert got == [
    "#read1\nread1_path = 'n.txt'\n<s:read1_text>\ntwo\nthree</s:read1_text>",
    "#read2\nread2_path = 'n.txt'\nread2_text = 'one'",
  ]


async def test_a_show_applies_to_a_text_or_to_a_stream() -> None:
  """A show applies to a text or to a stream."""
  sand, log, root = born("read('n.txt', span(1, 1))\nclose(1)", files={"/w/n.txt": "one\ntwo\nthree\n"}, auto=False)
  assert await engine.prompt(int, "read it", on=root) == 1
  act = engine.bash("many", show=span(1, 1), on=root)
  await settle()
  command = said(log, "bash")[0][1]
  world_says("out", command, "a\nb\nc\n", "stdout")
  sand.exits(command, 0)
  assert (await act).code == 0
  got = paragraphs(engine.turns(on=root))
  assert "#read1\nread1_path = 'n.txt'\nread1_text = 'one'" in got
  assert f"#{command} exited 0\n{command}_stdout = 'a'" in got
