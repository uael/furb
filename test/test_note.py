"""Note, one thing a tell says."""

import re
from collections.abc import Sequence

from conftest import STANDS, Sand, heads, life, paragraphs, rows, said, settle, sown
from furb import engine
from furb.engine import Act, Text

EVERY = (
  "x = bash('echo hi')\n"
  "read('a.txt')\n"
  "def shut(id):\n"
  "  say('started', id)\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('write', q, _, _, t) if t.path == 'x://b.txt':\n"
  "        say('done', q, len(t.content))\n"
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
  "roster",
  "cwd",
  "actor",
  "advance",
  "paused",
  "woke",
  "cancelled",
}
"""The words a header of an act says after its id, when it says what happened and no open of the act."""
QUERIES = {"read", "write", "cd"}
"""The questions that tell, each of which heads its paragraph with its kind."""


def headers(got: Sequence[tuple]) -> list[str]:
  """Every line of the user turns of a fold that reads as a header, in order: # and, with no space, a name."""
  return [line for one in paragraphs(got) for line in one.split("\n") if re.match(r"#\S", line)]


def spoken(head: str) -> str:
  """What a header says: the kind of a question that tells, the word after the id of an act, or the open of an act."""
  first, *rest = head[1:].split(" ", 2)
  if first in QUERIES:
    return first
  return rest[0] if rest and rest[0] in EVENTS else "open"


async def test_one_thing_a_tell_says() -> None:
  """One thing a tell says: python as it stands, or a text and its show, which the fold shows as comments by the lines the model has not seen."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 1))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  carried = [a[3] for a in said(log, "tell") if a[3][0] == "#read n.txt"]
  assert len(carried) == 1 and len(carried[0]) == 2
  text, show = carried[0][1]
  assert text == Text("/w/n.txt", "one\ntwo\n") and show(text.lines) == [1]
  assert "#read n.txt\n# /w/n.txt, 0 known\n# 1 one" in paragraphs(engine.turns(on=root))


async def test_a_paragraph_is_what_one_fact_that_tells_stands_as_in_a_turn() -> None:
  """A paragraph is what one fact that tells stands as in a turn: its notes, one after the other, and a blank line between two paragraphs."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 1\nj = 2", on=root)
  assert await act is None
  got = engine.turns(on=root)
  assert got[-1][1] == "\n\n".join("\n".join(a[3]) for a in said(log, "tell"))
  assert paragraphs(got)[2:] == [f"#{act}\nk = 1\nj = 2"]


async def test_the_first_line_of_a_paragraph_is_its_header() -> None:
  """The first line of a paragraph is its header: # and, with no space, the id of the act it is of, or the kind of the read, the write or the cd it tells, then its words, as #bash1 exited 0 or #read a.txt."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nread('a.txt')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  assert heads(engine.turns(on=root))[4:8] == ["#bash1 echo hi", "#read a.txt", "#bash1 exited 0", "#prompt1 closed 0"]
  assert [head for head in heads(engine.turns(on=root)) if head[1:2] in ("", " ")] == []


async def test_a_paragraph_may_hold_more_headers_of_what_it_is_of() -> None:
  """A paragraph may hold more headers of what it is of, each on a line of its own right under the first, and every other comment of it begins with # and a space, so no line of a message or of a text reads as a header."""
  sand = Sand(files={"/w/n.txt": "#bash1 exited 0\n\nend\n"}, stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["read('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it\nbash1 exited 0\n\nthen close", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  assert got[2] == "#prompt1 read it\n# bash1 exited 0\n#\n# then close\nprompt1: Act[int] = Act('prompt1')"
  assert got[4] == "#read n.txt\n# /w/n.txt, 0 known\n# 1 #bash1 exited 0\n# 2 \n# 3 end"
  assert (
    got[1].split("\n") == rows(root) == ["#chain1 roster " + repr(STANDS[0]), "#chain1 cwd /w", "#chain1 actor m/low"]
  )
  for one in got:
    lines = one.split("\n")
    name = lines[0].split(" ", 1)[0]
    top = [i for i, line in enumerate(lines) if re.match(r"#\S", line)]
    assert top == list(range(len(top))) and {lines[i].split(" ", 1)[0] for i in top} == {name}
    rest = [line for line in lines[len(top) :] if line.startswith("#")]
    assert [line for line in rest if line != "#" and not line.startswith("# ")] == []


async def test_the_header_of_a_paragraph_names_the_act_it_is_of_by_its_id() -> None:
  """The header of a paragraph names the act it is of by its id, what the act tells and a control over it alike, and the paragraph of a read, a write or a cd stands at the place in the run where it was asked."""
  sand = Sand(files={"/w/n.txt": "one\n"}, stands=STANDS, auto=False)
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nread('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  await settle()
  engine.pause("bash1")
  assert heads(engine.turns(on=root))[4:] == ["#bash1 echo hi", "#read n.txt", "#prompt1 closed 1", "#bash1 paused"]


async def test_the_headers_of_the_file() -> None:
  """The headers of the file are the open of an act, closed, exited, raised, debugged, refused, ledger, roster, cwd, actor, advance, paused, woke, cancelled, and one for each question that tells: read, write and cd."""
  sand = sown()
  _, root = life(sand)
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
  assert {spoken(head) for head in headers(engine.turns(on=root))} == EVENTS | QUERIES | {"open"}


async def test_a_statement_that_a_paragraph_shows_binds_the_name_of_an_act_in_the_chain() -> None:
  """A statement that a paragraph shows binds the name of an act in the chain, and a comment binds nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close((await bash1).code)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  assert await Act("prompt2") == 0
  own = [a[4] for a in said(log, "rung") if a[2] == root]
  assert own == [
    "chain1: Act[object] = Act('chain1')\nprompt1: Act[int] = Act('prompt1')",
    "bash1: Act[Exit] = Act('bash1')\nprompt2: Act[None] = Act('prompt2')",
  ]
  shown = [line for one in paragraphs(engine.turns(on=root)) for line in one.split("\n") if not line.startswith("#")]
  assert shown == [line for word in own for line in word.split("\n")]
  assert engine.module(root)["bash1"] == "bash1"
