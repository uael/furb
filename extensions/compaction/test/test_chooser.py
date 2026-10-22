"""CHOOSER, the actor that choice asks."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_chooser_is_the_actor_that_choice_asks(tmp_path: Path) -> None:
  """CHOOSER is the actor that choice asks when it is given none, and an empty CHOOSER is the default actor of the chain of the model that chooses."""
  sand, root, _ = compacting(tmp_path)
  sand.script.update(
    {root: ["await chosen()\nCHOOSER = 'n/low'\nawait chosen()\nclose(1)"], "chain2": ["close(([], ''))"]}
  )
  sand.script["chain3"] = ["close(([], ''))"]
  assert await engine.prompt(int, "choose twice", on=root) == 1
  asked = [
    a
    for one in ("chain2", "chain3")
    for a in engine.transcript(one)
    if (a[0] == "prompt" or a[:1] + a[4:5] == ("rung", "")) and a[3] == one
  ]
  assert [(a[0], a[6]) for a in asked] == [("prompt", ""), ("rung", "m/low"), ("prompt", "n/low"), ("rung", "n/low")]
