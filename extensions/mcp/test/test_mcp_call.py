"""mcp_call, a call of a tool."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import given, mcped
from furb import engine
from furb.engine import Refused

RESULT = {"text": "one\ntwo", "content": [{"type": "text", "text": "one\ntwo"}], "structured": None}
"""RESULT is what the MCP of the suite answers a call with."""


async def test_mcp_call_is_an_act_that_calls_a_tool_of_a_server(tmp_path: Path) -> None:
  """mcp_call(server, tool, arguments) is an act that calls a tool of a server under the original names of its arguments, which a word awaits for an McpResult."""
  root = mcped(tmp_path, {"mcp_call": RESULT})[1]
  assert await engine.rung("close(await mcp_call('github', 'create_issue', {'title': 'x'}))", on=root) == RESULT


async def test_it_puts_the_server_the_tool_and_the_arguments_to_the_world(tmp_path: Path) -> None:
  """It puts the server, the tool and the arguments to the World, and the arguments are an empty dict when it is given none."""
  root, heard = mcped(tmp_path, {"mcp_call": RESULT})[1:]
  await engine.rung("await mcp_call('s', 't', {'n': 1})\nawait mcp_call('s', 't')", on=root)
  assert [a[4:] for a in heard] == [("s", "t", {"n": 1}), ("s", "t", {})]


async def test_its_open_tells_the_server_and_the_tool_and_its_done_tells_the_lines_of_the_text_and_the_images(
  tmp_path: Path,
) -> None:
  """Its open tells the server and the tool, and the arguments when their text holds at most 200 characters, and binds the act to Act[McpResult]; its done tells under the word text the lines of the text of the result that its show picks, and a line of Markdown for each image."""
  image = {"type": "image", "image": "furb-image://ab12", "mime": "image/png"}
  root = mcped(tmp_path, {"mcp_call": {"text": "one\ntwo", "content": [image], "structured": None}})[1]
  await given(root, G="github", C="create")
  big = "x" * 300
  word = f"await mcp_call(G, C, {{'title': 'x'}}, span(1, 1))\nawait mcp_call(G, C, {{'b': {big!r}}})"
  await engine.rung(word, on=root)
  opened = "#mcp_call{n}\nmcp_call{n}_server = 'github'\nmcp_call{n}_tool = 'create'\n"
  bound = "mcp_call{n}: Act[McpResult] = Act('mcp_call{n}')"
  assert of(engine.turns(on=root), "mcp_call") == [
    opened.format(n=1) + "mcp_call1_arguments = {'title': 'x'}\n" + bound.format(n=1),
    "#mcp_call1 done\n<s:mcp_call1_text>\none\n![image](furb-image://ab12)</s:mcp_call1_text>",
    opened.format(n=2) + bound.format(n=2),
    "#mcp_call2 done\n<s:mcp_call2_text>\none\ntwo\n![image](furb-image://ab12)</s:mcp_call2_text>",
  ]


async def test_a_call_that_the_world_refuses_completes_with_refused_and_its_done_tells_nothing(tmp_path: Path) -> None:
  """A result with isError, a tool that the configs deny, a server that is not connected and a call past its timeout complete with Refused, which an await raises in the caller, and its done tells nothing."""
  root = mcped(tmp_path, {"mcp_call": Refused("Tool failed: boom")})[1]
  await given(root, S="srv", T="tt")
  with pytest.raises(Refused, match="boom"):
    await engine.rung("await mcp_call(S, T)", on=root)
  opened = "#mcp_call1\nmcp_call1_server = 'srv'\nmcp_call1_tool = 'tt'\nmcp_call1: Act[McpResult] = Act('mcp_call1')"
  assert of(engine.turns(on=root), "mcp_call") == [opened]
