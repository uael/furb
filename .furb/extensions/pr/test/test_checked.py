"""checked awaits one command that waits for the checks of a PR within its bound, on the real engine."""

from conftest import said
from extensions.conftest import Bench

LINK = "https://github.com/uael/furb/actions/runs/77/job/{}"


def github(made: Bench, rows: list[dict], runs: list[dict]) -> None:
  made.say(r"^pr checks", rows)
  made.say(r"^pr view", {"headRefOid": "abc1234def"})
  made.say(r"^run list", runs)


async def check(made: Bench) -> tuple[str, list[str]]:
  return await made.value("await checked(5, bound=1, every=0.2)")


async def test_checked_awaits_one_command_and_gives_its_state_and_lines(pulling: Bench) -> None:
  """checked(number) awaits one command that waits for the checks of the PR, and gives a tuple: pass, fail, or pending when the bound ended, and lines whose first names the head."""
  github(pulling, [{"name": "the gates", "bucket": "pass", "link": LINK.format(1)}], [{"status": "completed", "workflowName": "gates"}])
  assert await check(pulling) == ("pass", ["head abc1234: 1 checks, 0 failed"])
  assert len([x for x in said(pulling.log, "bash") if "gh pr checks" in str(x[4])]) == 1


async def test_checked_waits_for_a_check_no_pending_and_no_open_run(pulling: Bench) -> None:
  """It waits until a check is listed, no check pends and no run of the head is open."""
  done = [{"status": "completed", "workflowName": "gates"}]
  for rows, runs in (([], done), ([{"name": "a", "bucket": "pending", "link": ""}], done), ([{"name": "a", "bucket": "pass", "link": ""}], [{"status": "in_progress", "workflowName": "gates"}])):
    github(pulling, rows, runs)
    assert (await check(pulling))[0] == "pending"
  github(pulling, [{"name": "a", "bucket": "pass", "link": ""}], done)
  assert (await check(pulling))[0] == "pass"


async def test_checked_writes_a_line_at_each_beat(pulling: Bench) -> None:
  """Its command writes one short line each every seconds while it waits, so its chain says a fact."""
  github(pulling, [{"name": "a", "bucket": "pass", "link": ""}], [{"status": "completed", "workflowName": "gates"}])
  pulling.say(r"^pr checks 5 --json bucket", [], times=2)
  assert (await pulling.value("await checked(5, bound=60, every=0.2)"))[0] == "pass"
  [ran] = [x[1] for x in said(pulling.log, "bash") if "gh pr checks" in str(x[4])]
  beats = pulling.engine.peek(ran).stderr.lines
  polls = [x for x in pulling.gh() if x.startswith("pr checks 5 --json bucket")]
  assert len(beats) == 2 and len(polls) == 3 and all(x.startswith("waits ") for x in beats)


async def test_checked_names_what_pends_at_the_bound(pulling: Bench) -> None:
  """At the bound it gives pending with what pends: the checks that pend, the run that is open, or that no check started."""
  done = [{"status": "completed", "workflowName": "gates"}]
  github(pulling, [], done)
  assert await check(pulling) == ("pending", ["head abc1234: still pending at the bound of 1 s: no check has started"])
  github(pulling, [{"name": "hooks", "bucket": "pending", "link": ""}], done)
  assert (await check(pulling))[1] == ["head abc1234: still pending at the bound of 1 s: hooks"]
  github(pulling, [{"name": "hooks", "bucket": "pass", "link": ""}], [{"status": "queued", "workflowName": "gates"}])
  assert (await check(pulling))[1] == ["head abc1234: still pending at the bound of 1 s: the run of gates"]


async def test_checked_gives_the_kind_of_each_failed_check(pulling: Bench) -> None:
  """For each failed check it gives [machine] for a fault of the runner or the network, [defect], [cancelled] or [unread], with its run, its job and its first failed lines."""
  rows = [
    {"name": "suite", "bucket": "fail", "link": LINK.format(5)},
    {"name": "hooks", "bucket": "fail", "link": LINK.format(6)},
    {"name": "bench", "bucket": "cancel", "link": LINK.format(7)},
    {"name": "odd", "bucket": "fail", "link": "https://example.com/x"},
  ]
  github(pulling, rows, [{"status": "completed", "workflowName": "gates"}])
  pulling.say(r"actions/jobs/5/logs", "step\nThe runner has received a shutdown signal.\n")
  pulling.say(r"actions/jobs/6/logs", "step\nFAILED test/test_a.py::test_b - AssertionError\n")
  state, lines = await check(pulling)
  assert state == "fail" and lines[0] == "head abc1234: 4 checks, 4 failed"
  assert lines[1:] == [
    "suite [machine] run 77 job 5",
    "  step",
    "  The runner has received a shutdown signal.",
    "hooks [defect] run 77 job 6",
    "  FAILED test/test_a.py::test_b - AssertionError",
    f"bench [cancelled] {LINK.format(7)}",
    "odd [unread] https://example.com/x",
  ]
