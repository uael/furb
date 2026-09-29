"""The conftest of the suites of the extensions: the hooks and the fixtures of the harness of the engine, which reach
the suite of each extension only through a conftest above it, and the helpers that only those suites need."""

from collections.abc import Callable, Sequence
from pathlib import Path

from conftest import STANDS, Sand, World, engine_of, life, of, pytest_generate_tests, sown  # noqa: F401
from furb import engine
from furb_monty import _monty


def extended(
  name: str, at: Path, ear: Callable[[str], World], *, lives: bool = False, record: Sequence[tuple] = ()
) -> tuple[Sand, str]:
  """A life on a record, whose chains stand in the folder work of a directory, which enables at its tip the official
  extension of that name, with its life word when it lives: it hears the ear of the extension, whose config directory
  of the user is the folder config of the directory, and the files of the machine after the World of the suite. It
  gives the World and the root."""
  one = next(x for x in _monty.official() if x["name"] == name)
  (at / "work").mkdir(exist_ok=True)
  given = _monty.extensions([one if lives else {**one, "life": ""}])
  sand = sown(stands=[STANDS[0], str(at / "work"), STANDS[2]])
  return sand, life(sand, record, extensions=given, **{name: ear(str(at / "config"))}, files=_monty.files())[1]


def noted(folder: Path, text: str, name: str = "CLAUDE.md") -> Path:
  """A memory file in a folder, which it makes with the folders above it, and the path of that file."""
  folder.mkdir(parents=True, exist_ok=True)
  (path := folder / name).write_text(text, encoding="utf-8")
  return path


def recalled(chain: str, at: Path) -> list[str]:
  """Every paragraph of memory that a chain was told of a file under a directory, in order, since a folder above
  that directory belongs to the machine."""
  return [one for one in of(engine.turns(on=chain), "memory") if f"_path = {str(at)!r}"[:-1] in one.split("\n")[1]]


def memorized(question: str, path: Path, text: str) -> str:
  """The paragraph that a memory question tells of one memory file: its header, the binding of the path of the file,
  and the binding of its text, as a quote when it holds more than one line."""
  lines = text.splitlines()
  name = question + "_text"
  told = f"<s:{name}>\n{'\n'.join(lines)}</s:{name}>" if len(lines) > 1 else f"{name} = {lines[0]!r}"
  return f"#{question}\n{question}_path = {str(path)!r}\n{told}"


def skilled(skills: Path, folder: str, head: str) -> Path:
  """A skill in a folder of skills, whose SKILL.md file opens with a frontmatter of these lines, and the path of that
  file."""
  (skills / folder).mkdir(parents=True, exist_ok=True)
  (path := skills / folder / "SKILL.md").write_text(f"---\n{head}\n---\nSteps.\n", encoding="utf-8")
  return path
