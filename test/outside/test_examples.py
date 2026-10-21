"""The examples of the framework in docs/examples, each run as a user runs it, as the claims it shows."""

import os
import subprocess
import sys
from pathlib import Path

import pytest

EXAMPLE = Path(__file__).parents[2] / "docs" / "examples" / "python.py"


@pytest.mark.parametrize("engine", ["python", "monty"])
def test_the_example_of_python_holds_each_claim_it_shows_on_each_engine(engine: str) -> None:
  """The example of python runs as it stands, on the engine of this interpreter and on the engine of monty, and every
  claim it asserts holds: a life that resumes after its process ends, a tree of chains, a prompt to the operator, a
  grant and a door."""
  env = {**os.environ, "FURB_ENGINE": engine}
  ran = subprocess.run(  # noqa: S603
    [sys.executable, str(EXAMPLE)], env=env, capture_output=True, text=True, timeout=120, check=False
  )
  assert ran.returncode == 0, ran.stderr
  assert ran.stdout.splitlines()[-1] == "the second process resumed it from the record, and every claim holds"
