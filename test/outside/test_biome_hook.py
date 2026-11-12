"""The lint of the TypeScript that the gates run, bun run lint, runs in the hooks of each commit first."""

import json
import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def hook() -> dict:
  loaded = yaml.safe_load((ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8"))
  return next(one for repo in loaded["repos"] for one in repo["hooks"] if one["id"] == "biome")


def test_the_hook_of_biome_runs_the_lint_of_the_gates_whole_and_reads_each_path_that_it_checks() -> None:
  lint = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))["scripts"]["lint"]
  assert lint.startswith("biome check ")
  # A warning fails the gate and the hook alike, since both run this one script.
  assert lint.endswith(" --error-on-warnings")
  assert hook()["entry"] == "bun run lint"
  assert hook()["pass_filenames"] is False
  files = re.compile(hook()["files"])
  # One file under each path that the lint names, as a change of it would give to the hook.
  for path in (
    "bind/typescript/src/index.ts",
    "tui/test/children.test.ts",
    "tui/scenarios/owed-message.test.ts",
    "docs/examples/typescript/first.ts",
    ".agents/skills/compaction/tui.ts",
    "package.json",
    "tsconfig.json",
    "biome.json",
  ):
    assert files.search(path), path
  assert not files.search("src/furb/engine.py")


def test_the_job_of_the_hooks_has_no_bun_so_it_skips_the_hook_and_the_job_of_the_bindings_runs_the_lint() -> None:
  jobs = {
    **yaml.safe_load((ROOT / ".github" / "workflows" / "gates.yml").read_text(encoding="utf-8"))["jobs"],
    **yaml.safe_load((ROOT / ".github" / "workflows" / "system.yml").read_text(encoding="utf-8"))["jobs"],
  }
  skipped = [
    step["env"]["SKIP"] for step in jobs["hooks"]["steps"] if "pre-commit run" in step.get("run", "") and "env" in step
  ]
  assert skipped == ["pytest,biome,clippy,cargo-test"]
  runs = [step.get("run", "") for job in jobs.values() for step in job.get("steps", [])]
  assert "bun run lint" in runs
