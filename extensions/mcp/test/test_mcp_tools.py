"""mcp_tools, the signatures of the tools of a server."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import mcped, quoted, server, tooled
from furb import engine
from furb.engine import Refused

TITLE = {"type": "object", "properties": {"title": {"type": "string"}}, "required": ["title"]}
"""TITLE is the input schema of a tool that needs a title."""

GITHUB = server("github", tools=[tooled("create", "Create an issue.", TITLE), tooled("ping")])
"""GITHUB is a server with two tools."""


async def test_mcp_tools_is_an_act_that_tells_the_signatures_of_the_tools_of_a_server(tmp_path: Path) -> None:
  """mcp_tools(server) is an act that tells the signatures of the tools of a server, one line each as mcp_signature gives it, and completes with the same text."""
  root = mcped(tmp_path, {"mcp_servers": [GITHUB]})[1]
  text = "create(title: str): Create an issue.\nping()"
  assert await engine.rung("await mcp_servers()\nclose(await mcp_tools('github'))", on=root) == text
  opened = "#mcp_tools1\nmcp_tools1_server = 'github'\n" + quoted("mcp_tools1_tools", text)
  assert of(engine.turns(on=root), "mcp_tools") == [opened + "\nmcp_tools1: Act[str] = Act('mcp_tools1')"]


async def test_it_asks_the_world_nothing_and_a_server_that_the_list_does_not_hold_completes_it_with_refused(
  tmp_path: Path,
) -> None:
  """It asks the World nothing: it reads the last list of servers that the chain holds, so a server that the list does not hold completes it with Refused, which its open tells."""
  root, heard = mcped(tmp_path, {"mcp_servers": [GITHUB]})[1:]
  with pytest.raises(Refused, match=r"No server zzz is known\. The servers are: github\."):
    await engine.rung("await mcp_servers()\nawait mcp_tools('zzz')", on=root)
  assert [a[0] for a in heard] == ["mcp_servers"]
  error = "mcp_tools1_error = 'No server zzz is known. The servers are: github.'"
  assert of(engine.turns(on=root), "mcp_tools") == ["#mcp_tools1\nmcp_tools1_server = 'zzz'\n" + error]
