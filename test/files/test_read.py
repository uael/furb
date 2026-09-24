"""read, the text at a path."""

import pytest

from conftest import FILES, STANDS, Dead, Sand, life, of, paragraphs, said, settle, sown, texted, verb
from furb import engine
from furb.engine import OPERATOR, Refused, take

BIG = "".join(f"line {i}\n" for i in range(1, 2101))
"""A text of two thousand and one hundred lines, which is longer than HEAD."""
NUMS = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('nums://'):\n"
  "        yield 'done', qid, [1, 2]\n"
  "\n"
  "act('nums', '', kept)\n"
  "close(read('nums://a'))\n"
)
"""A word of a rung that opens a door of its own, which answers a read with a list, and gives what it read."""


async def test_a_read_whoever_serves_the_path_answers_it_with_the_text_of_it() -> None:
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines the model has not seen."""
  sand = sown(FILES)
  _, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nsame = read('a.txt')\nclose([len(t.lines), same.content])"]
  assert await engine.prompt(list, "read them", on=root) == [2, "one\ntwo\n"]
  assert of(engine.turns(on=root), "read") == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two",
    "#read a.txt\n# /w/a.txt, 2 known",
  ]


async def test_a_read_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A read that the World refuses raises Refused in the caller."""
  dead = Dead(stands=STANDS, words=FILES)
  _, root = life(dead)
  with pytest.raises(Refused, match="a dead World answers no read"):
    verb("read", root)("a.txt")
  word = "try:\n  read('a.txt')\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "a dead World answers no read"


async def test_a_read_on_a_chain_with_a_source_tells_the_lines_of_a_skipped_read_again() -> None:
  """A read on a chain with a source tells the lines of a skipped read again, since they are not known there."""
  sand = sown(FILES)
  log, root = life(sand)
  sand.script[root] = ["read('a.txt')\nclose(1)"]
  one = engine.prompt(int, "read it", on=root)
  assert await one == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == one]
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


