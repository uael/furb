# ruff: noqa: S603
"""A record that a later build of a major made opens on the release before it, as tests/corpus/README.md says."""

import re
import shutil
import subprocess
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "tests" / "corpus"


def build() -> str:
  return tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))["project"]["version"]


def chosen(version: str, folders: list[str]) -> str | None:
  """The last release of the major of `version` that holds a folder of the corpus, or None.

  The promise binds only within a major, so a release of another major is no build to open `next` on.
  """
  major = version.split(".", maxsplit=1)[0]
  same = [name for name in folders if name.split(".", maxsplit=1)[0] == major]
  return max(same, key=lambda name: tuple(map(int, name.split(".")))) if same else None


def refusals(version: str, folder: Path, tmp: Path) -> dict[str, str]:
  """What the program furb of a release says on each record of a folder that it does not open, by the record."""
  said = {}
  for record in sorted(folder.glob("*.jsonl")):
    copy = tmp / record.name
    shutil.copy(record, copy)
    command = ["uvx", "--quiet", "--from", f"furb-cli=={version}", "furb", "turns", "--record", str(copy)]
    got = subprocess.run(command, cwd=tmp, capture_output=True, text=True, timeout=600, check=False)
    if got.returncode != 0:
      said[record.name] = got.stderr.strip()[-400:]
  return said


@pytest.mark.parametrize(
  ("version", "folders", "expected"),
  [
    ("1.0.0", ["0.4.2"], None),
    ("1.2.0", ["0.4.2", "1.0.0", "1.1.0", "2.0.0"], "1.1.0"),
    ("0.4.2", ["0.4.2", "0.10.0"], "0.10.0"),
    ("0.4.2", [], None),
  ],
)
def test_next_opens_only_on_a_release_of_its_own_major(version: str, folders: list[str], expected: str | None) -> None:
  assert chosen(version, folders) == expected


def test_each_record_of_next_opens_on_the_last_release_of_its_major_once_that_release_holds_a_corpus(
  tmp_path: Path,
) -> None:
  folders = [p.name for p in CORPUS.iterdir() if p.is_dir() and re.fullmatch(r"\d+\.\d+\.\d+", p.name)]
  version = chosen(build(), folders)
  # The release flow copies next into the folder of its version, so a major with no folder came before the corpus,
  # and its build reads no record of the form of next.
  if version is None:
    pytest.skip(
      f"no release of the major of {build()} holds a folder of the corpus, so no record of next is of its form"
    )
  assert shutil.which("uvx"), "uvx installs the release from its package"
  assert refusals(version, CORPUS / "next", tmp_path) == {}
