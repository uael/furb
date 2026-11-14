"""lane_overdue, the jobs past their time."""

from skills.conftest import Bench


async def test_lane_overdue_names_each_job_past_its_patience(laning: Bench) -> None:
  """lane_overdue() names each job that is open past its patience by the clock, with its minutes, and says when it is paused."""
  await laning.word('job = prompt(str, "wait", to=OPERATOR)\nlane_job("a", job, clock(), 60.0)\nlane_job("b", job, clock(), 600.0)')
  job = laning.ns["job"]
  laning.sand.now = 1200.0
  assert await laning.value("lane_overdue()") == [f"a: {job} is open for 3 minutes, past its 1"]
  await laning.word("pause(job)")
  assert await laning.value("lane_overdue()") == [f"a: {job} is open for 3 minutes, past its 1 (paused)"]
