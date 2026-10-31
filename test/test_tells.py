"""tells, whether an act tells."""

import pytest

from conftest import STANDS, born, chained, said, settle
from furb import engine


async def test_whether_an_act_tells() -> None:
  """Whether an act tells: a rung that its chain made to retell another tells nothing, neither its word nor what its word does, since what it would tell stands told already."""
  _, log, root = born()
  step = engine.rung("t = read('a.txt')\nraise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await step
  twin = await chained("twin", root, 300)
  (again,) = [a[1] for a in said(log, "rung") if a[3] == twin and a[5] == step]
  assert engine.tells(step) and not engine.tells(again) and engine.get(again)[2] == twin
  told = [a[3][0].split("\n")[0] for a in said(engine.transcript(root), "tell") if a[2] == step]
  assert told == ["t = read('a.txt')", "#read1", f"#{step} raised"]
  assert [a for a in engine.transcript(twin) if a[:2] == ("tell", again)] == []
  assert engine.module(twin)["t"] == engine.module(root)["t"]


async def test_the_rung_of_the_stand_that_a_chain_plays_on_itself_tells_as_any_rung() -> None:
  """The rung of the stand that a chain plays on itself retells no rung, so it tells as any rung, and the stand and the cd of its word tell their paragraphs."""
  _, log, _ = born(stands=[*STANDS, "/w/DOCTRINE.md", []], files={"/w/DOCTRINE.md": "Read first.\n"})
  two = await chained("two")
  assert await engine.rung("k = 1", on=two) is None
  await settle()
  (placed,) = [a for a in said(log, "rung") if a[3] == two and a[2] == two and a[4].startswith("stand()\n")]
  assert placed[4:] == ("stand()\ncd('/w')", "", "m/low") and engine.tells(placed[1])
  made = [a[1] for a in said(log, "stand") + said(log, "cd") if a[2] == placed[1]]
  told = [a[3][0].split("\n")[0] for a in said(engine.transcript(two), "tell") if a[1] in made]
  assert told == [f"#{one}" for one in made] and len(made) == 2
