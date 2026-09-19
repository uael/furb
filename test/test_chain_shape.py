"""Chain, the act that carries the label and the source of a chain."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_chain_carries_the_label_and_the_source_of_a_chain() -> None:
  """A chain carries the label and the source of a chain, and every chain is chain://lineage.n whether boot or chain opened it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two", source=root)
  await settle()
  assert said(log, "chain") == [("chain", root, OPERATOR, "", "root", ""), ("chain", two, OPERATOR, "", "two", root)]
  assert (root, two) == ("chain://operator.1", "chain://operator.2")
  sand.script[root] = ["close(chain('deep'))"]
  deep = await engine.prompt(str, "fork", on=root)
  assert engine.get(deep)[4:] == ("deep", "")
  assert deep == "chain://operator.3.1.1"
