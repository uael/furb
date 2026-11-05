"""checked runs on the engine of monty with its real wait, as a worker that awaits it does."""

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
WORD = IMPORT.sub("", (HERE / "pr.py").read_text(encoding="utf-8"))
RUNS = '[{"status": "completed", "workflowName": "ci"}]'


class Checks(Sand):
  """Checks holds the check of the PR pending in its first poll, and passed in each later one."""

  def hear(self, a: tuple):
    match a:
      case ("bash", about, _, _, command, *_) if "gh pr checks" in command:
        first = not any("gh pr checks" in str(x[4]) for x in self.calls[:-1] if x[0] == "bash")
        out = f'abc1234\n[{{"name": "the gates", "bucket": "{"pending" if first else "pass"}", "link": ""}}]\n{RUNS}\n'
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


async def test_a_poll_of_checked_waits_on_its_chain_and_the_engine_takes_that_wait(monty: None) -> None:
  """No act of the life says that a check moved, so checked waits on its own chain, since the engine refuses a wait of
  no act, as #1218 made it."""
  sand, log, root = born(files={})
  sand.__class__ = Checks
  conftest.engine.rung(WORD + "\n\ngot = await checked(7, every=0)\nprompt(None, repr(got))", on=root)
  for _ in range(60):
    await settle(200)
    if said(log, "prompt"):
      break
  assert [str(x[5]) for x in said(log, "prompt")] == ["('pass', ['head abc1234: 1 checks, 0 failed'])"]
  assert [x[4:] for x in said(log, "wait")] == [(0, "chain1")]
