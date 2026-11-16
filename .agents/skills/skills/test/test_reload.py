"""reload, what a chain plays, taken again."""

from pathlib import Path

from conftest import Sand, World
from furb import engine
from furb_monty import _monty
from skills.conftest import extended, skilled

NOTE = "from furb.engine import Text, read\n\n\ndef note() -> Text:\n  return read('{}')\n"
"""NOTE is the file of the word of the skill note, whose read names a path."""

NEWEST = "furb runs its newest build."
"""NEWEST is what the host of the suite answers each act upgrade with."""


class Moved(Sand):
  """The World of the suite, whose host answers the act upgrade as the newest build serves."""

  def hear(self, a: tuple) -> World:
    """An act upgrade is done with NEWEST, and every other fact goes to the Sand."""
    match a:
      case ("upgrade", qid, *_):
        yield "done", qid, NEWEST
      case _:
        yield from super().hear(a)


def moved(tmp_path: Path) -> str:
  """The root of a life that runs the extension skills on the World Moved."""
  return extended("skills", tmp_path, _monty.skills, sand=Moved())[1]


async def test_the_key_of_a_reload_runs_it_and_a_model_runs_it_as_any_word(tmp_path: Path) -> None:
  """What a chain plays, taken again, which the key of a reload in a window runs on the root of its session, and a model runs as it runs any word."""
  root = moved(tmp_path)
  ran = engine.rung("said = await reload()", on=root)
  await ran
  assert [a[2] for a in engine.transcript(root) if a[0] == "upgrade"] == [ran]


async def test_reload_loads_again_each_skill_with_code_that_the_life_runs(tmp_path: Path) -> None:
  """reload loads again with HIDDEN each skill with code that the life runs on the chain, so the World enables again each word that changed on the disk, and the turn after it tells no text of a skill."""
  folder = skilled(
    tmp_path / "work" / ".furb" / "skills", "note", "name: note\ndescription: A note.\nmetadata:\n  furb-word: note.py"
  ).parent
  (folder / "note.py").write_text(NOTE.format("n"), encoding="utf-8", newline="")
  root = moved(tmp_path)
  await engine.rung('await skill("note")', on=root)
  (folder / "note.py").write_text(NOTE.format("m"), encoding="utf-8", newline="")
  await engine.rung("await reload()", on=root)
  words = [a[4] for a in engine.transcript(on=root) if a[0] == "enable" and a[3] == "note"]
  assert words == ["def note() -> Text:\n  return read('n')", "def note() -> Text:\n  return read('m')"]
  assert "A note." not in engine.turns(on=root)[-1][1]


async def test_reload_awaits_mcp_servers_when_the_module_binds_it(tmp_path: Path) -> None:
  """When the module of the chain binds mcp_servers, reload awaits it, so the kit mcp reads the servers of the configs again."""
  root = moved(tmp_path)
  await engine.rung("asked = []\nasync def mcp_servers(on=''):\n  asked.append(on)", on=root)
  await engine.rung("await reload()", on=root)
  assert engine.module(root)["asked"] == [""]


async def test_reload_gives_what_the_world_answers_the_act_upgrade(tmp_path: Path) -> None:
  """reload then awaits the act upgrade with no words on its chain, and gives what the World answers it: the host of the life moves furb to its newest build."""
  root = moved(tmp_path)
  await engine.rung("said = await reload()", on=root)
  assert [a[3:] for a in engine.transcript(root) if a[0] == "upgrade"] == [(root,)]
  assert engine.module(root)["said"] == NEWEST
