"""lane_job, the job that the watch of the root keeps for a lane."""

from skills.conftest import Bench


async def test_lane_job_keeps_the_job_of_a_label(laning: Bench) -> None:
  """lane_job(label, job, started, patience) keeps job as the job of label, and gives it."""
  await laning.word('job = prompt(str, "wait", to=OPERATOR)\ngot = lane_job("a", job, clock(), 60.0)')
  job = laning.ns["job"]
  assert laning.ns["got"] == job
  assert laning.ns["lane_jobs"] == {"a": (job, 1000.0, 60.0)}
