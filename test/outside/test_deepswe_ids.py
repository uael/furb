"""The names of baseline pytest cases remain stable when a model adds parameters."""

import os
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

PLUGIN = Path(__file__).parents[2] / "script"
TEST = """import pytest
from settings import DEFAULTS, validate

@pytest.mark.parametrize("key, value", list(DEFAULTS.items()))
def test_options(key, value):
    for wrong in (str, int, bool, list):
        if wrong is not type(value):
            with pytest.raises(TypeError):
                validate({key: wrong()})
"""
SOURCE = """DEFAULTS = {defaults}

def validate(values):
    for key, value in values.items():
        if type(value) is not type(DEFAULTS[key]):
            raise TypeError(key)
"""


def run_cases(directory: Path, mode: str, name: str) -> dict[str, bool]:
  report = directory / f"{name}.xml"
  env = {
    **os.environ,
    "PYTHONPATH": str(PLUGIN),
    "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
    "PYTEST_PLUGINS": "pytest_ids",
    "FURB_PYTEST_CASES": str(directory / "cases.json"),
    "FURB_PYTEST_CASE_MODE": mode,
  }
  subprocess.run(  # noqa: S603
    [sys.executable, "-m", "pytest", "-q", "--junitxml", str(report), "test_cases.py"],
    cwd=directory,
    env=env,
    capture_output=True,
    check=False,
  )
  return {
    case.attrib["name"]: not any(child.tag in {"failure", "error", "skipped"} for child in case)
    for case in ET.parse(report).iter("testcase")  # noqa: S314
  }


def test_parameter_positions_do_not_change_existing_cases_or_hide_failures(tmp_path: Path):
  (tmp_path / "test_cases.py").write_text(TEST)
  (tmp_path / "settings.py").write_text(
    SOURCE.format(defaults=repr({"config": "project.toml", "paths": [], "verbose": False}))
  )
  reference = run_cases(tmp_path, "capture", "reference")
  assert reference == {
    "test_options[config-project.toml]": True,
    "test_options[paths-value1]": True,
    "test_options[verbose-False]": True,
  }
  changed = SOURCE.format(defaults=repr({"cache": False, "config": "project.toml", "paths": [], "verbose": False}))
  (tmp_path / "settings.py").write_text(changed)
  assert run_cases(tmp_path, "apply", "added") == {**reference, "test_options[cache-False]": True}
  (tmp_path / "settings.py").write_text(
    changed.replace(
      "for key, value in values.items():",
      "for key, value in values.items():\n        if key == 'paths':\n            continue",
    )
  )
  assert run_cases(tmp_path, "apply", "broken")["test_options[paths-value1]"] is False


def test_changed_test_definitions_do_not_take_reference_names(tmp_path: Path):
  (tmp_path / "test_cases.py").write_text(TEST)
  (tmp_path / "settings.py").write_text(SOURCE.format(defaults=repr({"paths": [], "verbose": False})))
  assert run_cases(tmp_path, "capture", "reference") == {
    "test_options[paths-value0]": True,
    "test_options[verbose-False]": True,
  }
  (tmp_path / "settings.py").write_text(SOURCE.format(defaults=repr({"cache": False, "paths": [], "verbose": False})))
  (tmp_path / "test_cases.py").write_text(TEST.replace("validate({key: wrong()})", "raise TypeError(key)"))
  changed = run_cases(tmp_path, "apply", "changed")
  assert "test_options[paths-value0]" not in changed
  assert changed["test_options[paths-value1]"] is True


def test_duplicate_parameter_values_keep_their_distinct_pytest_names(tmp_path: Path):
  source = TEST.replace("from settings import DEFAULTS, validate", "from settings import DEFAULTS, validate, CASES")
  (tmp_path / "test_cases.py").write_text(source.replace("list(DEFAULTS.items())", "CASES"))
  (tmp_path / "settings.py").write_text(
    SOURCE.format(defaults=repr({"paths": []})) + '\nCASES = [("paths", []), ("paths", [])]\n'
  )
  assert run_cases(tmp_path, "capture", "reference") == {
    "test_options[paths-value0]": True,
    "test_options[paths-value1]": True,
  }
  (tmp_path / "settings.py").write_text(
    SOURCE.format(defaults=repr({"cache": False, "paths": []}))
    + '\nCASES = [("cache", False), ("paths", []), ("paths", [])]\n'
  )
  assert run_cases(tmp_path, "apply", "duplicates") == {
    "test_options[cache-False]": True,
    "test_options[paths-value1]": True,
    "test_options[paths-value2]": True,
  }
