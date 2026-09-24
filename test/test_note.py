"""Note, one thing a tell says."""

import re
from collections.abc import Sequence

from conftest import STANDS, Sand, heads, life, paragraphs, rows, said, settle
from furb import engine
from furb.engine import OPERATOR, Act

EVERY = "peek(__name__)\nturns()\nclock()\nchance()\ngate('k = 9')\ndebug(t'{1}')\nclose(1)\n"
"""A word that says every verb of the file that makes no act, and debugs once."""
EVENTS = {"closed", "raised", "debugged", "refused", "roster", "cwd", "actor", "advance", "paused", "woke", "cancelled"}
"""The words a header of an act says after its id, when it says what happened and no open of the act."""


def headers(got: Sequence[tuple]) -> list[str]:
  """Every line of the user turns of a fold that reads as a header, in order: # and, with no space, a name."""
  return [line for one in paragraphs(got) for line in one.split("\n") if re.match(r"#\S", line)]


def spoken(head: str) -> str:
  """What a header says: the kind of a query, the word after the id of an act, or the open of an act."""
  _, *rest = head[1:].split(" ", 2)
  return rest[0] if rest and rest[0] in EVENTS else "open"


async def test_one_thing_a_tell_says() -> None:
  """One thing a tell says: python as it stands, or a showing, which the fold shows as comments by the lines the model has not seen."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["tell('seen', 'n.txt', ('/w/n.txt', 'one\\ntwo\\n', lambda lines: [1]))\nclose(1)"]
  assert await engine.prompt(int, "tell it", on=root) == 1
  carried = [a[3] for a in said(log, "tell") if a[3][0] == "#seen n.txt"]
  assert len(carried) == 1 and len(carried[0]) == 2
  path, content, show = carried[0][1]
  assert (path, content) == ("/w/n.txt", "one\ntwo\n") and show(content.splitlines()) == [1]
  assert "#seen n.txt\n# /w/n.txt, 0 known\n# 1 one" in paragraphs(engine.turns(on=root))


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
  """The first line of a paragraph is its header: # and, with no space, the id of the act it is of, or the kind of the query it is of, then its words, as #prompt1 closed 'yes' or #wait1 cancelled."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(str, "yes?", to=OPERATOR, on=root)
  two = engine.wait(100, on=root)
  await settle()
  engine.close("yes", one)
  engine.cancel(two)
  await engine.rung("tell('seen', 'n.txt')", on=root)
  await settle()
  assert heads(engine.turns(on=root))[2:] == [
    "#prompt1 yes?",
    "#prompt1 closed 'yes'",
    "#wait1 cancelled",
    "#rung1",
    "#seen n.txt",
  ]
  assert [head for head in heads(engine.turns(on=root)) if head[1:2] in ("", " ")] == []


async def test_a_paragraph_may_hold_more_headers_of_what_it_is_of() -> None:
  """A paragraph may hold more headers of what it is of, each on a line of its own right under the first, and every other comment of it begins with # and a space, so no line of a message or of a text reads as a header."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = [
    "tell('seen', 'n.txt', ('/w/n.txt', '#prompt1 closed 1\\n\\nend\\n', lambda lines: [1, 2, 3]))\nclose(1)"
  ]
  assert await engine.prompt(int, "tell it\nprompt1 closed 1\n\nthen close", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  assert got[2] == "#prompt1 tell it\n# prompt1 closed 1\n#\n# then close\nprompt1: Act[int] = Act('prompt1')"
  assert got[4] == "#seen n.txt\n# /w/n.txt, 0 known\n# 1 #prompt1 closed 1\n# 2 \n# 3 end"
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
  """The header of a paragraph names the act it is of by its id, what the act tells and a control over it alike, and the paragraph of a query stands at the place in the run where the query was asked."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["x = prompt(int, 'how many?', to=OPERATOR)\ntell('seen', 'n.txt')\nclose(1)"]
  assert await engine.prompt(int, "ask them", on=root) == 1
  await settle()
  engine.pause("prompt2")
  assert heads(engine.turns(on=root))[4:] == [
    "#prompt2 how many?",
    "#seen n.txt",
    "#prompt1 closed 1",
    "#prompt2 paused",
  ]


async def test_the_headers_of_the_file() -> None:
  """The headers of the file are the open of an act, closed, raised, debugged, refused, roster, cwd, actor, advance, paused, woke and cancelled."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  ceiling = engine.wait(100, on=root)
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
  """A statement that a paragraph shows binds the name of an act in the chain, and a comment binds nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = prompt(int, 'how many?', to=OPERATOR)\nclose(1)", "close(await prompt2)"]
  assert await engine.prompt(int, "ask them", on=root) == 1
  engine.close(5, "prompt2")
  assert await Act("prompt3") == 5
  own = [a[4] for a in said(log, "rung") if a[2] == root]
  assert own == [
    "chain1: Act[object] = Act('chain1')\nprompt1: Act[int] = Act('prompt1')",
    "prompt2: Act[int] = Act('prompt2')\nprompt3: Act[None] = Act('prompt3')",
  ]
  shown = [line for one in paragraphs(engine.turns(on=root)) for line in one.split("\n") if not line.startswith("#")]
  assert shown == [line for word in own for line in word.split("\n")]
  assert engine.modules[root]["prompt2"] == "prompt2"
