import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def test_each_job_that_publishes_waits_for_the_proofs_and_each_package_ships_the_page() -> None:
  jobs = yaml.safe_load((ROOT / ".github" / "workflows" / "publish.yml").read_text(encoding="utf-8"))["jobs"]
  assert any("cargo xtask promise" in step.get("run", "") for step in jobs["promise"]["steps"])
  for name, job in jobs.items():
    if any("publish" in step.get("run", "") or "publish" in str(step.get("uses", "")) for step in job["steps"]):
      assert "promise" in job["needs"], name
  cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
  assert "/docs/promise.md" in cargo["package"]["include"]
  python = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
  assert "docs/promise.md" in python["tool"]["uv"]["build-backend"]["source-include"]
