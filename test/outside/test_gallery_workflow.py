"""The gallery renders on a dispatch and on an hourly schedule, and never on a push, so it takes no runner from the gates."""

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def workflow() -> dict:
  return yaml.safe_load((ROOT / ".github" / "workflows" / "gallery.yml").read_text(encoding="utf-8"))


def triggers() -> dict:
  loaded = workflow()
  # YAML reads the key `on` as true.
  return loaded.get("on") or loaded[True]


def test_the_gallery_has_a_dispatch_and_no_trigger_of_a_push() -> None:
  assert "workflow_dispatch" in triggers()
  assert not {"push", "pull_request", "pull_request_target", "workflow_run"} & set(triggers())


def test_the_gallery_has_one_schedule_that_looks_each_hour_at_a_fixed_minute() -> None:
  crons = [one["cron"] for one in triggers()["schedule"]]
  assert len(crons) == 1
  assert re.fullmatch(r"\d{1,2} \* \* \* \*", crons[0])


def test_a_newer_request_waits_and_ends_no_render_that_runs() -> None:
  held = workflow()["concurrency"]
  assert held["cancel-in-progress"] is False
  # A schedule runs on main and a dispatch on w/converge, so a group by ref would let two renders run at once.
  assert "github.ref" not in held["group"]


def test_the_render_waits_for_the_check_of_the_head_that_a_schedule_renders_once() -> None:
  jobs = workflow()["jobs"]
  assert "head" in jobs["render"]["needs"]
  assert "needs.head.outputs.render" in jobs["render"]["if"]


def test_the_gallery_of_w_converge_writes_a_branch_that_the_workflow_of_main_never_writes() -> None:
  # The gallery.yml of main renders main to w/gallery, and a proof run of it to w/gallery-proof. A schedule runs the
  # workflow of the default branch, so one branch for both would show main on the pull request of w/converge.
  branch = workflow()["env"]["BRANCH"]
  assert branch.startswith("w/gallery") and branch not in {"w/gallery", "w/gallery-proof"}


def test_each_use_of_the_branch_of_the_gallery_in_the_file_reads_the_one_constant() -> None:
  lines = [
    x
    for x in (ROOT / ".github" / "workflows" / "gallery.yml").read_text(encoding="utf-8").splitlines()
    if not x.lstrip().startswith("#")
  ]
  named = [x for x in lines if "w/gallery" in x]
  assert named == ["  BRANCH: w/gallery-converge", "  RETIRED: w/gallery"]
  assert any('commits/$BRANCH"' in x for x in lines)
