"""Showing, a path, the content at it and the show of it."""

from conftest import STANDS, Sand, life, of, paragraphs, said
from furb import engine

THREE = "one\ntwo\nthree\n"
"""A content of three lines."""

SHOWS = (
  f"THREE = {THREE!r}\n"
  "def span(lo, hi):\n"
  "  return lambda lines: [i for i in range(lo, min(hi, len(lines)) + 1)]\n"
  "\n"
  "ALL = span(1, 9)\n"
)
"""A word of a rung that binds the content of three lines and a show of the lines lo through hi, as a word writes
one."""


def seen(root: str) -> list[str]:
  """Every paragraph of the turns of the chain that a tell headed seen stands as, in order."""
  return of(engine.turns(on=root), "seen")


async def shows(*words: str) -> str:
  """A life whose root binds the shows of the suite and then runs each word, in order, as the operator writes it."""
  _, root = life(Sand(stands=STANDS))
  await engine.rung(SHOWS, on=root)
  for word in words:
    await engine.rung(word, on=root)
  return root


async def test_a_path_the_content_at_it_and_the_show_of_it() -> None:
  """A path, the content at it, and the show of it, which is what a tell carries of what it shows and what the fold of the turns makes comments of."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung(SHOWS, on=root)
  await engine.rung("tell('seen', 'n', ('/w/n', THREE, span(2, 3)))", on=root)
  (carried,) = [a[3] for a in said(log, "tell") if a[3][0] == "#seen n"]
  _, (path, content, show) = carried
  assert (path, content, show(content.splitlines())) == ("/w/n", THREE, [2, 3])
  assert seen(root) == ["#seen n\n# /w/n, 0 known\n# 2 two\n# 3 three"]


async def test_a_tell_shows_each_showing_it_holds_and_each_other_note_stands_as_it_is() -> None:
  """A tell shows each showing it holds, and each other note stands as it is."""
  root = await shows("tell('seen', 'x', '# as it is', ('p', 'a\\nb\\n', ALL))")
  assert seen(root) == ["#seen x\n# as it is\n# p, 0 known\n# 1 a\n# 2 b"]


async def test_a_showing_stands_as_a_comment_of_its_path_and_of_how_many_lines_the_chain_knows() -> None:
  """A showing stands as a comment of its path and of how many of the lines the show picked the chain knows, then a comment for each other line it picked, with its number."""
  root = await shows(
    "tell('seen', 'n', ('/w/n', THREE, span(2, 2)))",
    "tell('seen', 'n', ('/w/n', THREE, lambda lines: [i for i, x in enumerate(lines, 1) if x.startswith('t')]))",
  )
  assert seen(root) == [
    "#seen n\n# /w/n, 0 known\n# 2 two",
    "#seen n\n# /w/n, 1 known\n# 3 three",
  ]


async def test_the_engine_applies_a_show_before_it_writes_a_line() -> None:
  """The engine applies a show before it writes a line, so the paragraph holds the picked lines alone."""
  root = await shows("given = []\ndef second(lines):\n  given.append(lines)\n  return [2]\n\ntell('seen', 'n', ('/w/n', THREE, second))")
  assert seen(root) == ["#seen n\n# /w/n, 0 known\n# 2 two"]
  assert engine.modules[root]["given"][-1] == ["one", "two", "three"]
  assert [one for one in paragraphs(engine.turns(on=root)) if "# 1 one" in one or "# 3 three" in one] == []


async def test_a_line_told_once_on_a_chain_is_known_there() -> None:
  """A line told once on a chain is known there, by its path, its number and its content."""
  root = await shows(
    "tell('seen', 'n', ('/w/n', THREE, ALL))",
    "tell('seen', 'n', ('/w/n', 'two\\none\\nthree\\n', ALL))",
    "tell('seen', 'm', ('/w/m', THREE, ALL))",
  )
  assert seen(root) == [
    "#seen n\n# /w/n, 0 known\n# 1 one\n# 2 two\n# 3 three",
    "#seen n\n# /w/n, 1 known\n# 1 two\n# 2 one",
    "#seen m\n# /w/m, 0 known\n# 1 one\n# 2 two\n# 3 three",
  ]


async def test_a_line_is_told_again_after_its_content_changed() -> None:
  """A line is told again after its content changed."""
  root = await shows(
    "tell('seen', 'n', ('/w/n', THREE, ALL))",
    "tell('seen', 'n', ('/w/n', 'one\\nTWO\\nthree\\n', ALL))",
  )
  assert seen(root) == [
    "#seen n\n# /w/n, 0 known\n# 1 one\n# 2 two\n# 3 three",
    "#seen n\n# /w/n, 2 known\n# 2 TWO",
  ]


async def test_a_content_costs_its_size_once_on_a_chain() -> None:
  """A content costs its size once on a chain."""
  root = await shows(
    "tell('seen', 'n', ('/w/n', THREE, span(1, 2)))",
    "tell('seen', 'n', ('/w/n', THREE, ALL))",
    "tell('seen', 'n', ('/w/n', THREE, span(2, 3)))",
  )
  told = seen(root)
  assert told == [
    "#seen n\n# /w/n, 0 known\n# 1 one\n# 2 two",
    "#seen n\n# /w/n, 2 known\n# 3 three",
    "#seen n\n# /w/n, 2 known",
  ]
  assert sum(len(one.split("\n")) - 2 for one in told) == len(THREE.splitlines())
