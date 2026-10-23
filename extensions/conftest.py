"""The conftest of the suites of the extensions: the hooks and the fixtures of the harness of the engine, which reach
the suite of each extension only through a conftest above it, and the helpers that only those suites need."""

import asyncio
import sys
import tempfile
from collections.abc import Callable, Generator, Sequence
from dataclasses import replace
from pathlib import Path

import pytest

from conftest import STANDS, Sand, World, engine_of, life, of, pytest_generate_tests, sown, world_says  # noqa: F401
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty

EXTENDING = (
  "extending: How to extend furb: a def in a word, which lives as long as its chain, and an extension, which a host"
  " enables on each chain of a life."
)
"""EXTENDING is the line that the list of the skills holds of the skill that the extension carries."""


SKILLED = (".furb/skills", ".agents/skills", ".claude/skills")
"""SKILLED is the folders of skills that the World finds in the working directory of a chain and in each folder above
it."""


def stray(folder: Path) -> bool:
  """Whether a folder above this one holds a folder of skills, whose skills the World finds for a chain that stands
  under it."""
  return any((one / place).is_dir() for one in folder.parents for place in SKILLED)


TOP = Path(tempfile.gettempdir()).anchor if sys.platform == "win32" else "/tmp"  # noqa: S108
"""TOP is the folder of the system that a test takes its temporary folder in when the one of pytest lies under skills:
/tmp, and on Windows, which has no /tmp, the root of the drive of the temporary folder."""


@pytest.fixture
def tmp_path(tmp_path: Path) -> Generator[Path]:
  """The temporary folder of a test, under no folder that holds skills: the one of pytest, or else a new one in TOP,
  as when the temporary folder of the system lies under a home that holds skills. So a test sees only the skills that
  it gives, and none of the machine."""
  if not stray(tmp_path):
    yield tmp_path
    return
  with tempfile.TemporaryDirectory(dir=TOP) as made:
    one = Path(made).resolve()
    assert not stray(one), f"a folder above {one} holds skills"
    yield one


def extended(  # noqa: PLR0913
  name: str,
  at: Path,
  ear: Callable[[str], World] | None = None,
  *,
  lives: bool = False,
  record: Sequence[tuple] = (),
  sand: Sand | None = None,
) -> tuple[Sand, str]:
  """A life on a record, whose chains stand in the folder work of a directory, which enables at its tip the official
  extension of that name, with its life word when it lives: it hears the ear of the extension when it has one, whose
  config directory of the user is the folder config of the directory, and the files of the machine after the World
  of the suite, or after the World it is given. It gives the World and the root."""
  one = next(x for x in _monty.official() if x["name"] == name)
  (at / "work").mkdir(exist_ok=True)
  given = _monty.extensions([one if lives else {**one, "life": ""}])
  sand = replace(sand or sown(), stands=[STANDS[0], str(at / "work"), STANDS[2]])
  files = _monty.files(str(at / "scratch"))
  return sand, life(sand, record, extensions=given, **({name: ear(str(at / "config"))} if ear else {}), files=files)[1]


def noted(folder: Path, text: str, name: str = "AGENTS.md") -> Path:
  """A memory file in a folder, which it makes with the folders above it, and the path of that file."""
  folder.mkdir(parents=True, exist_ok=True)
  (path := folder / name).write_text(text, encoding="utf-8", newline="")
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
  (path := skills / folder / "SKILL.md").write_text(f"---\n{head}\n---\nSteps.\n", encoding="utf-8", newline="")
  return path


FULL = (300000, 0, 0, 0, 0.0)
"""An answer that reads three quarters of the window of the default actor of the suite, which is 400000 tokens."""


def compacting(at: Path) -> tuple[Sand, str, str]:
  """A life that runs the extension compaction, whose folder work holds a.txt: the World, the root and the folder."""
  sand, root = extended("compaction", at)
  work = str(at / "work")
  sand.files[f"{work}/a.txt"] = "one\ntwo\n"
  return sand, root, work


def surfed(answers: dict[str, Text], heard: list[tuple]) -> World:
  """The web of the suite, in memory: an ear that takes each web_search and each web_fetch, keeps it in heard, says it
  started, and says it done from its own loop with the answer of its query or its address, or with a refusal when it
  has none."""
  while True:
    a = yield
    if a[0] in ("web_search", "web_fetch") and engine.question(a):
      heard.append(a)
      yield "started", a[1]
      got = answers.get(a[4], Refused(f"No page answers {a[4]}."))
      asyncio.get_running_loop().call_soon(world_says, "done", a[1], got)


def webbed(at: Path, answers: dict[str, Text]) -> tuple[str, list[tuple]]:
  """A life that runs the extension web, on the web of the suite with these answers: the root, and what the web
  heard."""
  heard: list[tuple] = []
  return extended("web", at, lambda _: surfed(answers, heard))[1], heard
