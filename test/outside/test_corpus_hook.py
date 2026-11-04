"""The hook of the corpus refuses a change of each file of a folder that a tag holds."""

import importlib.util
import os
import shutil
import subprocess
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location(
  "corpus", Path(__file__).resolve().parents[2] / "script" / "hooks" / "corpus.py"
)
assert SPEC and SPEC.loader
corpus = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(corpus)


@pytest.fixture(autouse=True)
def alone(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
  """A repository of its own, with a tagged release 0.1.0 and a tagged release 1.0.0, each with one record."""
  for name in [x for x in os.environ if x.startswith("GIT_")]:
    monkeypatch.delenv(name)
  monkeypatch.chdir(tmp_path)
  git("init", "-q", "-b", "main")
  for version in ["0.1.0", "1.0.0"]:
    put(f"tests/corpus/{version}/work.jsonl", "[]\n")
    git("add", "-A")
    git("commit", "-qm", f"Release {version}")
    git("tag", f"v{version}")


def git(*args: str) -> None:
  """Runs a command of git, as a test author."""
  who = ["-c", "user.name=test", "-c", "user.email=test@furb", "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"]
  subprocess.run([shutil.which("git") or "git", *who, *args], check=True, capture_output=True)  # noqa: S603


def put(path: str, text: str) -> None:
  """Writes a file of the repository."""
  Path(path).parent.mkdir(parents=True, exist_ok=True)
  Path(path).write_text(text)


def test_a_change_of_a_file_of_a_tagged_folder_is_refused() -> None:
  put("tests/corpus/1.0.0/work.jsonl", "[1]\n")
  put("tests/corpus/0.1.0/more.jsonl", "[]\n")
  git("add", "-A")
  assert len(corpus.refused()) == 2


def test_a_record_of_next_and_a_folder_that_no_tag_holds_pass() -> None:
  put("tests/corpus/next/work.jsonl", "[2]\n")
  put("tests/corpus/1.1.0/work.jsonl", "[]\n")
  git("add", "-A")
  assert corpus.refused() == []


def test_a_title_with_a_bang_deletes_a_folder_of_0x_and_never_one_of_1x() -> None:
  git("switch", "-qc", "change")
  git("rm", "-rq", "tests/corpus/0.1.0", "tests/corpus/1.0.0")
  git("commit", "-qm", "feat(crate)!: Make the corpus again")
  refused = corpus.refused("main", "feat(crate)!: Make the corpus again")
  assert [x.split(":")[0] for x in refused] == ["tests/corpus/1.0.0/work.jsonl"]
  assert len(corpus.refused("main", "feat(crate): Make the corpus again")) == 2
