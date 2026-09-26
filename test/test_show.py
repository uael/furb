"""Show, what says which lines of a text the engine tells."""

from conftest import MANY, acts, born, paragraphs, said, settle
from furb import engine
from furb.engine import HEAD, Text, differs, grep, span

ODD = "odd = lambda lines: [i for i in range(1, len(lines) + 1) if i % 2]\nread('a.txt', odd)\nclose(1)\n"
"""A word of a rung that writes a show of its own, which picks the odd lines, and reads with it."""


def test_a_show_is_given_the_lines_of_a_text_and_gives_the_numbers_of_the_lines_to_tell() -> None:
  """A show is given the lines of a text and gives the numbers of the lines to tell."""
  assert span(2, 3)(["a", "b", "c", "d"]) == [2, 3]
  assert grep("^t")(["one", "two", "three"]) == [2, 3]
  assert differs(["one", "two"])(["one", "TWO"]) == [2]
  assert HEAD(Text("/w/n.txt", MANY).lines) == list(range(1, 301))


async def test_a_show_is_any_callable_of_that_shape() -> None:
  """A show is any callable of that shape, so a word adds a show by writing one, and span, grep and differs make the shows of the file."""
  assert callable(span(1, 2)) and callable(grep("^t")) and callable(differs(["one"]))
  _, _, root = born(ODD)
  assert await engine.prompt(int, "show the odd lines", on=root) == 1
  assert engine.turns(on=root)[-1][1] == "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n\n#prompt1 closed 1"


async def test_a_show_is_no_word_of_a_fact() -> None:
  """A show is no word of a fact: the verb that was given it keeps it for what it tells, and the ear of the act closes over it, so no record holds one."""
  sand, log, root = born("read('a.txt', span(1, 1))\nx = bash('echo hi', show=span(1, 1))\nclose(1)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  assert said(log, "bash")[0][4:] == ("echo hi", False, 600.0)
  assert [a[4:] for a in acts(log).values() if a[0] == "read"] == [("a.txt",)]
  assert [word for (fact,) in sand.record for word in fact if callable(word)] == []
  told = [one for one in paragraphs(engine.turns(on=root)) if one.startswith(("#read ", f"#{command} exited"))]
  assert told == [
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one",
    f"#{command} exited 0\n# {command}/stdout, 0 known\n# 1 ran echo hi",
  ]
