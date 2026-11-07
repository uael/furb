"""Show, what says which lines of a text the engine tells."""

from conftest import MANY, Sand, acts, born, paragraphs, plain, relived, said, settle
from furb import engine
from furb.engine import HEAD, Text, differs, grep, picked, span

ODD = "odd = lambda lines: [i for i in range(1, len(lines) + 1) if i % 2]\nread('a.txt', odd)\nclose(1)\n"
"""A word of a rung that writes a show of its own, which picks the odd lines, and reads with it."""


async def test_a_show_says_which_lines_of_a_text_to_tell_and_picked_gives_their_numbers() -> None:
  """A show says which lines of a text to tell, and picked gives their numbers from the lines of the text."""
  born()
  assert picked(span(2, 3), ["a", "b", "c", "d"]) == [2, 3]
  assert picked(grep("^t"), ["one", "two", "three"]) == [2, 3]
  assert picked(differs(["one", "two"]), ["one", "TWO"]) == [2]
  assert picked(HEAD, Text("/w/n.txt", MANY).lines) == list(range(1, 301))


async def test_a_show_of_the_file_is_plain_data_so_it_is_a_word_of_each_act_that_it_is_given_to() -> None:
  """A show of the file is plain data, which span, grep and differs make, so it is a word of each act that it is given to and a record holds it."""
  assert (span(1, 2), grep("^t"), differs(["one"])) == (("span", 1, 2), ("grep", "^t"), ("differs", ["one"]))
  sand, log, root = born("x = bash('echo hi', show=span(1, 1), show_err=grep('a'))\nclose(1)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  (command,) = said(log, "bash")
  assert command[4:] == ("echo hi", "", 120.0, None, ("span", 1, 1), ("grep", "a"))
  assert [fact[8:] for (fact,) in plain(sand.record) if fact[:2] == command[:2]] == [(("span", 1, 1), ("grep", "a"))]


async def test_a_show_is_any_callable_that_is_given_the_lines_of_a_text() -> None:
  """A show is any callable that is given the lines of a text and gives the numbers of the lines to tell, so a word adds a show by writing one, which the word makes again when it runs again; no later life makes it again as it was, so it is no word of an act, which holds None in its place, and a later life tells by the default show."""
  _, log, root = born(ODD)
  assert await engine.prompt(int, "show the odd lines", on=root) == 1
  assert engine.turns(on=root)[-1][1] == "#read1\nread1_path = 'a.txt'\nread1_text = 'one'"
  assert [a[4:] for a in acts(log).values() if a[0] == "read"] == [("a.txt",)]
  told = [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#read1")]
  assert told == ["#read1\nread1_path = 'a.txt'\nread1_text = 'one'"]
  sand, log, root = born()
  ran = engine.bash("echo hi", show=lambda _: [1], show_err=grep("x"), on=root)
  done = await ran
  assert done.code == 0
  await settle()
  (word,) = said(log, "bash")
  assert word[-2:] == (None, grep("x"))
  again, _ = await relived(Sand(), plain(sand.record))
  assert said(again, "bash") == [word] and engine.peek(word[1]) == done
  assert [a[3] for a in again if a[0] == "tell" and a[1] == ran][-1:] == [
    [a[3] for a in log if a[0] == "tell" and a[1] == ran][-1]
  ]
  [chain] = engine.gathered(lambda one: one, {})["chains"]
  assert word in chain["turns"]
