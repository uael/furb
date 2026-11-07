"""A run of the gates on main and on the pull request of w/converge goes to its end, and a push on another branch ends the run before it."""

import re
from pathlib import Path
from types import SimpleNamespace

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]


def cancels(ref: str, head_ref: str = "") -> bool:
  """The cancel-in-progress of gates.yml for a run with that ref and that head_ref (a pull request names its branch
  in head_ref, and its ref is refs/pull/<n>/merge), read as GitHub reads the expression."""
  workflow = yaml.safe_load((ROOT / ".github" / "workflows" / "gates.yml").read_text(encoding="utf-8"))
  expression = workflow["concurrency"]["cancel-in-progress"]
  found = re.fullmatch(r"\$\{\{ (.*) \}\}", expression)
  assert found, expression
  body = found[1].replace("&&", " and ").replace("||", " or ")
  body = re.sub(r"!(?!=)", " not ", body)
  return eval(body, {"github": SimpleNamespace(ref=ref, head_ref=head_ref)})  # noqa: S307


@pytest.mark.parametrize(("ref", "head_ref"), [("refs/heads/main", ""), ("refs/pull/1389/merge", "w/converge")])
def test_a_push_on_main_or_on_the_pull_request_of_w_converge_leaves_the_run_in_progress(
  ref: str, head_ref: str
) -> None:
  assert not cancels(ref, head_ref)


@pytest.mark.parametrize(
  ("ref", "head_ref"),
  [("refs/heads/c/ci", ""), ("refs/pull/1390/merge", "w/gallery-proof"), ("refs/pull/1385/merge", "w/ask-a151-1")],
)
def test_a_push_on_another_branch_ends_the_run_in_progress(ref: str, head_ref: str) -> None:
  assert cancels(ref, head_ref)
