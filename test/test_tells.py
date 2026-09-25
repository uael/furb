"""tells, whether an act tells."""

import pytest

from conftest import life, said, settle, sown
from furb import engine


async def test_whether_an_act_tells() -> None:
  """Whether an act tells: a rung that its chain made tells nothing, neither its word nor what its word does, since what it would tell stands told already."""
  sand = sown()
  log, root = life(sand)
  step = engine.rung("t = read('a.txt')\nraise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await step
  twin = engine.chain("twin", source=root)
  await settle(300)
  (again,) = [a[1] for a in said(log, "rung") if a[3] == twin]
  assert engine.tells(step) and not engine.tells(again) and engine.get(again)[2] == twin
  told = [a[3][0].split("\n")[0] for a in said(engine.transcript(root), "tell") if a[1] == step]
  assert told == [f"#{step}", "#read a.txt", f"#{step} raised ValueError('boom')"]
  assert [a for a in engine.transcript(twin) if a[:2] == ("tell", again)] == []
  assert engine.module(twin)["t"] == engine.module(root)["t"]
