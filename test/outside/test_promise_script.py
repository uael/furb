# ruff: noqa: S603
import importlib.util
import re
import subprocess
import sys
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "script" / "promise.py"
SPEC = importlib.util.spec_from_file_location("promise", SCRIPT)
assert SPEC and SPEC.loader
promise = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(promise)


def test_the_page_of_the_promise_names_its_proofs_of_python_and_of_rust_and_each_goes_to_its_target() -> None:
  found = promise.proofs((ROOT / "docs" / "promise.md").read_text(encoding="utf-8"))
  assert ("tests/promise.rs", "each_proof_that_the_page_of_the_promise_names_stands_in_its_file") in found
  # A test that the page names as no proof yet runs in no gate of a release.
  assert all("rollback" not in file for file, _ in found)
  commands = [command for command, _, _ in promise.runs(found)]
  assert any("pytest" in command for command in commands)
  assert ["cargo", "test", "-p", "furb-host", "--test", "daemon"] in [one[:6] for one in commands]
  assert ["cargo", "test", "-p", "furb", "--lib"] in [one[:5] for one in commands]


def page(tmp: Path, body: str) -> Path:
  # A pytest.ini roots the proof at its folder: with none, pytest roots it at the common folder of it and the repo, the
  # home on a desk, and walks that tree for seconds before it runs one test.
  (tmp / "pytest.ini").write_text("[pytest]\n", encoding="utf-8")
  (tmp / "test_proof.py").write_text(body, encoding="utf-8")
  made = tmp / "promise.md"
  made.write_text(
    f"## The proofs that a machine runs (R4)\n\n- `{tmp / 'test_proof.py'}::test_holds`\n\nWhat no machine proves yet\n"
  )
  return made


def run(made: Path) -> subprocess.CompletedProcess[str]:
  return subprocess.run(
    [sys.executable, str(SCRIPT), str(made)], capture_output=True, text=True, timeout=120, check=False
  )


def test_a_proof_that_fails_refuses_the_release_and_is_named(tmp_path: Path) -> None:
  got = run(page(tmp_path, "def test_holds():\n  assert False\n"))
  assert got.returncode == 1
  assert f"promise: {tmp_path / 'test_proof.py'}::test_holds did not pass" in got.stdout


def test_the_same_proof_that_passes_lets_the_release_go(tmp_path: Path) -> None:
  got = run(page(tmp_path, "def test_holds():\n  pass\n"))
  assert got.returncode == 0, got.stdout
  assert "promise: 1 of 1 proofs passed" in got.stdout


def test_each_job_that_publishes_waits_for_the_proofs_and_each_package_ships_the_page() -> None:
  jobs = yaml.safe_load((ROOT / ".github" / "workflows" / "publish.yml").read_text(encoding="utf-8"))["jobs"]
  assert any("script/promise.py" in step.get("run", "") for step in jobs["promise"]["steps"])
  for name, job in jobs.items():
    if any("publish" in step.get("run", "") or "publish" in str(step.get("uses", "")) for step in job["steps"]):
      assert "promise" in job["needs"], name
  cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
  assert "/docs/promise.md" in cargo["package"]["include"]
  python = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
  assert "docs/promise.md" in python["tool"]["uv"]["build-backend"]["source-include"]


def test_a_page_that_names_a_proof_by_a_path_of_windows_gives_it_and_its_line_of_pytest_matches() -> None:
  page = "## The proofs that a machine runs (R4)\n\n- `C:\\Users\\x\\test_proof.py::test_holds`\n\nWhat no machine proves yet\n"
  found = promise.proofs(page)
  assert found == [("C:\\Users\\x\\test_proof.py", "test_holds")]
  _, patterns, _ = promise.runs(found)[0]
  assert re.search(patterns[0], "PASSED C:\\Users\\x\\test_proof.py::test_holds", re.MULTILINE)
  assert re.search(patterns[0], "PASSED ../x/test_proof.py::test_holds [case]", re.MULTILINE)
  # A file on another drive than the root of pytest has no path from it, so pytest names the proof by its test alone.
  assert re.search(patterns[0], "PASSED ::test_holds", re.MULTILINE)
  assert not re.search(patterns[0], "PASSED other/test_other.py::test_holds", re.MULTILINE)
