"""mcp_act, the act that every mcp act of the World shares."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import mcped
from furb import engine
from furb.engine import Refused

WORD = "mcp_act('mcp_read', '', 'McpResult', {'server': 's', 'uri': 'u'}, HEAD, 's', 'u')"
"""WORD is a call of mcp_act that makes the act of a read."""

OPENED = "#mcp_read1\nmcp_read1_server = 's'\nmcp_read1_uri = 'u'\nmcp_read1: Act[McpResult] = Act('mcp_read1')"
"""OPENED is what the open of that act tells."""


async def test_mcp_act_is_the_act_that_every_mcp_act_of_the_world_shares(tmp_path: Path) -> None:
  """mcp_act(kind, on, of, opened, show, *words) is the act that every mcp act of the World shares: it puts the words to the World, its open tells the words of opened and binds the act to Act of the type that of names, and its done tells what mcp_closed gives for the answer."""
  answer = {"text": "hi", "content": []}
  root, heard = mcped(tmp_path, {"mcp_read": answer})[1:]
  assert await engine.rung(f"close(await {WORD})", on=root) == answer
  assert [a[4:] for a in heard] == [("s", "u")]
  assert of(engine.turns(on=root), "mcp_read") == [OPENED, "#mcp_read1 done\nmcp_read1_text = 'hi'"]


async def test_an_act_that_the_world_refuses_completes_with_refused_and_its_done_tells_nothing(tmp_path: Path) -> None:
  """An act that the World refuses completes with Refused, which an await raises in the caller, and its done tells nothing."""
  root = mcped(tmp_path, {"mcp_read": Refused("No such resource.")})[1]
  with pytest.raises(Refused, match=r"No such resource\."):
    await engine.rung(f"await {WORD}", on=root)
  assert of(engine.turns(on=root), "mcp_read") == [OPENED]
