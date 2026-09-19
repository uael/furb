"""read, the text at a path."""

import pytest

from conftest import STANDS, Dead, Sand, attr, life, said, settle, shown, sown, tags
from furb import engine
from furb.engine import HEAD, HIDDEN, Refused, Text, span, take

BIG = "".join(f"line {i}\n" for i in range(1, 2101))
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


def body_of(tag: tuple) -> str:
  """The body of the first shown tag that a tag holds, which is a string."""
  one = shown(tag)[0][2]
  assert isinstance(one, str)
  return one


async def test_a_read_whoever_serves_the_path_answers_it_with_the_text_of_it() -> None:
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines the model has not seen."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nsame = read('a.txt')\nclose([len(t.lines), same.content])"]
  assert await engine.prompt(list, "read them", on=root) == [2, "one\ntwo\n"]
  told = tags(engine.turns(on=root), "read")
  assert [attr(tag, "path") for tag in told] == ["a.txt", "a.txt"]
  assert [body_of(tag) for tag in told] == ["1 one\n2 two", ""]


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
  _, step, *_ = said(log, "rung")[0]
  fork = engine.chain("fork", source=root, filter=take(step, inside=False))
  await settle(300)
  sand.script[fork] = ["read('a.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it again", on=fork) == 2
  sand.script[root] = ["read('a.txt')\nclose(3)"]
  assert await engine.prompt(int, "read it again", on=root) == 3
  here, there = tags(engine.turns(on=fork), "read"), tags(engine.turns(on=root), "read")
  assert [body_of(tag) for tag in here] == ["1 one\n2 two"]
  assert [body_of(tag) for tag in there] == ["1 one\n2 two", ""]


async def test_the_engine_judges_no_scheme() -> None:
  """The engine judges no scheme, so a path of an unknown scheme goes to the World too."""
  sand = Sand(files={"weird://x": "kept\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.read("weird://x", on=root) == Text("weird://x", "kept\n")
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x"]
  assert engine.read("weird://y", on=root) is None


async def test_read_is_given_a_path_and_a_show() -> None:
  """read is given a path and a show."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('a.txt', span(2, 2))\nread('a.txt', grep('^o'))\nclose(1)"]
  assert await engine.prompt(int, "read them", on=root) == 1
  told = tags(engine.turns(on=root), "read")
  assert [attr(tag, "path") for tag in told] == ["a.txt", "a.txt"]
  assert [body_of(tag) for tag in told] == ["2 two", "1 one"]


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
  body = body_of(tags(engine.turns(on=root), "read")[0])
  assert body.splitlines()[0] == "1 line 1"
  assert body.splitlines()[-1] == "2000 line 2000"
  assert len(body.splitlines()) == 2000


async def test_a_read_of_chain_lineage_gives_the_program_of_the_chain() -> None:
  """A read of chain://lineage gives the program of the chain, its accepted words in order."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["BAD = 1", "a = 1", "close(a + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  assert engine.read(root, on=root) == Text(root, "a = 1\nclose(a + 1)")


async def test_whether_a_name_is_the_chains_own_or_one_of_the_acts_it_has_heard_on_itself() -> None:
  """Whether a name is the chain's own or one of the acts it has heard on itself, which is what its doors serve and no other path, a path of no name being none of them."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  assert engine.read(root, on=root).content == "a = 1\nclose(a + 1)"
  assert engine.read(act, on=root).content == "a = 1\nclose(a + 1)"
  assert engine.read("", on=root) is None and engine.read("nowhere://x", on=root) is None


async def test_the_chain_is_the_door_of_its_program() -> None:
  """The chain is the door of its program, so a read of its name gives its accepted words in order, and a read of the name of any act it has heard gives the words of that ladder alone, which are none at all for an act that ran no word."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  _, command, *_ = said(log, "bash")[0]
  assert engine.read(root, on=root).content == "x = bash('echo hi')\nclose(1)\nclose(None)"
  assert engine.read(act, on=root).content == "x = bash('echo hi')\nclose(1)"
  assert engine.read(command, on=root).content == ""


async def test_one_asked_from_inside_an_act_tells_itself() -> None:
  """One asked from inside an act tells itself, with its path and what it was answered, on the scope of that act; one asked from outside an act tells nothing, and neither does one whose show is hidden."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('a.txt')\nread('a.txt', HIDDEN)\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  _, step, *_ = said(log, "rung")[0]
  told = [a for a in said(log, "tell") for tag in a[3] if tag[0] == "read"]
  assert [(a[1], a[2]) for a in told] == [(step, step)]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert engine.scope(step) == root and told[0] in held
  was = tags(engine.turns(on=root))
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("a.txt", HIDDEN, on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert tags(engine.turns(on=root)) == was


async def test_a_read_answered_with_what_is_no_text_gives_that_value() -> None:
  """A read answered with what is no text gives that value, and tells it as python shows it."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [NUMS]
  assert await engine.prompt(list, "a door of my own", on=root) == [1, 2]
  assert [tag[2] for tag in tags(engine.turns(on=root), "read")] == ["[1, 2]"]
