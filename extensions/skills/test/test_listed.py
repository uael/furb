"""listed, the skills of a chain, read where its word runs."""

from pathlib import Path

from conftest import chained, of
from extensions.conftest import extended, skilled
from furb import engine
from furb_monty import _monty


async def test_the_skills_of_a_chain_read_where_its_word_runs(tmp_path: Path) -> None:
  """The skills of a chain, read where its word runs: listed runs skills() on its chain, so the turn after the word that runs it shows the list of the skills of the chain as every read tells it."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  ran = engine.rung("await listed()", on=root)
  await ran
  assert [a[2] for a in engine.transcript(root) if a[0] == "read" and a[4] == "skills://"] == [ran]
  assert "Make tea." in "\n".join(of(engine.turns(on=root), "read"))


async def test_the_life_word_of_the_extension_is_await_listed(tmp_path: Path) -> None:
  """The life word of the extension is await listed(), so the setup of the root reads the skills in the turn of the life words of the extensions, and each chain with no source holds that read in its setup."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills, lives=True, doctrine="Read first.\n")
  two = await chained("two")
  for on in (root, two):
    turns = engine.turns(on=on)
    (at,) = [n for n, one in enumerate(turns) if one[0] == "assistant" and one[1].endswith("\nawait listed()")]
    assert "Make tea." in turns[at + 1][1]
