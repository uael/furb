"""automatic, the actor of automatic work."""

import json
from pathlib import Path

from conftest import STANDS
from extensions.conftest import extended
from furb import engine


async def test_automatic_gives_the_actor_of_automatic_work(tmp_path: Path) -> None:
  """automatic() gives the actor of automatic work: the model of the roster with the first place in the list of automatic work, which the entry of each model of that list gives as its fifth field, else the first cheap model of the roster, else an empty str."""
  sand, root = extended("compaction", tmp_path)
  work = str(tmp_path / "work")
  config = f"{work}/.furb/config.json"
  plain = [*STANDS[0], ["h", ["low"], 200000, "cheap"], ["s", [], 200000, "dear"], ["k", [], 200000, "cheap"]]
  placed = [*STANDS[0], ["h", ["low"], 200000, "cheap", 1], ["s", [], 200000, "dear", 0], ["k", [], 200000, "cheap"]]
  sand.files[config] = json.dumps({"automatic": ["k"]})
  got = []
  for offers in (placed, plain, STANDS[0]):
    sand.stands = [offers, work, "m/low"]
    sand.script[root] = ["stand()\nclose(automatic())"]
    got.append(await engine.prompt(str, "which", on=root))
  assert got == ["s", "h", ""]
