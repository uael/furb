"""lane_work, the loop of a worker on a command that fails."""

from conftest import said
from skills.conftest import Bench

from ...conftest import gitted


def made(laning: Bench, name: str) -> str:
  """The word of a worker that commits the file name in the tree, then closes."""
  return f'await bash("cd {laning.tmp} && echo {name} > {name} && git add {name} && git commit -qm {name}")\nclose("made {name}")'


def asked(laning: Bench) -> list[str]:
  """The message of each prompt of the item to its worker."""
  return [str(x[5]) for x in said(laning.log, "prompt") if x[4] == "str" and x[6] == "claude-cli:sonnet/medium"]


async def work(laning: Bench, accept: str, **words: object) -> object:
  more = "".join(f", {k}={v!r}" for k, v in words.items())
  await laning.word('c = chain("item")')
  return await laning.value(f'await lane_work(c, {accept!r}, "First.", {str(laning.tmp)!r}{more})')


async def test_lane_work_asks_the_worker_until_the_command_passes(laning: Bench) -> None:
  """lane_work(c, accept, first, tree) runs accept in tree, asks worker on c with first to make it pass while it fails, and runs it again after each close, so a red command turns green."""
  laning.words["item"] = [made(laning, "b"), "close(None)"]
  assert await work(laning, "test -f b") == ("accepted", "test -f b")
  [message] = asked(laning)
  assert message.startswith("First.\nThe acceptance command, which a word with no model runs in the tree after each close of yours")
  assert "\ntest -f b\n\nIt exits 1 now." in message


async def test_lane_work_gives_void_when_the_command_passes_before_any_work(laning: Bench) -> None:
  """It gives void when accept passes before any work, unless begun says that the item stands in its middle."""
  assert await work(laning, "true") == ("void", "the acceptance command passed before any work")
  laning.words["item"] = ["close(None)"]
  assert await work(laning, "true", begun=True) == ("accepted", "true")
  assert asked(laning) == []


async def test_lane_work_ends_blocked_when_the_worker_closes_with_no_commit(laning: Bench) -> None:
  """It gives blocked with the close of the worker when the head of tree did not move, since a close with no commit changes nothing that accept reads."""
  laning.words["item"] = ['close("nothing to do")']
  assert await work(laning, "test -f b") == ("blocked", "the worker closed with no new commit: nothing to do")


async def test_lane_work_asks_the_worker_to_fix_each_check_that_fails(laning: Bench) -> None:
  """Once accept passes, it runs each check of LANE_CHECKS in tree, with BASE put to the head where the item began, and asks worker to fix each check that fails."""
  base = gitted(laning.tmp, "rev-parse", "HEAD").strip()
  await laning.word('LANE_CHECKS = {"the fix": "test -f fixed && test BASE = ' + base + '"}')
  laning.words["item"] = [made(laning, "b"), made(laning, "fixed"), "close(None)"]
  assert await work(laning, "test -f b") == ("accepted", "test -f b")
  assert asked(laning)[1] == "The acceptance command passes, and these checks of code fail. Fix each one at its cause, commit and push:\nthe fix: "


async def test_lane_work_joins_a_probe_that_fails_to_the_command(laning: Bench) -> None:
  """Once the checks pass, it asks each breaker, one breaker by default, on c for a probe, a shell command that fails on the head, and a probe that fails joins accept, which worker then makes pass."""
  laning.words["item"] = [made(laning, "b"), 'close("test -f c")', made(laning, "c"), "close(None)"]
  assert await work(laning, "test -f b") == ("accepted", "test -f b && ( test -f c )")
  assert asked(laning)[1].startswith("A probe of another model fails on the head, and it is now a part of the acceptance command:\ntest -f c\n")
  probes = [(x[3], x[6]) for x in said(laning.log, "prompt") if x[4] == "str | None"]
  assert [who for _, who in probes] == ["claude-cli:opus/high", "claude-cli:opus/high"]
  assert {on for on, _ in probes} == {laning.ns["c"]}


async def test_lane_work_refuses_a_probe_that_changed_the_tree(laning: Bench) -> None:
  """It refuses the probe of a breaker that changed the files or the head of tree, and asks the next breaker."""
  tree = str(laning.tmp)
  laning.words["item"] = [made(laning, "b"), f'await bash("cd {tree} && echo x > stray")\nclose("false")', "close(None)"]
  breakers = ["claude-cli:opus/high", "claude-cli:fable/high"]
  assert await work(laning, "test -f b", breakers=breakers) == ("accepted", "test -f b")
  assert [x[6] for x in said(laning.log, "prompt") if x[4] == "str | None"] == breakers


async def test_lane_work_ends_blocked_after_16_closes(laning: Bench) -> None:
  """It gives accepted with accept once no breaker gives a probe that fails, and blocked after 16 closes of worker."""
  laning.words["item"] = [made(laning, f"f{n}") for n in range(16)]
  assert await work(laning, "false") == ("blocked", "16 closes of the worker, and the acceptance command still fails")
  assert len(asked(laning)) == 16
