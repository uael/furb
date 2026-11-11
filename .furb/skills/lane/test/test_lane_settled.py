"""lane_settled, the wait of a lane on a job of a model."""

import asyncio
import re

from conftest import said
from skills.conftest import Bench
from furb.engine import Refused

SONNET = "claude-cli:sonnet/high"


def notes(laning: Bench, job: str) -> list[str]:
  """The messages that went to the thread of job."""
  return [str(x[5]) for x in said(laning.log, "prompt") if x[3] == job and x[4] == "None"]


async def test_lane_settled_gives_the_value_of_the_job_or_a_refused_once_seconds_pass(laning: Bench) -> None:
  """lane_settled(job, seconds) awaits job in looks of LANE_LOOK seconds, and gives its value once it is over, or a Refused once seconds pass."""
  laning.words["item"] = ['close("done")']
  await laning.word(f'LANE_LOOK = 0.02\nc = chain("item")\njob = prompt(str, "do", to="{SONNET}", on=c)')
  assert await laning.value("await lane_settled(job, 5.0)") == "done"
  got = await laning.value('await lane_settled(prompt(str, "do", to=OPERATOR, on=c), 0.05)')
  assert isinstance(got, Refused)
  assert re.fullmatch(r"prompt\d+ did not close in 0.05 s", str(got))


async def test_lane_settled_adds_the_nudge_to_a_thread_that_stands_quiet(laning: Bench) -> None:
  """After a look with no new reply, it adds LANE_NUDGE to the thread of job when the chain of job is not paused and holds no open reply, run, command or wait, since the engine asks a thread no more after a word that closes nothing and leaves no act open."""
  laning.words["item"] = ["x = 1", 'close("went on")']
  await laning.word(f'LANE_LOOK = 0.02\nc = chain("item")\njob = prompt(str, "do", to="{SONNET}", on=c)')
  assert await laning.value("await lane_settled(job, 5.0)") == "went on"
  assert notes(laning, laning.ns["job"]) == [laning.ns["LANE_NUDGE"]]


async def test_lane_settled_cancels_a_reply_that_does_not_end(laning: Bench) -> None:
  """After six looks with no new reply, it cancels the reply of the chain that does not end, and adds LANE_HUNG to the thread of job."""
  laning.words["item"] = []
  await laning.word(f'LANE_LOOK = 0.02\nc = chain("item")\njob = prompt(str, "do", to="{SONNET}", on=c)')
  await laning.value("await lane_settled(job, 0.3)")
  replies = [x[1] for x in said(laning.log, "reply") if x[3] == laning.ns["c"]]
  assert isinstance(laning.engine.peek(replies[0]), asyncio.CancelledError)
  assert notes(laning, laning.ns["job"])[0] == laning.ns["LANE_HUNG"]
