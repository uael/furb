"""The built-in skills of furb stand in .agents/skills, as skills that Claude Code reads too, with no folder of extensions."""

import re
import shutil
import subprocess
from pathlib import Path

from furb.engine import OPERATOR
from furb_monty import _monty
from skills.conftest import Bench

ROOT = Path(__file__).parents[2]
HOME = ROOT / ".agents" / "skills"
PLAIN = {"name", "description", "metadata"}
"""PLAIN is the fields of the frontmatter of a skill that Claude Code reads at the top of a SKILL.md: furb gives its own
under metadata alone."""


def fields(manifest: Path) -> tuple[list[str], list[str]]:
  """The keys at the top of the frontmatter of a SKILL.md file, and the keys under its metadata."""
  head = manifest.read_text(encoding="utf-8").split("---\n")[1]
  top = re.findall(r"^([\w-]+):", head, re.M)
  under = re.findall(r"^  ([\w-]+):", head, re.M)
  return top, under


def test_each_built_in_skill_stands_in_the_skills_of_the_repository_and_no_folder_of_extensions_is_left() -> None:
  """Each skill that furb carries stands in .agents/skills with its manifest, its word and its interface, and the
  repository holds no folder extensions, at its top or in .furb."""
  for one in _monty.official():
    folder = HOME / one["name"]
    _, under = fields(folder / "SKILL.md")
    assert (folder / f"{one['name']}.py").is_file() and (folder / f"{one['name']}.pyi").is_file(), folder
    assert "furb-word" in under, folder
  tracked = subprocess.run(  # noqa: S603
    [shutil.which("git") or "git", "ls-files", "extensions", ".furb/extensions"],
    cwd=ROOT,
    capture_output=True,
    text=True,
    check=True,
  )
  assert tracked.stdout == ""


def test_the_manifest_of_each_built_in_skill_is_an_ordinary_skill_with_the_fields_of_furb_under_its_metadata() -> None:
  """The manifest of each built-in skill gives at its top only the fields of an ordinary skill, a name that is its
  folder and a description, so Claude Code reads it, and each field of furb under metadata, which only furb reads."""
  for one in _monty.official():
    manifest = HOME / one["name"] / "SKILL.md"
    top, under = fields(manifest)
    assert set(top) <= PLAIN and {"name", "description"} <= set(top), (manifest, top)
    assert re.search(rf"^name: {re.escape(one['name'])}$", manifest.read_text(encoding="utf-8"), re.M), manifest
    assert all(key.startswith("furb-") for key in under), (manifest, under)


async def test_a_script_of_a_skill_folder_gives_the_same_output_through_bash_as_from_a_shell(tmp_path: Path) -> None:
  """A script in the folder of a skill runs through bash() of a chain as a shell runs it, as the tool Bash of Claude
  Code does, with no shim: both give the same output from the one folder."""
  folder = tmp_path / "kit"
  folder.mkdir()
  (folder / "SKILL.md").write_text(
    "---\nname: kit\ndescription: Counts.\n---\nRun sh kit/count.sh.\n", encoding="utf-8"
  )
  (folder / "count.sh").write_text('printf "%s %s\\n" "$(basename "$(dirname "$0")")" "$((6 * 7))"\n', encoding="utf-8")
  shell = subprocess.run(["sh", "kit/count.sh"], cwd=tmp_path, capture_output=True, text=True, check=True).stdout  # noqa: S607
  bench = Bench(tmp_path, [], [[OPERATOR, [], 200000]], OPERATOR)
  await bench.word(f"cd({str(tmp_path)!r})")
  got = await bench.value('(await bash("sh kit/count.sh")).stdout.content')
  assert shell == "kit 42\n"
  assert got == shell
