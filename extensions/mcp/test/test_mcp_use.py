"""mcp_use, the binding of a server run as a rung."""

from pathlib import Path

import pytest

from extensions.conftest import bindings, mcped, server
from furb import engine
from furb.engine import Refused


async def test_mcp_use_is_a_rung_that_runs_the_binding_of_a_server_so_its_class_stands_for_the_next_word(
  tmp_path: Path,
) -> None:
  """mcp_use(server) is a rung of the chain that runs the binding of a server as mcp_bound gives it, so the class of its tools stands in the module for the next word."""
  root = mcped(tmp_path, {"mcp_servers": [server("github", binding=bindings("github"))]})[1]
  await engine.rung("await mcp_servers()\nawait mcp_use('github')", on=root)
  assert await engine.rung("close(hasattr(github, 'ping'))", on=root) is True


async def test_a_server_that_the_list_does_not_hold_or_that_has_no_binding_raises_refused(tmp_path: Path) -> None:
  """A server that the last list does not hold, or that has no binding, raises Refused."""
  root = mcped(tmp_path, {"mcp_servers": [server("plain")]})[1]
  with pytest.raises(Refused, match=r"No server zzz is known\. The servers are: none\."):
    await engine.rung("mcp_use('zzz')", on=root)
  await engine.rung("await mcp_servers()", on=root)
  with pytest.raises(Refused, match=r"plain has no tools to bind\."):
    await engine.rung("mcp_use('plain')", on=root)
