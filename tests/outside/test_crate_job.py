"""The job of the crate runs the archive of the tests with cargo-nextest, and fails when its selection hides a suite."""

import re
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def step() -> dict:
  """The step of the job of the crate that runs the archive of the tests."""
  jobs = yaml.safe_load((ROOT / ".github" / "workflows" / "system.yml").read_text(encoding="utf-8"))["jobs"]
  [run] = [one for one in jobs["crate"]["steps"] if "nextest run --archive-file" in one.get("run", "")]
  return run


def members() -> set[str]:
  """The name of each package of the workspace."""
  manifests = [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]
  return {tomllib.loads(one.read_text(encoding="utf-8")).get("package", {}).get("name", "") for one in manifests}


def test_on_macos_the_tests_of_the_crate_and_of_the_window_run_and_the_kills_of_the_command_line_alone() -> None:
  only = step()["env"]["ONLY"]
  assert only == (
    "${{ runner.os == 'macOS' && 'package(furb) | package(furb-window) | (package(furb-cli) & test(/^kills::/))'"
    " || 'all()' }}"
  )


def test_each_package_that_the_selection_names_stands_in_the_workspace() -> None:
  # The folders before the move of the crates named packages by their folder, which the selection knows no more.
  named = set(re.findall(r"package\(([^)]+)\)", step()["env"]["ONLY"]))
  assert named == {"furb", "furb-window", "furb-cli"}
  assert named <= members()


def test_a_filter_that_selects_no_test_fails_the_job() -> None:
  run = step()["run"]
  assert "--no-tests fail" in run and '-E "$ONLY"' in run
