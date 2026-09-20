"""Kernel, the interface that gates and runs the rungs of a chain."""

from asyncio import CancelledError

from conftest import STANDS, Sand, gated, life, ran, said, settle
from furb import engine
from furb.engine import OPERATOR, Exit


async def test_the_chain_has_the_kernel_gate_and_begin_every_rung_by_the_facts_gate_and_run() -> None:
  """The chain has the Kernel gate and begin every rung, by the facts gate and run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  assert gated(log) == ["k = 1", "close(k + 1)"] and ran(log) == gated(log)
  assert [one[2] for one in said(log, "done") if one[1].startswith("gate://")] == ["kernel", "kernel"]


async def test_the_kernel_answers_a_gate_with_its_findings_and_runs_the_word_of_a_run() -> None:
  """The Kernel answers a gate with its findings, runs the word of a run in the module of the chain the run names, says wants for the act a run waits for, takes a sent of what that act came to, and says ran with what the word gave."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nout = await x\nclose(out.code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  command, step = said(log, "bash")[0][1], said(log, "rung")[0][1]
  assert [one[3] for one in said(log, "done") if one[1].startswith("gate://")] == [[]]
  assert [(one[1], one[3]) for one in said(log, "run")] == [(step, root)]
  assert [(one[1], one[3]) for one in said(log, "wants")] == [(step, command)]
  assert [(one[1], type(one[3]).__name__) for one in said(log, "sent")] == [(step, "Exit")]
  assert [isinstance(one[3], CancelledError) for one in said(log, "ran")] == [True]
  out = engine.modules[root]["out"]
  assert isinstance(out, Exit) and out.code == 0


async def test_the_kernel_sets_the_site_to_the_rung_whose_word_it_steps() -> None:
  """The Kernel sets the site to the rung whose word it steps, for as long as it steps it, so what the word says is said by that rung."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  step, command = said(log, "rung")[0][1], said(log, "bash")[0]
  assert command[2] == step and engine.site.get() == OPERATOR
