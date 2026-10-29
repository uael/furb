"""McpResult, what a tool, a resource and a prompt give."""

from pathlib import Path

from extensions.conftest import mcped
from furb import engine


async def test_mcp_result_is_a_dict_whose_values_a_word_uses_with_no_narrowing(tmp_path: Path) -> None:
  """McpResult is what a tool, a resource and a prompt give: a dict whose key text holds the text of its blocks joined, and whose other keys hold the answer of the World, the content, the structured result or the messages."""
  root = mcped(tmp_path, {})[1]
  assert engine.gate("x: McpResult = {}\nclose(x['text'].upper())", on=root) == []
