import re
from pathlib import Path

import pytest
import yaml

CONFIG = Path(__file__).resolve().parents[2] / ".pre-commit-config.yaml"


def hooks() -> dict[str, dict]:
  loaded = yaml.safe_load(CONFIG.read_text(encoding="utf-8"))
  return {one["id"]: one for repo in loaded["repos"] for one in repo["hooks"]}


@pytest.mark.parametrize("hook", ["pytest", "ty"])
@pytest.mark.parametrize(
  ("path", "runs"),
  [
    ("src/furb/engine.py", True),
    ("test/test_peek.py", True),
    (".furb/extensions/conductor/conductor.py", True),
    ("pyproject.toml", True),
    ("tui/package.json", False),
    ("tui/src/session.ts", False),
    ("docs/models.md", False),
  ],
)
def test_the_hooks_of_the_suite_run_only_on_the_files_that_it_reads(hook: str, path: str, runs: bool) -> None:
  assert bool(re.search(hooks()[hook]["files"], path)) is runs
