"""OPERATOR, the name of the operator."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_operator_is_the_name_of_the_operator_in_the_roster_and_as_an_actor() -> None:
  """OPERATOR is the name of the operator in the roster and as an actor."""
  assert OPERATOR == "operator"
  assert (OPERATOR, (), 200000) in STANDS[0]
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert root == f"chain://{OPERATOR}.1"
  wanted = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert said(log, "ask") == [] and wanted not in engine.outcomes
  engine.close(12, wanted)
  await settle()
  assert (await wanted) == 12
