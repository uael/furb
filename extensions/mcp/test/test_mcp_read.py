"""mcp_read, a read of a resource."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import given, mcped
from furb import engine
from furb.engine import Refused

ANSWER = {"text": "a\nb", "content": [{"type": "text", "text": "a\nb"}]}
"""ANSWER is what the MCP of the suite answers a read with."""

OPENED = (
  "#mcp_read1\nmcp_read1_server = 'srv'\nmcp_read1_uri = 'file:///a'\nmcp_read1: Act[McpResult] = Act('mcp_read1')"
)
"""OPENED is what the open of the first read tells."""


async def test_mcp_read_is_an_act_that_reads_a_resource_of_a_server(tmp_path: Path) -> None:
  """mcp_read(server, uri) is an act that reads a resource of a server, which a word awaits for an McpResult."""
  root = mcped(tmp_path, {"mcp_read": ANSWER})[1]
  assert await engine.rung("close(await mcp_read('s', 'file:///a'))", on=root) == ANSWER


async def test_it_puts_the_server_and_the_uri_to_the_world_and_tells_the_lines_of_the_text(tmp_path: Path) -> None:
  """It puts the server and the uri to the World, its open tells them and binds the act to Act[McpResult], and its done tells under the word text the lines of the text that its show picks."""
  root, heard = mcped(tmp_path, {"mcp_read": ANSWER})[1:]
  await given(root, S="srv", U="file:///a")
  await engine.rung("await mcp_read(S, U, span(1, 1))", on=root)
  assert [a[4:] for a in heard] == [("srv", "file:///a")]
  assert of(engine.turns(on=root), "mcp_read") == [OPENED, "#mcp_read1 done\nmcp_read1_text = 'a'"]


async def test_a_read_that_the_world_refuses_completes_with_refused_and_its_done_tells_nothing(tmp_path: Path) -> None:
  """A read that the World refuses completes with Refused, and its done tells nothing."""
  root = mcped(tmp_path, {"mcp_read": Refused("No such resource.")})[1]
  await given(root, S="srv", U="file:///a")
  with pytest.raises(Refused, match=r"No such resource\."):
    await engine.rung("await mcp_read(S, U)", on=root)
  assert of(engine.turns(on=root), "mcp_read") == [OPENED]
