"""Show, what says which lines of a showing the engine tells."""

from conftest import STANDS, Sand, life, of, paragraphs, said
from furb import engine

LOOK = (
  "def look(path, content, show, on=''):\n"
  "  def ear(id):\n"
  "    told(id, path, (path, content, show))\n"
  "    yield 'done', id, None\n"
  "\n"
  "  return act('look', on, ear, path)\n"
)
"""A word of a rung that writes a verb of its own, which is given a show and tells a showing with it."""


async def test_a_show_is_given_the_lines_of_the_content_of_a_showing() -> None:
  """A show is given the lines of the content of a showing and gives the numbers of the lines to tell."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  word = "given = []\ndef two(lines):\n  given.append(lines)\n  return [2]\n\ntell('seen', 'n', ('/w/n', 'a\\nb\\nc\\n', two))"
  await engine.rung(word, on=root)
  assert of(engine.turns(on=root), "seen") == ["#seen n\n# /w/n, 0 known\n# 2 b"]
  assert engine.modules[root]["given"][-1] == ["a", "b", "c"]


async def test_a_show_is_any_callable_of_that_shape() -> None:
  """A show is any callable of that shape, so a word adds a show by writing one."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("odd = lambda lines: [i for i in range(1, len(lines) + 1) if i % 2]", on=root)
  await engine.rung("tell('seen', 'n', ('/w/n', 'a\\nb\\nc\\n', odd))", on=root)
  await engine.rung("def last(lines):\n  return [len(lines)]\n\ntell('seen', 'm', ('/w/m', 'x\\ny\\n', last))", on=root)
  assert of(engine.turns(on=root), "seen") == [
    "#seen n\n# /w/n, 0 known\n# 1 a\n# 3 c",
    "#seen m\n# /w/m, 0 known\n# 2 y",
  ]


async def test_a_show_is_no_word_of_a_fact() -> None:
  """A show is no word of a fact: the verb that was given it keeps it for what it tells, and the ear of the act closes over it, so no record holds one."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung(LOOK, on=root)
  await engine.rung("x = look('/w/n', 'a\\nb\\n', lambda lines: [1])\nawait x", on=root)
  (made,) = said(log, "look")
  assert made[4:] == ("/w/n",)
  assert [word for entry in sand.record for word in (*entry[0], *entry[1:]) if callable(word)] == []
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{made[1]} ")] == [
    f"#{made[1]} /w/n\n# /w/n, 0 known\n# 1 a"
  ]
