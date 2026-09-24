"""edit, the text with one more edit of its lines."""

from conftest import worded

ONE = "one = Text('/w/n.txt', 'one\\ntwo\\nthree\\n')\n"
"""The start of a word that binds a text of three lines."""


async def test_the_text_with_one_more_edit_an_edit_whose_lines_are_not_there_is_refused() -> None:
  """The text with one more edit; an edit whose lines are not there is refused."""
  word = ONE + (
    "made = one.edit(2, 2, ['TWO'])\nno = []\n"
    "for lo, hi in ((9, 9), (1, 4)):\n"
    "  try:\n"
    "    one.edit(lo, hi, ['x'])\n"
    "  except Refused as why:\n"
    "    no.append(str(why))\n"
    "close([made.content, made.before is one, no])"
  )
  assert await worded(word) == ["one\nTWO\nthree\n", True, ["/w/n.txt no lines 9:9", "/w/n.txt no lines 1:4"]]


async def test_the_edits_that_say_themselves_are_one_edit_each() -> None:
  """The edits that say themselves are one edit each: a replace of a string, an append at the end, an insert before a line, and a delete of lines."""
  word = ONE + (
    "made = [one.replace('one', '1'), one.append('four'), one.insert(1, 'top'), one.delete(2, 2)]\n"
    "close([[x.content for x in made], [x.before is one and x.undo() is one for x in made]])"
  )
  assert await worded(word) == [
    ["1\ntwo\nthree\n", "one\ntwo\nthree\nfour\n", "top\none\ntwo\nthree\n", "one\nthree\n"],
    [True, True, True, True],
  ]
