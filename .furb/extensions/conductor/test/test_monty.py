"""The round runs on the engine of monty, as the live round does, with no fault in a step."""

import re
from pathlib import Path

import furb
import furb_monty.engine
import pytest

import conftest
from conftest import ENGINES, Sand, born, said, settle, swapped
from furb.engine import Exit, Text

HERE = Path(__file__).resolve().parent.parent
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)
WORD = "\n\n".join(IMPORT.sub("", (HERE / f).read_text(encoding="utf-8")) for f in ("../pr/pr.py", "conductor.py"))
OPEN = '[{"n": 7, "draft": false, "labels": ["w:a"], "mergeable": "MERGEABLE", "base": "main", "head": "w/a", "sha": "s7", "checks": ["SUCCESS"], "names": ["the gates"], "open": [], "files": ["src/a.py"], "auto": false}]'
LISTED = '{}\n--- 0\n[{}]\n--- 0\n{{"sha": "m1", "runs": ["success"]}}\n--- 0\n["the gates"]\n--- 0\n[]\n--- 0\n'


class GitHub(Sand):
  """GitHub holds one green PR of the round in its first listing, and has merged it in each later one."""

  def hear(self, a: tuple):
    match a:
      case ("bash", about, _, _, command, *_):
        out = "64.0\n" if "MemAvailable" in command else ""
        if "gh pr list --state open" in command:
          first = not any("gh pr list --state open" in str(x[4]) for x in self.calls[:-1] if x[0] == "bash")
          out = LISTED.format(OPEN if first else "[]", "" if first else "7")
        yield "done", about, Exit(0, Text(f"{about}/stdout", out), Text(f"{about}/stderr", ""))
      case _:
        yield from super().hear(a)


@pytest.fixture
def monty():
  swapped(ENGINES["monty"])
  yield
  swapped(furb.python)
  if furb_monty.engine.LIFE is not None:
    furb_monty.engine.LIFE.end()
    furb_monty.engine.LIFE = None


async def test_a_round_on_monty_sweeps_staffs_and_ends_with_no_fault(monty: None) -> None:
  sand, log, root = born(files={})
  sand.__class__ = GitHub
  setup = 'ends["a"] = []\nqueue.append(("late", "b", "t"))\nafter["late"] = ["gone"]\nareas["late"] = ["src/a.py"]\n'
  conftest.engine.rung(WORD + "\n\n" + setup + "await sweep()\nstaff()\ntend()\nawait review()\nasked()\nalign()\nqueue.clear()\ngot = await conduct(0)", on=root)
  for _ in range(60):
    await settle(200)
    if any("ended:" in str(x[5]) for x in said(log, "prompt")):
      break
  told = [str(x[5]) for x in said(log, "prompt")]
  assert [x for x in told if "met a fault" in x] == []
  assert not [x for x in said(log, "tell") if "raised" in str(x[3])]
  assert [x for x in told if "ended:" in x][-1].startswith("The round on chain1 ended: it holds no item")
  assert any("gh pr merge 7 --auto --squash" in str(x[4]) for x in said(log, "bash"))


class Pending(Sand):
  """Pending holds one PR of the round open in its first two listings, and has merged it in each later one."""

  def hear(self, a: tuple):
    match a:
      case ("bash", about, _, _, command, *_):
        out = "64.0\n" if "MemAvailable" in command else ""
        if "gh pr list --state open" in command:
          early = sum("gh pr list --state open" in str(x[4]) for x in self.calls[:-1] if x[0] == "bash") < 2
          out = LISTED.format(OPEN if early else "[]", "" if early else "7")
        yield "done", about, Exit(0, Text(f"{about}/stdout", out), Text(f"{about}/stderr", ""))
      case _:
        yield from super().hear(a)


async def test_a_round_with_no_live_worker_waits_on_its_chain_through_two_beats(monty: None) -> None:
  """A round that tends an open PR with no worker waits for its next beat on its own chain, which the engine takes."""
  sand, log, root = born(files={})
  sand.__class__ = Pending
  conftest.engine.rung(WORD + '\n\nends["a"] = []\ngot = await conduct(0)', on=root)
  for _ in range(60):
    await settle(200)
    if any("ended:" in str(x[5]) for x in said(log, "prompt")):
      break
  ended = [str(x[5]) for x in said(log, "prompt") if "ended:" in str(x[5])]
  assert len(ended) == 1 and ended[0].startswith("The round on chain1 ended: it holds no item"), ended
  assert [x[4:] for x in said(log, "wait")] == [(0, "chain1"), (0, "chain1")]
