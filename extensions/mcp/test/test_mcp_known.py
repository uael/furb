"""mcp_known, the dict of a server in the last list of servers."""

from pathlib import Path

import pytest

from extensions.conftest import mcped, server
from furb import engine
from furb.engine import Refused


async def test_mcp_known_gives_the_dict_of_a_server_in_the_last_list_of_servers(tmp_path: Path) -> None:
  """mcp_known(server) gives the dict of a server in the last list of servers that an mcp_servers act on the chain came to."""
  one, two = server("a"), server("b", version="2.0")
  states = [[one, two], [one, server("b", version="3.0")]]
  root = mcped(tmp_path, {"mcp_servers": lambda _: states.pop(0)})[1]
  assert await engine.rung("await mcp_servers()\nclose(mcp_known('b'))", on=root) == two
  assert await engine.rung("await mcp_servers()\nclose(mcp_known('b'))", on=root) == server("b", version="3.0")


async def test_a_server_that_the_list_does_not_hold_raises_refused_and_names_the_servers(tmp_path: Path) -> None:
  """A server that the list does not hold raises Refused, which names the servers that it holds."""
  root = mcped(tmp_path, {"mcp_servers": [server("a"), server("b")]})[1]
  with pytest.raises(Refused, match=r"No server z is known\. The servers are: none\."):
    await engine.rung("close(mcp_known('z'))", on=root)
  await engine.rung("await mcp_servers()", on=root)
  with pytest.raises(Refused, match=r"No server z is known\. The servers are: a, b\."):
    await engine.rung("close(mcp_known('z'))", on=root)
