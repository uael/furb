"""mcp_control, an action to a server."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import mcped, server
from furb import engine
from furb.engine import Refused

OPENED = "#mcp_control1\nmcp_control1_action = 'reconnect'\nmcp_control1_server = 'a'\nmcp_control1: Act[dict] = Act('mcp_control1')"
"""OPENED is what the open of the first action tells."""


async def test_mcp_control_is_an_act_that_does_an_action_to_a_server(tmp_path: Path) -> None:
  """mcp_control(action, server) is an act that does an action to a server, which a word awaits for the dict of that server after the action: reconnect, enable, disable, approve, login or logout."""
  root = mcped(tmp_path, {"mcp_control": server("a")})[1]
  assert await engine.rung("close(await mcp_control('reconnect', 'a'))", on=root) == server("a")


async def test_it_puts_the_action_and_the_server_to_the_world_and_tells_the_line_of_the_server(tmp_path: Path) -> None:
  """It puts the action and the server to the World, its open tells them and binds the act to Act[dict], and its done tells the line of the server under the word server."""
  root, heard = mcped(tmp_path, {"mcp_control": server("a")})[1:]
  await engine.rung("await mcp_control('reconnect', 'a')", on=root)
  assert [a[4:] for a in heard] == [("reconnect", "a")]
  assert of(engine.turns(on=root), "mcp_control") == [
    OPENED,
    "#mcp_control1 done\nmcp_control1_server = 'a connected, no tools'",
  ]


async def test_any_other_action_raises_refused_before_the_call_makes_an_act(tmp_path: Path) -> None:
  """Any other action raises Refused before the call makes an act."""
  root, heard = mcped(tmp_path, {"mcp_control": server("a")})[1:]
  with pytest.raises(Refused, match="No action explode"):
    await engine.rung("mcp_control('explode', 'a')", on=root)
  assert heard == []
  assert of(engine.turns(on=root), "mcp_control") == []


async def test_an_action_that_the_world_refuses_completes_with_refused_and_its_done_tells_nothing(
  tmp_path: Path,
) -> None:
  """An action that the World refuses completes with Refused, and its done tells nothing."""
  root = mcped(tmp_path, {"mcp_control": Refused("The login ended.")})[1]
  for action in ["reconnect", "enable", "disable", "approve", "login", "logout"]:
    word = f"await mcp_control({action!r}, 'a')"
    with pytest.raises(Refused, match=r"The login ended\."):
      await engine.rung(word, on=root)
  assert of(engine.turns(on=root), "mcp_control")[0] == OPENED
  assert len(of(engine.turns(on=root), "mcp_control")) == 6
