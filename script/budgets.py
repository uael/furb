"""Set the budget of time of each job of the gates from its measures, in `.github/budgets.json`.

The budget of a job is one and a half times its median time in the last green runs of `gates.yml` on main, rounded up
to a whole minute. The round of the conductor compares the median of the last five green runs of each job with its
budget at each sweep, and queues an item when a job passes it. Run it from the root of the repository, on `gh`:

    python script/budgets.py [--runs 60]
"""

import argparse
import json
import math
import shutil
import statistics
import subprocess
import sys
from datetime import datetime
from pathlib import Path


def gh(*args: str) -> object:
  command = [shutil.which("gh") or "gh", "api", *args]
  return json.loads(subprocess.run(command, capture_output=True, text=True, check=True).stdout)  # noqa: S603


def seconds(stamp: str) -> float:
  return datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()


def main() -> None:
  parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
  parser.add_argument("--runs", type=int, default=60, help="how many green runs of main to measure")
  runs = parser.parse_args().runs
  path = "repos/{owner}/{repo}/actions/workflows/gates.yml/runs"
  green = gh(f"{path}?branch=main&status=success&per_page={runs}", "--jq", "[.workflow_runs[].id]")
  assert isinstance(green, list)
  took: dict[str, list[float]] = {}
  for run in green:
    jobs = gh(f"repos/{{owner}}/{{repo}}/actions/runs/{run}/jobs?per_page=100", "--jq", ".jobs")
    assert isinstance(jobs, list)
    for job in jobs:
      if job["conclusion"] == "success":
        took.setdefault(job["name"], []).append(seconds(job["completed_at"]) - seconds(job["started_at"]))
  budgets = {name: 60 * math.ceil(1.5 * statistics.median(times) / 60) for name, times in sorted(took.items())}
  Path(".github/budgets.json").write_text(json.dumps(budgets, indent=2) + "\n")
  for name, times in sorted(took.items()):
    sys.stdout.write(
      f"{name}: median {statistics.median(times):.0f} s of {len(times)} runs, budget {budgets[name]} s\n"
    )


if __name__ == "__main__":
  main()
