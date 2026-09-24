"""Text, a path, what stands at it, and the text it came from."""

from conftest import worded

ONE = "one = Text('/w/n.txt', 'one\\ntwo\\nthree\\n')\n"
"""The start of a word that binds a text of three lines."""


async def test_a_text_its_path_what_stands_at_it_of_which_its_lines_are_the_lines() -> None:
  """A text: its path, what stands at it, of which its lines are the lines, and the text it came from."""
  word = ONE + "after = one.delete(2, 2)\nclose([one.path, one.content, one.lines, one.before, after.lines, after.before is one])"
  assert await worded(word) == ["/w/n.txt", "one\ntwo\nthree\n", ["one", "two", "three"], None, ["one", "three"], True]


async def test_the_lines_of_a_text_derive_from_its_content() -> None:
  """The lines of a text derive from its content."""
  word = ONE + "close([Text('/w/n.txt').lines, Text('/w/n.txt', 'one').lines, one.lines == one.content.splitlines()])"
  assert await worded(word) == [[], ["one"], True]


async def test_every_edit_of_it_gives_another_text_which_came_from_this_one() -> None:
  """Every edit of it gives another text, which came from this one."""
  word = ONE + (
    "made = one.append('four')\nagain = made.delete(1, 1)\n"
    "close([made is not one, made.before is one, one.content, again.before is made, again.content])"
  )
  assert await worded(word) == [True, True, "one\ntwo\nthree\n", True, "two\nthree\nfour\n"]
