# ruff: noqa: S603
"""A record that a later build of a major made opens on the release before it, as tests/corpus/README.md says."""

import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "tests" / "corpus"


def released() -> str:
  return json.loads((ROOT / ".release-please-manifest.json").read_text(encoding="utf-8"))["."]


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


def test_each_record_of_next_opens_on_the_last_release_once_that_release_holds_a_corpus(tmp_path: Path) -> None:
  version = released()
  # The release flow copies next into the folder of its version, so a release with no folder came before the corpus,
  # and its build reads no record of the form of next.
  if not (CORPUS / version).is_dir():
    pytest.skip(f"the release {version} holds no folder of the corpus, so no record of next is of its form")
  assert shutil.which("uvx"), "uvx installs the release from its package"
  assert refusals(version, CORPUS / "next", tmp_path) == {}
