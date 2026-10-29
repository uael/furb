"""mcp_closed, the words that the done of an mcp act tells."""

from pathlib import Path

from extensions.conftest import mcped, server
from furb import engine

IMAGE = {"type": "image", "image": "furb-image://ab12", "mime": "image/png"}
"""IMAGE is the block of an image in the content of a result."""


async def test_mcp_closed_gives_the_words_that_the_done_of_an_mcp_act_tells(tmp_path: Path) -> None:
  """mcp_closed(got, show) gives the words that the done of an mcp act tells, as a dict, and an empty dict for an answer that is none of the following."""
  root = mcped(tmp_path, {})[1]
  got = await engine.rung("close([mcp_closed(None, HEAD), mcp_closed('x', HEAD), mcp_closed(3, HEAD)])", on=root)
  assert got == [{}, {}, {}]


async def test_a_list_of_servers_and_a_server_tell_their_lines(tmp_path: Path) -> None:
  """A list of servers tells under the word servers one line for each server, and a server tells its line under the word server, as mcp_line gives them."""
  root = mcped(tmp_path, {})[1]
  two = [server("a"), server("b", state="failed", error="x")]
  got = await engine.rung(f"close([mcp_closed({two!r}, HEAD), mcp_closed({two[1]!r}, HEAD)])", on=root)
  assert got == [{"servers": "a connected, no tools\nb failed: x"}, {"server": "b failed: x"}]


async def test_a_result_tells_the_lines_of_its_text_that_its_show_picks_and_a_line_for_each_image(
  tmp_path: Path,
) -> None:
  """A result tells under the word text the lines of its text that its show picks, and one line of Markdown for each image block of its content, which shows the image."""
  root = mcped(tmp_path, {})[1]
  both = {"text": "a\nb\nc", "content": [{"type": "text", "text": "a\nb\nc"}, IMAGE], "structured": None}
  only = {"text": "", "content": [IMAGE]}
  plain = {"text": "a"}
  empty = {"text": ""}
  word = f"close([mcp_closed(x, span(1, 2)) for x in {[both, only, plain, empty]!r}])"
  got = await engine.rung(word, on=root)
  assert got == [
    {"text": "a\nb\n![image](furb-image://ab12)"},
    {"text": "![image](furb-image://ab12)"},
    {"text": "a"},
    {"text": ""},
  ]
