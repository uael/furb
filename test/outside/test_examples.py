"""The examples of python in docs/examples/python, each run as a user runs it, on a claude command line that answers
from a script, so no test asks a model."""

import json
import os
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import pytest

ROOT = Path(__file__).parents[2]
EXAMPLES = ROOT / "docs" / "examples" / "python"
FAKE = ROOT / "src" / "world" / "provider" / "fake-claude.sh"

pytestmark = pytest.mark.skipif(
  sys.platform == "win32", reason="the claude of the tests is a script of sh, which Windows does not start"
)


@dataclass
class Yard:
  """The directory an example runs in, and the fake claude beside it."""

  at: Path
  fake: Path

  def words(self, *words: str) -> None:
    """The words the fake answers with, in order."""
    for at, word in enumerate(words, 1):
      (self.fake / f"word.{at}").write_text(json.dumps(word))

  def env(self) -> dict[str, str]:
    """The environment of an example: the fake claude, and the config and the cache of the yard."""
    env = {key: value for key, value in os.environ.items() if not key.startswith("FURB_")}
    config, cache = self.fake.with_name("config"), self.fake.with_name("cache")
    return {
      **env,
      "FURB_CLAUDE_BIN": str(self.fake / "claude"),
      "FURB_CONFIG_DIR": str(config),
      "XDG_CACHE_HOME": str(cache),
    }

  def command(self, name: str) -> list[str]:
    return [sys.executable, str(EXAMPLES / f"{name}.py")]

  def run(self, name: str, stdin: str = "") -> str:
    """What the example printed, once it ended well."""
    ran = subprocess.run(  # noqa: S603
      self.command(name),
      cwd=self.at,
      env=self.env(),
      input=stdin,
      capture_output=True,
      text=True,
      timeout=60,
      check=False,
    )
    assert ran.returncode == 0, ran.stderr
    return ran.stdout

  def facts(self, record: str) -> list[list]:
    """The facts of a record of the yard."""
    return [json.loads(line)[0] for line in (self.at / record).read_text().splitlines()]


@pytest.fixture
def yard(tmp_path: Path) -> Yard:
  root = tmp_path.resolve()
  at, fake = root / "work", root / "fake"
  at.mkdir()
  fake.mkdir()
  shutil.copy(FAKE, fake / "claude")
  (fake / "claude").chmod(0o755)
  return Yard(at, fake)


def test_a_life_goes_on_in_a_second_process_after_its_first_process_ends(yard: Yard) -> None:
  (yard.fake / "deaf").touch()
  first = subprocess.Popen(yard.command("durable"), cwd=yard.at, env=yard.env())  # noqa: S603
  while not (yard.fake / "pids").exists():
    assert first.poll() is None
    time.sleep(0.05)
  first.kill()
  first.wait()
  (yard.fake / "deaf").unlink()
  yard.words('close("Ship version 2 on Friday.")')
  assert yard.run("durable") == "Ship version 2 on Friday.\n"
  asked = [one for one in yard.facts("release.jsonl") if one[0] == "prompt" and one[2] == "operator"]
  assert [one[1] for one in asked] == ["prompt1"]


def test_a_reviewer_on_a_chain_of_its_own_reviews_the_draft_of_the_root(yard: Yard) -> None:
  yard.words('close("Login is twice as fast.")', 'close("The notes read well.")')
  assert yard.run("chains") == "The notes read well.\nreviewer under root\n"


def test_the_model_asks_the_operator_before_it_deploys(yard: Yard) -> None:
  yard.words('go = await prompt(bool, "Deploy version 2 now?", to=OPERATOR)\nclose("deployed" if go else "held")')
  assert yard.run("questions", "yes\n") == "Deploy version 2 now? deployed\n"


def test_a_grant_pauses_the_chain_at_its_ceiling_and_a_larger_grant_lets_it_go_on(yard: Yard) -> None:
  (yard.fake / "cost").write_text("0.3")
  yard.words("files = 3", "files = 3", "close(1200)")
  assert yard.run("grant") == "paused at 0.60 dollars\n1200\n"


def test_an_edit_of_the_door_of_a_thread_changes_the_plan_of_the_model(yard: Yard) -> None:
  yard.words('await wait(3600)\nclose("released")', "await wait(3600)")
  assert yard.run("door") == 'await wait(3600)\nclose("released")\nreleased\n'
