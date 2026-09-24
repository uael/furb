"""grow, the text as more of it is told."""

from conftest import worded


async def test_the_text_as_more_of_it_is_told() -> None:
  """The text as more of it is told, which is how a stream of a command grows, and which comes from no text, since a stream that grows is no edit of one."""
  word = (
    "grown = Text('/x/stdout').grow('half\\n').grow('rest\\n')\n"
    "close([grown.content, grown.lines, grown.before, grown.undo() is grown])"
  )
  assert await worded(word) == ["half\nrest\n", ["half", "rest"], None, True]
