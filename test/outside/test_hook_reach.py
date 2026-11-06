"""The hooks of the tests run what the files of a commit reach, and the gates, which give every file, run the whole set."""

import importlib.util
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("reach", ROOT / "script" / "hooks" / "reach.py")
assert SPEC and SPEC.loader
reach = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(reach)


def hooks() -> dict[str, dict]:
  loaded = yaml.safe_load((ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8"))
  return {one["id"]: one for repo in loaded["repos"] for one in repo["hooks"]}


def test_the_hooks_of_the_tests_give_all_the_files_of_the_commit_to_the_reach_in_one_call() -> None:
  for name, kind in (("pytest", "pytest"), ("cargo-test", "cargo")):
    hook = hooks()[name]
    assert hook["entry"] == f"python3 script/hooks/reach.py {kind}"
    assert hook.get("pass_filenames", True) and hook["require_serial"]


def test_a_file_of_the_engine_of_the_crate_or_of_a_manifest_reaches_the_whole_suite() -> None:
  for path in ("src/furb/engine.py", "src/world/files.rs", "pyproject.toml", "Cargo.lock", "test/conftest.py"):
    assert reach.suite([path]) is None


def test_a_script_reaches_the_tests_that_name_it_and_no_other() -> None:
  assert "test/outside/test_build_slot_script.py" in reach.suite(["script/build-slot.sh"])
  assert "test/test_boot.py" not in reach.suite(["script/build-slot.sh"])
  assert "test/outside/test_scope.py" in reach.suite(["script/scope.py"])


def test_a_file_of_an_extension_reaches_its_tests_and_the_hygiene_laws() -> None:
  assert reach.suite(["extensions/memory/memory.py"]) == ["extensions/memory/test", "test/test_hygiene.py"]


def test_a_file_of_a_crate_reaches_that_crate_and_a_manifest_the_workspace() -> None:
  assert reach.crates(["host/src/daemon.rs"]) == ["furb-host"]
  assert reach.crates(["src/engine.rs", "cli/src/main.rs"]) == ["furb", "furb-cli"]
  assert reach.crates(["src/engine.rs", "Cargo.toml"]) is None


def test_every_file_of_the_tree_reaches_the_whole_set_as_the_gates_give_it() -> None:
  every = ["script/scope.py", "src/furb/engine.py", "Cargo.toml", "host/src/daemon.rs"]
  assert reach.suite(every) is None and reach.crates(every) is None
