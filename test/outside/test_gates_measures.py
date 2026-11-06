"""The jobs that measure keep what they measured as an artifact, even when the job fails."""

from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def steps(job: str) -> list[dict]:
  workflow = yaml.safe_load((ROOT / ".github" / "workflows" / "gates.yml").read_text(encoding="utf-8"))
  return workflow["jobs"][job]["steps"]


def published(job: str) -> list[dict]:
  return [one for one in steps(job) if one.get("uses") == "./.github/actions/measures"]


def test_the_bench_writes_its_measures_in_a_file_and_publishes_them_even_when_it_goes_over_a_budget() -> None:
  run = [one for one in steps("bench") if "bun run bench" in one.get("run", "")]
  assert len(run) == 1 and "FURB_BENCH_JSON" in run[0]["env"]
  [kept] = published("bench")
  assert kept["if"] == "always()" and kept["with"] == {"name": "bench"}
  assert steps("bench").index(kept) > steps("bench").index(run[0])


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
