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
def test_the_hooks_of_the_suite_run_only_on_the_files_that_it_reads(hook: str, path: str, *, runs: bool) -> None:
  assert bool(re.search(hooks()[hook]["files"], path)) is runs


@pytest.mark.parametrize(
  ("path", "runs"),
  [
    ("src/engine.rs", True),
    ("src/world/files.rs", True),
    ("build/verbs.rs", True),
    ("Cargo.toml", True),
    ("rust-toolchain.toml", True),
    ("cli/src/main.rs", False),
    ("host/src/daemon.rs", False),
  ],
)
def test_the_hook_of_the_suite_runs_on_the_rust_that_the_crate_builds_for_it(path: str, *, runs: bool) -> None:
  hook = hooks()["pytest"]
  assert bool(re.search(hook["files"], path)) is runs
  assert ("rust" in hook["types_or"]) is True
