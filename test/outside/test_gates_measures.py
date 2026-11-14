"""The jobs that measure keep what they measured as an artifact, even when the job fails."""

from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def steps(job: str) -> list[dict]:
  """The steps of a job of the gates, or of the workflow system that the gates call for each system."""
  jobs = {}
  for name in ("gates.yml", "system.yml"):
    jobs |= yaml.safe_load((ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8"))["jobs"]
  return jobs[job]["steps"]


def published(job: str) -> list[dict]:
  return [one for one in steps(job) if one.get("uses") == "./.github/actions/measures"]


def test_the_suite_on_linux_publishes_the_output_of_the_script_of_the_tokens_even_when_a_test_fails() -> None:
  [run] = [one for one in steps("suite") if "script/tokens.py" in one.get("run", "")]
  [kept] = published("suite")
  assert "Linux" in run["if"] and "always()" in run["if"]
  assert "Linux" in kept["if"] and "always()" in kept["if"] and kept["with"] == {"name": "tokens"}


def test_the_action_of_the_measures_names_the_head_and_the_run_beside_the_folder_that_it_uploads() -> None:
  action = yaml.safe_load((ROOT / ".github" / "actions" / "measures" / "action.yml").read_text(encoding="utf-8"))
  meta, upload = action["runs"]["steps"]
  assert "github.event.pull_request.head.sha" in meta["env"]["HEAD"]
  assert all(word in meta["run"] for word in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "meta.json"))
  assert upload["with"]["path"] == "${{ runner.temp }}/measures"
