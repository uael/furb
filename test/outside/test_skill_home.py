"""The built-in skills of furb stand in .agents/skills, as skills that Claude Code reads too, with no folder of extensions."""

import io
import re
import shutil
import subprocess
import tokenize
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


def test_each_built_in_skill_stands_in_the_skills_of_the_repository_and_no_former_folder_is_left() -> None:
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


QUOTED = [r"//[^\n]*", r"/\*[\s\S]*?\*/", r'"(?:[^"\\]|\\[\s\S])*"']
PROSE = {
  ".rs": re.compile("|".join([*QUOTED, r'r(#*)"[\s\S]*?"\1', r"'(?:[^'\\\n]|\\[^'\n]{1,10})'"])),
  ".ts": re.compile(
    "|".join(
      [
        *QUOTED,
        r"'(?:[^'\\\n]|\\.)*'",
        r"`(?:[^`\\]|\\[\s\S])*`",
        r"[(,=:\[!&|?{};]\s*/(?![/*])(?:[^/\\\n\[]|\\.|\[(?:[^\]\\\n]|\\.)*\])+/",
      ]
    )
  ),
}
"""PROSE is a comment, or a literal of a string, of a char or of a regular expression, of Rust or of TypeScript. It holds
prose or the name that a record holds, and names nothing in code but the names that a string puts in its text, as
{name} of Rust and ${name} of TypeScript do. A lifetime of Rust is no literal."""
STANDARD = re.compile(r"\.extension\(\)|\.with_extension\(")
"""STANDARD is a call of the standard library of Rust on the extension of the name of a file, which is no skill."""


def put(literal: re.Match[str]) -> str:
  """The names that a string of Rust or of TypeScript puts in its text, and none for a comment or another literal."""
  return " ".join(re.findall(r"\{([A-Za-z_][\w.]*)", literal[0])) if literal[0][0] in '"`r' else " "


def names(path: Path) -> list[str]:
  """The names in the code of a file of Rust, Python or TypeScript, with no comment, no docstring and no literal."""
  text = path.read_text(encoding="utf-8")
  if path.suffix in {".py", ".pyi"}:
    lines = io.StringIO(text).readline
    return [one.string for one in tokenize.generate_tokens(lines) if one.type == tokenize.NAME]
  return re.findall(r"[A-Za-z_]\w*", PROSE[path.suffix.replace(".cts", ".ts")].sub(put, STANDARD.sub(" ", text)))


def test_each_name_in_code_says_skill_and_the_name_of_the_ear_that_a_record_holds_stands_as_a_literal() -> None:
  """A skill has one name in code: no name of Rust, Python or TypeScript says extension. The ear extensions, the fact
  enable and words/<name>/<file> are names that each record holds, and the code gives them as literals alone."""
  tracked = subprocess.run(  # noqa: S603
    [shutil.which("git") or "git", "ls-files", "*.rs", "*.py", "*.pyi", "*.ts", "*.cts"],
    cwd=ROOT,
    capture_output=True,
    text=True,
    check=True,
  ).stdout.split()
  left = sorted({f"{file}: {one}" for file in tracked for one in names(ROOT / file) if "extension" in one.lower()})
  assert left == [], "\n".join(left)
