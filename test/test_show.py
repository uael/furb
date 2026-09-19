"""Show, what says which lines of a text the engine tells."""

from conftest import MANY, life, said, settle, shown, sown, tags
from furb import engine
from furb.engine import HEAD, Text, differs, grep, span

ODD = "odd = lambda lines: [i for i in range(1, len(lines) + 1) if i % 2]\nread('a.txt', odd)\nclose(1)\n"


def test_a_show_is_given_the_lines_of_a_text_and_gives_the_numbers_of_the_lines_to_tell() -> None:
  """A show is given the lines of a text and gives the numbers of the lines to tell."""
  assert span(2, 3)(["a", "b", "c", "d"]) == [2, 3]
  assert grep("^t")(["one", "two", "three"]) == [2, 3]
  assert differs(["one", "two"])(["one", "TWO"]) == [2]
  assert HEAD(Text("/w/n.txt", MANY).lines) == list(range(1, 301))


async def test_a_show_is_any_callable_of_that_shape() -> None:
  """A show is any callable of that shape, so a word adds a show by writing one, and span, grep and differs make the shows of the file."""
  assert callable(span(1, 2)) and callable(grep("^t")) and callable(differs(["one"]))
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [ODD]
  assert await engine.prompt(int, "show the odd lines", on=root) == 1
  told = tags(engine.turns(on=root), "read")[0]
  assert [one[2] for one in shown(told)] == ["1 one"]


async def test_a_show_is_no_word_of_a_fact() -> None:
  """A show is no word of a fact: the verb that was given it keeps it for what it tells, and the life of the act closes over it, so no record holds one."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('a.txt', span(1, 1))\nx = bash('echo hi', show=span(1, 1))\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  assert said(log, "bash")[0][4:] == ("echo hi", False, 600.0)
  assert said(log, "read")[0][4:] == ("a.txt",)
  assert [word for entry in sand.record for word in (*entry[1], *entry[2:]) if callable(word)] == []
  told = tags(engine.turns(on=root), "read")[0]
  assert [one[2] for one in shown(told)] == ["1 one"]
