"""Cwd, the question of the working directory of a chain."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_cwd_gives_the_working_directory_of_the_chain_it_is_on() -> None:
  """A cwd gives the working directory of the chain it is on."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  engine.cd("/x", on=root)
  engine.cd("/y", on=two)
  assert (engine.cwd(on=root), engine.cwd(on=two)) == ("/x", "/y")
  answered = [one for one in said(log, "done") if one[1].startswith("cwd://")]
  assert [(one[2], one[3]) for one in answered] == [(root, "/x"), (two, "/y")]
