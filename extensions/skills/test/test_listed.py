"""listed, the skills of a chain as a turn of its model."""

from pathlib import Path

from conftest import chained, of
from extensions.conftest import extended, skilled
from furb import engine
from furb_monty import _monty

TURN = ["assistant", "read('skills://')"]


def crafted(on: str) -> list[list]:
  return [list(one[:2]) for one in engine.turns(on=on)]


async def test_the_skills_of_a_chain_as_a_turn_of_its_model(tmp_path: Path) -> None:
  """The skills of a chain as a turn of its model: listed runs read('skills://') on its chain as a rung of the default actor of the chain, so the turns show the model that reads the list of its skills, then that read as every read tells it."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  await engine.rung("await listed()", on=root)
  turns = crafted(root)
  at = turns.index(TURN)
  assert turns[at + 1][0] == "user"
  assert "Make tea." in turns[at + 1][1]
  assert "Make tea." in "\n".join(of(engine.turns(on=root), "read"))


async def test_listed_awaits_that_rung(tmp_path: Path) -> None:
  """listed awaits that rung, so the rung that runs listed ends once the list stands in the turns, and the chain acknowledges nothing of it."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  outer = engine.rung("await listed()", on=root)
  await outer
  (inner,) = [a[1] for a in engine.transcript(root) if a[0] == "rung" and a[2] == outer]
  done = [a[1] for a in engine.transcript(root) if a[0] == "done"]
  assert done.index(inner) < done.index(outer)
  assert [a for a in engine.transcript(root) if a[0] == "prompt"] == []


async def test_the_life_word_of_the_extension_is_await_listed(tmp_path: Path) -> None:
  """The life word of the extension is await listed(), so a chain reads its skills when the life enables the extension, and a chain born later reads them at its birth."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills, lives=True)
  two = await chained("two")
  assert (TURN in crafted(root), TURN in crafted(two)) == (True, True)
