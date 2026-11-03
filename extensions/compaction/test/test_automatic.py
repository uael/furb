"""automatic, the actor of automatic work."""

import json
from pathlib import Path

from conftest import STANDS
from extensions.conftest import extended
from furb import engine


async def test_automatic_gives_the_actor_of_automatic_work(tmp_path: Path) -> None:
  """automatic() gives the actor of automatic work: the first model of the list automatic of .furb/config.json in the directory of the standing that the roster holds, else the first cheap model of the roster, else an empty str."""
  sand, root = extended("compaction", tmp_path)
  work = str(tmp_path / "work")
  config = f"{work}/.furb/config.json"
  roster = [*STANDS[0], ["h", ["low"], 200000, "cheap"], ["s", [], 200000, "dear"], ["k", [], 200000, "cheap"]]
  got = []
  for offers, listed in ((roster, ["ghost", "s", "h"]), (roster, ["ghost"]), (STANDS[0], ["ghost"])):
    sand.stands = [offers, work, "m/low"]
    sand.files[config] = json.dumps({"automatic": listed})
    sand.script[root] = ["stand()\nclose(automatic())"]
    got.append(await engine.prompt(str, "which", on=root))
  assert got == ["s", "h", ""]
