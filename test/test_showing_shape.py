"""Showing, a text a tag shows and the show of it."""

from conftest import STANDS, Sand, attr, life, said, settle, shown, tags
from furb import engine
from furb.engine import TAIL, WORLD, Text, grep

THREE = "one\ntwo\nthree\n"


def body_of(tag: tuple) -> str:
  """The body of the first shown tag that a tag holds, which is a string."""
  one = shown(tag)[0][2]
  assert isinstance(one, str)
  return one


def sown() -> Sand:
  """A World with a text of three lines and the roster of the suite."""
  return Sand(files={"/w/n.txt": THREE}, stands=STANDS)


async def test_a_text_a_tag_shows_and_the_show_of_it() -> None:
  """A text a tag shows, and the show of it, which is what a tell of a text carries and what the fold of the turns makes a tag of."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(2, 3))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  carried = [tag for a in said(log, "tell") for tag in a[3] if tag[0] == "read"]
  assert [len(tag[2]) for tag in carried] == [1]
  ((text, show),) = carried[0][2]
  assert text == Text("/w/n.txt", THREE) and show(text.lines) == [2, 3]
  made = tags(engine.turns(on=root), "read")[0]
  assert shown(made) == [("shown", [("path", "/w/n.txt"), ("known", 0)], "2 two\n3 three")]


async def test_the_body_of_a_closed_tag_holds_one_shown_for_each_text_told() -> None:
  """The body of a closed tag holds one shown for each text told."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", show_err=TAIL, on=root)
  assert (await act).code == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  closed = next(tag for tag in tags(engine.turns(on=root), "closed") if ("id", command) in tag[1])
  assert [attr(one, "path") for one in shown(closed)] == [f"{command}/stdout", f"{command}/stderr"]


async def test_the_shown_tag_holds_as_its_body_the_lines_that_a_show_picked_with_their_numbers() -> None:
  """The shown tag holds as its body the lines that a show picked, with their numbers."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', grep('^t'))\nclose(1)"]
  assert await engine.prompt(int, "grep it", on=root) == 1
  told = tags(engine.turns(on=root), "read")[0]
  assert [one[2] for one in shown(told)] == ["2 two\n3 three"]
  assert grep("^t")(Text("/w/n.txt", THREE).lines) == [2, 3]


async def test_the_engine_applies_a_show_before_it_makes_a_tag() -> None:
  """The engine applies a show before it makes a tag, so the body holds the picked lines alone."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(2, 2))\nclose(1)"]
  assert await engine.prompt(int, "read one line", on=root) == 1
  told = tags(engine.turns(on=root), "read")[0]
  assert shown(told) == [("shown", [("path", "/w/n.txt"), ("known", 0)], "2 two")]
  assert "one" not in body_of(told)


async def test_a_show_applies_to_a_text_or_to_a_stream() -> None:
  """A show applies to a text or to a stream."""
  sand = sown()
  sand.auto = False
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 1))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  act = engine.bash("many", show=engine.span(1, 1), on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  engine.send("out", command, "a\nb\nc\n", "stdout", by=WORLD)
  engine.send("exited", command, 0, by=WORLD)
  assert (await act).code == 0
  told = tags(engine.turns(on=root), "read")[0]
  closed = next(tag for tag in tags(engine.turns(on=root), "closed") if ("id", command) in tag[1])
  assert body_of(told) == "1 one"
  assert body_of(closed) == "1 a"


async def test_a_line_told_once_on_a_chain_is_known_there() -> None:
  """A line told once on a chain is known there, by its path, its number and its content."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt')\nclose(1)", "read('n.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  assert await engine.prompt(int, "read it again", on=root) == 2
  told = tags(engine.turns(on=root), "read")
  assert [body_of(tag) for tag in told] == ["1 one\n2 two\n3 three", ""]
  assert [attr(shown(tag)[0], "known") for tag in told] == [0, 3]


async def test_read_tells_a_line_again_after_the_content_of_the_line_changed() -> None:
  """read tells a line again after the content of the line changed."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  sand.files["/w/n.txt"] = "one\nTWO\nthree\n"
  sand.script[root] = ["read('n.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it again", on=root) == 2
  told = tags(engine.turns(on=root), "read")
  assert [body_of(tag) for tag in told] == ["1 one\n2 two\n3 three", "2 TWO"]
  assert [attr(shown(tag)[0], "known") for tag in told] == [0, 2]


async def test_a_text_costs_its_size_once_on_a_chain() -> None:
  """A text costs its size once on a chain."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 2))\nread('n.txt')\nread('n.txt', grep('^t'))\nclose(1)"]
  assert await engine.prompt(int, "read it thrice", on=root) == 1
  told = tags(engine.turns(on=root), "read")
  lines = [line for tag in told for line in body_of(tag).splitlines()]
  assert lines == ["1 one", "2 two", "3 three"]
  assert len(lines) == len(Text("/w/n.txt", THREE).lines)


async def test_a_second_read_of_a_text_tells_the_model_no_line_that_an_earlier_read_told() -> None:
  """A second read of a text tells the model no line that an earlier read of the chain told."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 2))\nread('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it twice", on=root) == 1
  told = tags(engine.turns(on=root), "read")
  assert [body_of(tag) for tag in told] == ["1 one\n2 two", "3 three"]
  assert [attr(shown(tag)[0], "known") for tag in told] == [0, 2]


async def test_a_list_body_shows_each_showing_in_it_and_anything_else_in_it_stands_as_it_is() -> None:
  """A list body shows each showing in it, and anything else in it stands as it is."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["tell('found', body=[1, 'x', (Text('p', 'a\\nb\\n'), TAIL)])\nclose(1)"]
  assert await engine.prompt(int, "tell a list", on=root) == 1
  await settle()
  found = [tag[2] for tag in tags(engine.turns(on=root), "found")]
  assert found == [[1, "x", ("shown", [("path", "p"), ("known", 0)], "1 a\n2 b")]]
