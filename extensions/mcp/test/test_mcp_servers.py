"""mcp_servers, the list of the servers."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import mcped, server
from furb import engine
from furb.engine import Refused

OPENED = "#mcp_servers1\nmcp_servers1: Act[list[dict]] = Act('mcp_servers1')"
"""OPENED is what the open of the first list tells."""


async def test_mcp_servers_is_an_act_that_lists_the_servers(tmp_path: Path) -> None:
  """mcp_servers() is an act that lists the servers, which a word awaits for a list of dicts, one for each server of the configs."""
  two = [server("a"), server("b", state="failed", error="x")]
  root = mcped(tmp_path, {"mcp_servers": two})[1]
  assert await engine.rung("close(await mcp_servers())", on=root) == two


async def test_it_puts_the_question_to_the_world_with_no_words_and_tells_a_line_for_each_server(tmp_path: Path) -> None:
  """It puts the question to the World with no words, its open binds the act to Act[list[dict]], and its done tells under the word servers one line for each server."""
  root, heard = mcped(tmp_path, {"mcp_servers": [server("a")]})[1:]
  await engine.rung("await mcp_servers()", on=root)
  assert [a[4:] for a in heard] == [()]
  assert of(engine.turns(on=root), "mcp_servers") == [
    OPENED,
    "#mcp_servers1 done\nmcp_servers1_servers = 'a connected, no tools'",
  ]


async def test_a_list_that_the_world_refuses_completes_with_refused_and_its_done_tells_nothing(tmp_path: Path) -> None:
  """A list that the World refuses completes with Refused, and its done tells nothing."""
  root = mcped(tmp_path, {"mcp_servers": Refused("No config.")})[1]
  with pytest.raises(Refused, match=r"No config\."):
    await engine.rung("await mcp_servers()", on=root)
  assert of(engine.turns(on=root), "mcp_servers") == [OPENED]
