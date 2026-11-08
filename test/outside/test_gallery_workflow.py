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
