"""skill, a skill read into a chain."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import extended, skilled
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty


def brewed(tmp_path: Path) -> tuple[str, Path]:
  """A life that stands in the folder work of tmp_path, whose one skill is brew: the root, and the SKILL.md file."""
  path = skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  return extended("skills", tmp_path, _monty.skills)[1], path


async def test_a_skill_read_into_a_chain(tmp_path: Path) -> None:
  """A skill read into a chain: the SKILL.md file of the skill of that name, which the chain tells as a read."""
  root, path = brewed(tmp_path)
  assert await engine.rung('close(skill("brew"))', on=root) == Text(str(path), path.read_text(encoding="utf-8"))


async def test_skill_reads_the_path_skills_and_the_name_with_its_show(tmp_path: Path) -> None:
  """skill reads the path skills:// and the name, with its show, so its lines stand in the turns as the lines of any read."""
  root, path = brewed(tmp_path)
  await engine.rung('skill("brew", span(1, 1))', on=root)
  assert of(engine.turns(on=root), "read") == [f"#read skills://brew\n# {path}, 0 known\n# 1 ---"]


async def test_the_world_answers_it_with_the_text_of_the_skill_md_file(tmp_path: Path) -> None:
  """The World answers it with the text of the SKILL.md file at the path of that file, so a later read of that path tells no line that the chain knows."""
  root, path = brewed(tmp_path)
  await engine.rung(f'skill("brew")\nread({str(path)!r})', on=root)
  assert of(engine.turns(on=root), "read")[1] == f"#read {path}\n# {path}, 5 known"


async def test_a_skill_of_a_name_that_no_skill_has_raises_refused(tmp_path: Path) -> None:
  """A skill of a name that no skill has raises Refused in the caller."""
  root, _ = brewed(tmp_path)
  with pytest.raises(Refused, match=r"There is no skill tea\."):
    await engine.rung('close(skill("tea"))', on=root)
