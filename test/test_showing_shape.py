"""Showing, a text a note shows and the show of it."""

from conftest import STANDS, Sand, life, of, paragraphs, said, settle
from furb import engine
from furb.engine import TAIL, WORLD, Text, span

THREE = "one\ntwo\nthree\n"
"""A text of three lines."""


def sown() -> Sand:
  """A World with a text of three lines and the roster of the suite."""
  return Sand(files={"/w/n.txt": THREE}, stands=STANDS)


async def test_a_text_a_note_shows_and_the_show_of_it() -> None:
  """A text a note shows, and the show of it, which is what a tell of a text carries and what the fold of the turns makes comments of."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(2, 3))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  carried = [a[3] for a in said(log, "tell") if a[3][0] == "#read n.txt"]
  assert [len(notes) for notes in carried] == [2]
  _, (text, show) = carried[0]
  assert text == Text("/w/n.txt", THREE) and show(text.lines) == [2, 3]
  assert of(engine.turns(on=root), "read") == ["#read n.txt\n# /w/n.txt, 0 known\n# 2 two\n# 3 three"]


async def test_the_paragraph_of_an_exited_command_holds_one_showing_for_each_text_told() -> None:
  """The paragraph of an exited command holds one showing for each text told."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  both = engine.bash("echo hi", show_err=TAIL, on=root)
  assert (await both).code == 0
  merged = engine.bash("echo two", on=root)
  assert (await merged).code == 0
  await settle()
  exited = {a[1]: a[3][1:] for a in said(log, "tell") if a[3][0].endswith(" exited 0")}
  assert {act: [text.path for text, _ in notes] for act, notes in exited.items()} == {
    both: [f"{both}/stdout", f"{both}/stderr"],
    merged: [f"{merged}/stdout"],
  }
  assert [one for one in paragraphs(engine.turns(on=root)) if " exited " in one.split("\n", 1)[0]] == [
    f"#{both} exited 0\n# {both}/stdout, 0 known\n# 1 ran echo hi\n# {both}/stderr, 0 known",
    f"#{merged} exited 0\n# {merged}/stdout, 0 known\n# 1 ran echo two",
  ]


async def test_a_showing_stands_as_a_comment_of_the_path_of_its_text_and_of_how_many_lines_the_chain_knows() -> None:
  """A showing stands as a comment of the path of its text and of how many of the lines the show picked the chain knows, then a comment for each other line it picked, with its number."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(2, 2))\nread('n.txt', grep('^t'))\nclose(1)"]
  assert await engine.prompt(int, "grep it", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    "#read n.txt\n# /w/n.txt, 0 known\n# 2 two",
    "#read n.txt\n# /w/n.txt, 1 known\n# 3 three",
  ]


async def test_the_engine_applies_a_show_before_it_writes_a_line() -> None:
  """The engine applies a show before it writes a line, so the paragraph holds the picked lines alone."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(2, 2))\nclose(1)"]
  assert await engine.prompt(int, "read one line", on=root) == 1
  assert engine.turns(on=root)[-1][1] == "#read n.txt\n# /w/n.txt, 0 known\n# 2 two\n\n#prompt1 closed 1"


async def test_a_show_applies_to_a_text_or_to_a_stream() -> None:
  """A show applies to a text or to a stream."""
  sand = sown()
  sand.auto = False
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 1))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  act = engine.bash("many", show=span(1, 1), on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.send("out", command, "a\nb\nc\n", "stdout", by=WORLD)
  engine.send("exited", command, 0, by=WORLD)
  assert (await act).code == 0
  assert of(engine.turns(on=root), "read") == ["#read n.txt\n# /w/n.txt, 0 known\n# 1 one"]
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{command} exited")] == [
    f"#{command} exited 0\n# {command}/stdout, 0 known\n# 1 a"
  ]


async def test_a_line_told_once_on_a_chain_is_known_there() -> None:
  """A line told once on a chain is known there, by its path, its number and its content."""
  sand = sown()
  sand.files["/w/m.txt"] = THREE
  _, root = life(sand)
  sand.script[root] = ["read('n.txt')\nclose(1)", "read('n.txt')\nread('m.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  sand.files["/w/n.txt"] = "two\none\nthree\n"
  assert await engine.prompt(int, "read it again", on=root) == 2
  assert of(engine.turns(on=root), "read") == [
    "#read n.txt\n# /w/n.txt, 0 known\n# 1 one\n# 2 two\n# 3 three",
    "#read n.txt\n# /w/n.txt, 1 known\n# 1 two\n# 2 one",
    "#read m.txt\n# /w/m.txt, 0 known\n# 1 one\n# 2 two\n# 3 three",
  ]


async def test_read_tells_a_line_again_after_the_content_of_the_line_changed() -> None:
  """read tells a line again after the content of the line changed."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  sand.files["/w/n.txt"] = "one\nTWO\nthree\n"
  sand.script[root] = ["read('n.txt')\nclose(2)"]
  assert await engine.prompt(int, "read it again", on=root) == 2
  assert of(engine.turns(on=root), "read") == [
    "#read n.txt\n# /w/n.txt, 0 known\n# 1 one\n# 2 two\n# 3 three",
    "#read n.txt\n# /w/n.txt, 2 known\n# 2 TWO",
  ]


async def test_a_text_costs_its_size_once_on_a_chain() -> None:
  """A text costs its size once on a chain."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 2))\nread('n.txt')\nread('n.txt', grep('^t'))\nclose(1)"]
  assert await engine.prompt(int, "read it thrice", on=root) == 1
  told = of(engine.turns(on=root), "read")
  assert told == [
    "#read n.txt\n# /w/n.txt, 0 known\n# 1 one\n# 2 two",
    "#read n.txt\n# /w/n.txt, 2 known\n# 3 three",
    "#read n.txt\n# /w/n.txt, 2 known",
  ]
  assert sum(len(one.split("\n")) - 2 for one in told) == len(Text("/w/n.txt", THREE).lines)


async def test_a_second_read_of_a_text_tells_the_model_no_line_that_an_earlier_read_told() -> None:
  """A second read of a text tells the model no line that an earlier read of the chain told."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 2))\nread('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it twice", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    "#read n.txt\n# /w/n.txt, 0 known\n# 1 one\n# 2 two",
    "#read n.txt\n# /w/n.txt, 2 known\n# 3 three",
  ]


async def test_a_tell_shows_each_showing_it_holds_and_each_other_note_stands_as_it_is() -> None:
  """A tell shows each showing it holds, and each other note stands as it is."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["tell('found', 'x', '# as it is', (Text('p', 'a\\nb\\n'), TAIL))\nclose(1)"]
  assert await engine.prompt(int, "tell a note", on=root) == 1
  await settle()
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#found ")] == [
    "#found x\n# as it is\n# p, 0 known\n# 1 a\n# 2 b"
  ]
  assert engine.shown("# as it is", {}) == "# as it is"