async def test_read_judges_no_scheme() -> None:
  """read judges no scheme, so a path of an unknown scheme goes to the World too."""
  sand = Sand(files={"weird://x": "kept\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  assert texted(verb("read", root)("weird://x")) == ("weird://x", "kept\n")
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x"]
  assert verb("read", root)("weird://y") is None
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x", "weird://y"]


async def test_read_is_given_a_path_and_a_show() -> None:
  """read is given a path and a show."""
  sand = sown(FILES)
  _, root = life(sand)
  sand.script[root] = ["read('a.txt', span(2, 2))\nread('a.txt', grep('^o'))\nclose(1)"]
  assert await engine.prompt(int, "read them", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 2 two",
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one",
  ]


async def test_read_gives_a_text() -> None:
  """read gives a Text."""
  sand = sown(FILES)
  _, root = life(sand)
  word = "t = read('a.txt')\nu = read('a.txt', span(1, 1))\nclose([type(t) is Text, t.path, t.content, u == t])"
  assert await engine.rung(word, on=root) == [True, "/w/a.txt", "one\ntwo\n", True]
  assert texted(verb("read", root)("a.txt")) == ("/w/a.txt", "one\ntwo\n")


async def test_a_text_without_a_show_is_told_as_head() -> None:
  """A text without a show is told as HEAD, which is the span of its first 2000 lines."""
  sand = Sand(files={"/w/big.txt": BIG}, stands=STANDS, words=FILES)
  _, root = life(sand)
  word = f"close(HEAD({BIG.splitlines()!r}) == span(1, 2000)({BIG.splitlines()!r}) == list(range(1, 2001)))"
  assert await engine.rung(word, on=root) is True
  sand.script[root] = ["read('big.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  told = ["#read big.txt", "# /w/big.txt, 0 known", *[f"# {i} line {i}" for i in range(1, 2001)]]
  assert of(engine.turns(on=root), "read") == ["\n".join(told)]


async def test_a_read_of_a_door_of_a_ladder_asks_its_chain_for_the_ladder() -> None:
  """A read of a door of a ladder asks its chain for the ladder, and gives the program of that ladder as a text of that path, so the World is asked nothing."""
  sand = sown(FILES)
  _, root = life(sand)
  sand.script[root] = ["k = BAD", "a = 1", "close(a + 1)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  assert texted(verb("read", root)(act)) == (act, "k = BAD\na = 1\nclose(a + 1)")
  (asked,) = [a for a in engine.asked.values() if a[0] == "ladder"]
  assert asked[2:] == (OPERATOR, root, act) and engine.outcomes[asked[1]] == "k = BAD\na = 1\nclose(a + 1)"
  assert said(sand.calls, "read") == []


async def test_one_asked_from_inside_an_act_tells_itself() -> None:
  """One asked from inside an act tells itself, with its path and what it was answered, on the scope of that act; one asked from outside an act tells nothing, and neither does one whose show is hidden."""
  sand = sown(FILES)
  log, root = life(sand)
  sand.script[root] = ["read('a.txt')\nread('a.txt', HIDDEN)\nclose(1)"]
  one = engine.prompt(int, "read it", on=root)
  assert await one == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == one]
  told = [a for a in said(log, "tell") if a[3][0].startswith("#read ")]
  assert [(a[1], a[2], a[3][0], a[3][1][:2]) for a in told] == [(step, step, "#read a.txt", ("/w/a.txt", "one\ntwo\n"))]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert engine.scope(step) == root
  assert [(a[1], a[3][0]) for a in held if a[0] == "tell" and a[3][0].startswith("#read ")] == [(step, "#read a.txt")]
  was = engine.turns(on=root)
  assert of(was, "read") == ["#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two"]
  assert texted(verb("read", root)("a.txt")) == ("/w/a.txt", "one\ntwo\n")
  assert paragraphs(engine.turns(on=root)) == paragraphs(was)


async def test_a_read_answered_with_what_is_no_text_gives_that_value() -> None:
  """A read answered with what is no text gives that value, and tells it as python shows it."""
  sand = sown(FILES)
  _, root = life(sand)
  sand.script[root] = [NUMS]
  assert await engine.prompt(list, "a door of my own", on=root) == [1, 2]
  assert of(engine.turns(on=root), "read") == ["#read nums://a\n# [1, 2]"]


async def test_a_read_answered_with_plain_data_gives_the_text_of_its_path_and_its_content() -> None:
  """A read answered with plain data gives the Text of its path and its content."""
  sand = sown(FILES)
  _, root = life(sand)
  word = "t = read('a.txt')\nclose([type(t) is Text, t.path, t.content, t.before])"
  assert await engine.rung(word, on=root) == [True, "/w/a.txt", "one\ntwo\n", None]
  (asked,) = [a for a in engine.asked.values() if a[0] == "read"]
  assert engine.outcomes[asked[1]] == {"path": "/w/a.txt", "content": "one\ntwo\n"}


async def test_read_tells_a_line_again_after_the_content_of_the_line_changed() -> None:
  """read tells a line again after the content of the line changed."""
  sand = sown(FILES)
  _, root = life(sand)
  sand.script[root] = ["read('a.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  sand.files["/w/a.txt"] = "one\nTWO\n"
  sand.script[root] = ["read('a.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it again", on=root) == 2
  assert of(engine.turns(on=root), "read") == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two",
    "#read a.txt\n# /w/a.txt, 1 known\n# 2 TWO",
  ]


async def test_a_second_read_of_a_text_tells_the_model_no_line_that_an_earlier_read_told() -> None:
  """A second read of a text tells the model no line that an earlier read of the chain told."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\nthree\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 2))\nread('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it twice", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    "#read n.txt\n# /w/n.txt, 0 known\n# 1 one\n# 2 two",
    "#read n.txt\n# /w/n.txt, 2 known\n# 3 three",
  ]
