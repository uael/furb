"""counted, how many skills a chain knows."""

from pathlib import Path

from conftest import chained, of
from extensions.conftest import extended, skilled
from furb import engine
from furb_monty import _monty


async def test_how_many_skills_a_chain_knows(tmp_path: Path) -> None:
  """How many skills a chain knows, which it tells in one line, so a reply reads the count and not the list."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  _, root = extended("skills", tmp_path, _monty.skills)
  assert await engine.rung("close(counted())", on=root) == 3


async def test_counted_reads_the_path_skills_on_its_chain_with_the_show_hidden(tmp_path: Path) -> None:
  """counted reads the path skills:// on its chain with the show HIDDEN, so the list stands in no turns."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  await engine.rung("counted()", on=root)
  assert "Make tea." not in "\n".join(of(engine.turns(on=root), "read"))


async def test_counted_tells_the_read_under_the_word_value(tmp_path: Path) -> None:
  """counted tells the read under the word value, as one line with the count of the skills and the word skills() that lists them, and gives the count."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung("close(counted())", on=root)
  assert (got, of(engine.turns(on=root), "read")) == (
    2,
    ["#read1\nread1_path = 'skills://'\nread1_value = '2 skills: skills() lists them'"],
  )


async def test_the_life_word_of_the_extension_is_counted(tmp_path: Path) -> None:
  """The life word of the extension is counted(), so a chain tells the count of its skills when the life enables the extension, and a chain born later tells it at its birth."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills, lives=True)
  two = await chained("two")
  assert (of(engine.turns(on=root), "read"), of(engine.turns(on=two), "read")) == (
    ["#read1\nread1_value = '2 skills: skills() lists them'"],
    ["#read2\nread2_value = '2 skills: skills() lists them'"],
  )
