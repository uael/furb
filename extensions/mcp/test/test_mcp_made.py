"""mcp_made, an act typed as the act of its shape."""

from pathlib import Path

from extensions.conftest import mcped
from furb import engine


async def test_mcp_made_gives_the_act_that_it_is_given_typed_as_an_act_of_the_shape(tmp_path: Path) -> None:
  """mcp_made(shape, made) gives the act that it is given, typed as an act of the shape that a word awaits for, since the gate reads each act that the engine gives as an act of object."""
  root = mcped(tmp_path, {})[1]
  assert await engine.rung("close(str(mcp_made(dict, Act('a'))))", on=root) == "a"
  assert engine.gate("x: Act[dict] = mcp_made(dict, Act('a'))", on=root) == []
  assert engine.gate("x: Act[dict] = Act('a')", on=root) != []
