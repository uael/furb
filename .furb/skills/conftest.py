"""The conftest of the suites of the extensions of the project: no test sees a variable of git that a commit sets, and a
test runs on the engine that engine_of names, pulling is a Bench that plays pr in a folder with the template, title.py
and the config of release-please of the repository, and laning is a Lane that plays the kit of the lane in a git tree."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

from conftest import engine_of  # noqa: F401
from skills.conftest import Bench
from furb.engine import OPERATOR


@pytest.fixture(autouse=True)
def no_git_variables(monkeypatch: pytest.MonkeyPatch) -> None:
  """A commit through the hook sets GIT_INDEX_FILE and its kin, which each git of a test would inherit."""
  for name in [name for name in os.environ if name.startswith("GIT_")]:
    monkeypatch.delenv(name)


REPO = Path(__file__).resolve().parents[2]


@pytest.fixture
async def pulling(tmp_path: Path) -> Bench:
  (tmp_path / "script").mkdir()
  shutil.copy(REPO / "script" / "title.py", tmp_path / "script")
  shutil.copy(REPO / "release-please-config.json", tmp_path)
  made = Bench(tmp_path, [REPO / ".furb" / "skills" / "pr" / "pr.py"], [[OPERATOR, [], 200000]], OPERATOR)
  made.sand.files[f"{tmp_path}/.github/pull_request_template.md"] = (REPO / ".github" / "pull_request_template.md").read_text(encoding="utf-8")
  await made.load()
  return made


LANE = REPO / ".furb" / "skills" / "lane"
LANE_ROSTER = [
  [OPERATOR, [], 200000],
  ["claude-cli:sonnet", ["high"], 1000000],
  ["claude-cli:opus", ["high"], 1000000],
  ["claude-cli:fable", ["high"], 1000000],
]


def gitted(where: Path, *args: str) -> str:
  """git in a folder of a test, with a name and no signature, so a commit needs no config of the machine."""
  who = ["-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false"]
  return subprocess.run(["git", *who, *args], cwd=where, capture_output=True, text=True, check=True).stdout


class Labeled(dict):
  """The words of the models by the label of the chain of a reply: the World reads them by the id of the chain, and a
  test names a chain by its label before the kit makes it."""

  def __init__(self, engine: object) -> None:
    super().__init__()
    self.engine = engine

  def label(self, on: str) -> str:
    return str(self.engine.get(on)[4])

  def get(self, on: str, default: object = None) -> object:
    return super().get(self.label(on), default)

  def __getitem__(self, on: str) -> list[str]:
    return super().__getitem__(self.label(on))


class Lane(Bench):
  """The Bench of the kit of the lane: its root stands in a git tree whose origin is a bare repository beside it, the
  World gives the word and the contract of the kit at skills://lane, and words gives what the models answer on each
  chain, by its label."""

  def __init__(self, tmp: Path) -> None:
    super().__init__(tmp, [LANE / "lane.py"], LANE_ROSTER, "claude-cli:sonnet/high")
    self.words = self.sand.script = Labeled(self.engine)
    self.origin = tmp.parent / f"{tmp.name}.git"
    gitted(tmp.parent, "init", "-q", "--bare", "-b", "main", str(self.origin))
    gitted(tmp, "init", "-q", "-b", "main")
    for key, value in (("user.name", "t"), ("user.email", "t@example.com"), ("commit.gpgsign", "false")):
      gitted(tmp, "config", key, value)
    gitted(tmp, "commit", "-q", "--allow-empty", "-m", "base")
    gitted(tmp, "remote", "add", "origin", str(self.origin))
    gitted(tmp, "push", "-q", "origin", "main")
    self.kit(self.text)
    self.sand.files[self.sand.placed(self.root, "skills://lane/lane.pyi")] = (LANE / "lane.pyi").read_text(encoding="utf-8")

  def kit(self, word: str) -> None:
    """The word of the kit that the World gives at skills://lane."""
    self.sand.files[self.sand.placed(self.root, "skills://lane")] = word


@pytest.fixture
async def laning(tmp_path: Path) -> Lane:
  made = Lane(tmp_path)
  await made.load("LANE_CHECKS = {}")
  return made
