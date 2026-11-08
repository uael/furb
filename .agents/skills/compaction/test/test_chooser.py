"""CHOOSER, the actor that choice asks."""

from pathlib import Path

from conftest import STANDS
from furb import engine
from skills.conftest import Refusing, compacting, extended


async def test_chooser_is_the_actor_that_choice_asks(tmp_path: Path) -> None:
  """CHOOSER is the actor that choice asks when it is given none, and an empty CHOOSER is the actor that automatic() gives, else the default actor of the chain of the model that chooses."""
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


async def test_when_the_model_of_automatic_work_refuses_its_reply_choice_asks_the_next(tmp_path: Path) -> None:
  """When the model of automatic work refuses its reply, choice asks the next actor of automatic work, automatic(1), then the next, each on a new chain labelled chosen, and the refusal of the last ends the prompt."""
  roster = [*STANDS[0], ["h", ["low"], 200000, "cheap", 0], ["k", ["low"], 200000, "cheap", 1]]
  sand, root = extended("compaction", tmp_path, sand=Refusing(), roster=roster)
  sand.script[root] = ["await chosen()\nclose(1)"]
  sand.script.update({f"chain{n}": ["close(([], 'k chose'))"] for n in range(2, 6)})
  assert await engine.prompt(int, "choose", on=root) == 1
  chains = [a[1] for a in engine.transcript(root) if a[0] == "chain" and a[4] == "chosen"]
  asked = [a[6] for one in chains for a in engine.transcript(one) if a[0] == "prompt" and a[3] == one]
  assert asked == ["h", "k"]
  (tmp_path / "all").mkdir()
  sand, root = extended("compaction", tmp_path / "all", sand=Refusing(), roster=roster[:-1])
  sand.script[root] = ["try:\n  await chosen()\nexcept Refused as no:\n  close(str(no))"]
  assert "h answered nothing" in await engine.prompt(str, "choose", on=root)
