"""mcp_tool, the whole description of a tool."""

import json
from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import mcped, quoted, server, tooled
from furb import engine
from furb.engine import Refused

TITLE = {"type": "object", "properties": {"title": {"type": "string"}}, "required": ["title"]}
"""TITLE is the input schema of a tool that needs a title."""

CREATE = tooled(
  "create", "Create an issue.", TITLE, title="Create", output={"type": "object"}, annotations={"x": False}
)
"""CREATE is a tool with every part that mcp_tool tells."""


async def test_mcp_tool_is_an_act_that_tells_the_whole_description_of_a_tool(tmp_path: Path) -> None:
  """mcp_tool(server, name) is an act that tells the title, the description, the input schema, the output schema and the annotations of a tool, and completes with the dict of the tool."""
  root = mcped(tmp_path, {"mcp_servers": [server("github", tools=[CREATE])]})[1]
  assert await engine.rung("await mcp_servers()\nclose(await mcp_tool('github', 'create'))", on=root) == CREATE
  head = "#mcp_tool1\nmcp_tool1_server = 'github'\nmcp_tool1_tool = 'create'\nmcp_tool1_title = 'Create'\n"
  head += "mcp_tool1_description = 'Create an issue.'\n"
  dumps = [
    quoted(f"mcp_tool1_{k}", json.dumps(v, indent=1)) for k, v in (("input", TITLE), ("output", {"type": "object"}))
  ]
  dumps.append(quoted("mcp_tool1_annotations", json.dumps({"x": False}, indent=1)))
  assert of(engine.turns(on=root), "mcp_tool") == [
    head + "\n".join(dumps) + "\nmcp_tool1: Act[dict] = Act('mcp_tool1')"
  ]


async def test_it_asks_the_world_nothing_and_a_tool_that_the_list_does_not_hold_completes_it_with_refused(
  tmp_path: Path,
) -> None:
  """It asks the World nothing: a server or a tool that the last list does not hold completes it with Refused, which its open tells."""
  root, heard = mcped(tmp_path, {"mcp_servers": [server("github", tools=[CREATE])]})[1:]
  with pytest.raises(Refused, match=r"No tool zzz on github\."):
    await engine.rung("await mcp_servers()\nawait mcp_tool('github', 'zzz')", on=root)
  with pytest.raises(Refused, match=r"No server nope is known"):
    await engine.rung("await mcp_tool('nope', 'create')", on=root)
  assert [a[0] for a in heard] == ["mcp_servers"]
  head = "#mcp_tool1\nmcp_tool1_server = 'github'\nmcp_tool1_tool = 'zzz'\n"
  assert of(engine.turns(on=root), "mcp_tool")[0] == head + "mcp_tool1_error = 'No tool zzz on github.'"
