"""mcp_prompt, the expansion of a prompt."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import given, mcped
from furb import engine
from furb.engine import Refused

ANSWER = {"text": "Review x", "messages": [{"role": "user", "content": "Review x"}]}
"""ANSWER is what the MCP of the suite answers an expansion with."""

OPENED = "#mcp_prompt1\nmcp_prompt1_server = 'srv'\nmcp_prompt1_name = 'review'\n"
"""OPENED is the start of what the open of the first expansion tells."""

BOUND = "mcp_prompt1: Act[McpResult] = Act('mcp_prompt1')"
"""BOUND is the end of what the open of the first expansion tells."""


async def test_mcp_prompt_is_an_act_that_expands_a_prompt_of_a_server(tmp_path: Path) -> None:
  """mcp_prompt(server, name, arguments) is an act that expands a prompt of a server, which a word awaits for an McpResult."""
  root = mcped(tmp_path, {"mcp_prompt": ANSWER})[1]
  assert await engine.rung("close(await mcp_prompt('s', 'review', {'n': 1}))", on=root) == ANSWER


async def test_it_puts_the_server_the_name_and_the_arguments_as_text_to_the_world_and_tells_the_text(
  tmp_path: Path,
) -> None:
  """It puts the server, the name and the arguments to the World, each argument as text and an empty dict when it is given none; its open tells them and binds the act to Act[McpResult], and its done tells under the word text the lines of the text that its show picks."""
  root, heard = mcped(tmp_path, {"mcp_prompt": ANSWER})[1:]
  await given(root, S="srv", N="review")
  await engine.rung("await mcp_prompt(S, N, {'n': 1})\nawait mcp_prompt(S, N)", on=root)
  assert [a[4:] for a in heard] == [("srv", "review", {"n": "1"}), ("srv", "review", {})]
  assert of(engine.turns(on=root), "mcp_prompt")[:2] == [
    OPENED + "mcp_prompt1_arguments = {'n': '1'}\n" + BOUND,
    "#mcp_prompt1 done\nmcp_prompt1_text = 'Review x'",
  ]
  assert of(engine.turns(on=root), "mcp_prompt")[2] == OPENED.replace("1", "2") + BOUND.replace("1", "2")


async def test_a_prompt_that_the_world_refuses_completes_with_refused_and_its_done_tells_nothing(
  tmp_path: Path,
) -> None:
  """A prompt that the World refuses completes with Refused, and its done tells nothing."""
  root = mcped(tmp_path, {"mcp_prompt": Refused("No such prompt.")})[1]
  await given(root, S="srv", N="review")
  with pytest.raises(Refused, match=r"No such prompt\."):
    await engine.rung("await mcp_prompt(S, N)", on=root)
  assert of(engine.turns(on=root), "mcp_prompt") == [OPENED + BOUND]
