"""read, the text at a path."""

import pytest

from conftest import STANDS, Dead, Sand, life, of, paragraphs, said, settle, sown
from furb import engine
from furb.engine import HEAD, HIDDEN, OPERATOR, Refused, Text, span, take

BIG = "".join(f"line {i}\n" for i in range(1, 2101))
"""A text of two thousand and one hundred lines, which is longer than HEAD."""
NUMS = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('nums://'):\n"
  "        say('done', qid, [1, 2])\n"
  "\n"
  "act('nums', '', kept)\n"
  "close(read('nums://a'))\n"
)
"""A word of a rung that opens a door of its own, which answers a read with a list, and gives what it read."""


async def test_a_read_whoever_serves_the_path_answers_it_with_the_text_of_it() -> None:
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines the model has not seen."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nsame = read('a.txt')\nclose([len(t.lines), same.content])"]
  assert await engine.prompt(list, "read them", on=root) == [2, "one\ntwo\n"]
  assert of(engine.turns(on=root), "read") == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two",
    "#read a.txt\n# /w/a.txt, 2 known",
  ]


async def test_a_read_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A read that the World refuses raises Refused in the caller."""
  dead = Dead(stands=STANDS)
  _, root = life(dead)
  with pytest.raises(Refused, match="a dead World answers no read"):
    engine.read("a.txt", on=root)
  word = "try:\n  read('a.txt')\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "a dead World answers no read"


async def test_a_read_on_a_chain_with_a_source_tells_the_lines_of_a_skipped_read_again() -> None:
  """A read on a chain with a source tells the lines of a skipped read again, since they are not known there."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('a.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  step = said(log, "rung")[0][1]
  fork = engine.chain("fork", source=root, filter=take(step, inside=False))
  await settle(300)
  assert of(engine.turns(on=fork), "read") == []
  sand.script[fork] = ["read('a.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it again", on=fork) == 2
  sand.script[root] = ["read('a.txt')\nclose(3)"]
  assert await engine.prompt(int, "read it again", on=root) == 3
  assert of(engine.turns(on=fork), "read") == ["#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two"]
  assert of(engine.turns(on=root), "read") == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two",
    "#read a.txt\n# /w/a.txt, 2 known",
  ]


async def test_the_engine_judges_no_scheme() -> None:
  """The engine judges no scheme, so a path of an unknown scheme goes to the World too."""
  sand = Sand(files={"weird://x": "kept\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.read("weird://x", on=root) == Text("weird://x", "kept\n")
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x"]
  with pytest.raises(Refused, match="nothing takes read"):
    engine.read("weird://y", on=root)
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x", "weird://y"]


async def test_read_is_given_a_path_and_a_show() -> None:
  """read is given a path and a show."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('a.txt', span(2, 2))\nread('a.txt', grep('^o'))\nclose(1)"]
  assert await engine.prompt(int, "read them", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 2 two",
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one",
  ]


async def test_read_gives_a_text() -> None:
  """read gives a Text."""
  sand = sown()
  _, root = life(sand)
  got = engine.read("a.txt", on=root)
  assert isinstance(got, Text) and got == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("a.txt", span(1, 1), on=root) == Text("/w/a.txt", "one\ntwo\n")


async def test_a_text_without_a_show_is_told_as_head() -> None:
  """A text without a show is told as HEAD, which is the span of its first 2000 lines."""
  assert HEAD(BIG.splitlines()) == span(1, 2000)(BIG.splitlines()) == list(range(1, 2001))
  sand = Sand(files={"/w/big.txt": BIG}, stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["read('big.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  told = ["#read big.txt", "# /w/big.txt, 0 known", *[f"# {i} line {i}" for i in range(1, 2001)]]
  assert of(engine.turns(on=root), "read") == ["\n".join(told)]


async def test_a_read_of_the_name_of_a_prompt_gives_the_program_of_that_ladder() -> None:
  """A read of the name of a prompt gives the program of that ladder, the words of its rungs in order."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["k = BAD", "a = 1", "close(a + 1)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  assert engine.read(act, on=root) == Text(act, "k = BAD\na = 1\nclose(a + 1)")


async def test_whether_a_name_is_one_of_the_prompts_a_chain_has_heard_on_itself() -> None:
  """Whether a name is one of the prompts a chain has heard on itself, which is what its doors serve and no other path, a path of no name being none of them."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  assert engine.read(act, on=root) == Text(act, "a = 1\nclose(a + 1)")
  other = engine.chain("other")
  await settle()
  for path, on in [
    (act, other),
    *[(one, root) for one in ("", "nowhere://x", root, *[a[1] for a in said(log, "rung")])],
  ]:
    with pytest.raises(Refused, match="nothing takes read"):
      engine.read(path, on=on)


async def test_a_prompt_is_the_door_of_the_program_of_its_ladder() -> None:
  """A prompt is the door of the program of its ladder, so a read of its name gives the word of every rung of it in order, the words the gate refused among them, which are none at all for a prompt that ran no word."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["x = BAD", "x = bash('echo hi')\nclose(1)", "close(None)"]
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  assert engine.read(act, on=root) == Text(act, "x = BAD\nx = bash('echo hi')\nclose(1)")
  bare = engine.prompt(int, "later", to=OPERATOR, on=root)
  assert engine.read(bare, on=root) == Text(bare, "")


async def test_a_read_of_a_door_that_a_rung_of_that_ladder_says_leaves_the_word_of_that_rung_out() -> None:
  """A read of a door that a rung of that ladder says leaves the word of that rung out, since a rung is no part of the program it reads."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = 1", "close(read(get(acting())[2]).content)", "close(None)"]
  act = engine.prompt(str, "read it", on=root)
  assert await act == "a = 1"
  await settle()
  assert of(engine.turns(on=root), "read") == [f"#read {act}\n# {act}, 0 known\n# 1 a = 1"]
  assert engine.read(act, on=root) == Text(act, "a = 1\nclose(read(get(acting())[2]).content)")


async def test_one_made_from_inside_an_act_tells_itself() -> None:
  """One made from inside an act tells itself, with its path and what it was answered, on the scope of that act; one made from outside an act tells nothing, and neither does one whose show is hidden."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('a.txt')\nread('a.txt', HIDDEN)\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  step = said(log, "rung")[0][1]
  told = [a for a in said(log, "tell") if a[3][0].startswith("#read ")]
  assert [(a[1], a[2], a[3][0], a[3][1][0]) for a in told] == [
    (step, step, "#read a.txt", Text("/w/a.txt", "one\ntwo\n"))
  ]
  held = engine.transcript(root)
  assert engine.scope(step) == root and told[0] in held
  was = engine.turns(on=root)
  assert of(was, "read") == ["#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two"]
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("a.txt", HIDDEN, on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert paragraphs(engine.turns(on=root)) == paragraphs(was)


async def test_a_read_answered_with_what_is_no_text_gives_that_value() -> None:
  """A read answered with what is no text gives that value, and tells it as python shows it."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [NUMS]
  assert await engine.prompt(list, "a door of my own", on=root) == [1, 2]
  assert of(engine.turns(on=root), "read") == ["#read nums://a\n# [1, 2]"]
