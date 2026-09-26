"""Act.__await__, to await an act for what it comes to."""

import asyncio
from asyncio import CancelledError

import pytest

from conftest import Sand, acts, heads, life, rows, said, settle
from furb import engine
from furb.engine import OPERATOR, Exit, Text


async def test_to_await_an_act_gives_the_value_of_the_act_when_the_act_completes() -> None:
  """To await an act gives the value of the act when the act completes."""
  sand = Sand()
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert (await one).code == 0


async def test_an_act_is_awaited_from_any_chain() -> None:
  """An act is awaited from any chain."""
  sand = Sand()
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(x)"]
  which = await engine.prompt(str, "start one", on=root)
  two = engine.chain("two")
  sand.script[two] = [f"out = await Act({which!r})\nassert isinstance(out, Exit)\nclose(out.code)"]
  assert await engine.prompt(int, "await it", on=two) == 0


async def test_a_rung_that_awaits_an_act_reads_the_result_of_the_act() -> None:
  """A rung that awaits an act reads the result of the act."""
  sand = Sand()
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nout = await x\nclose([out.code, out.stdout.content])"]
  assert await engine.prompt(list, "run it", on=root) == [0, "ran echo hi\n"]


async def test_a_rung_awaits_an_act_and_nothing_else() -> None:
  """A rung awaits an act and nothing else."""
  sand = Sand(auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  one = engine.prompt(int, "run it", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert [a[4] for a in said(log, "wants")] == [command]
  assert {a[4] for a in said(log, "wants")} <= set(acts(log))
  engine.cancel(one)


async def test_to_await_an_act_raises_the_exception_that_the_act_completed_with() -> None:
  """To await an act raises the exception that the act completed with."""
  sand = Sand()
  _, root = life(sand)
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(ValueError("boom"), one)
  with pytest.raises(ValueError, match="boom"):
    await one


async def test_to_await_a_cancelled_act_raises_cancellederror() -> None:
  """To await a cancelled act raises CancelledError."""
  sand = Sand(auto=False)
  _, root = life(sand)
  one = engine.bash("slow", on=root)
  engine.cancel(one)
  with pytest.raises(CancelledError):
    await one


async def test_a_word_that_awaits_a_chain_raises_refused_where_it_waited() -> None:
  """A word that awaits a chain raises Refused where it waited, since a chain never settles and the word could go no further."""
  sand = Sand()
  log, root = life(sand)
  sand.script[root] = ["sub = chain('sub')\nawait sub\nclose(1)", "close(2)"]
  assert await engine.prompt(int, "fork", on=root) == 2
  await settle()
  assert heads(engine.turns(on=root)) == [
    "#chain1 root",
    rows("chain1")[0],
    "#prompt1 fork",
    "#rung1 advance on prompt1",
    "#rung1 raised Refused('chain2 never settles')",
    "#rung3 advance on prompt1",
    "#prompt1 closed 2",
  ]
  assert [(a[1], a[4]) for a in said(log, "chain")] == [("chain1", "root"), ("chain2", "sub")]


async def test_the_awaiter_of_the_prompt_raises_that_exception() -> None:
  """The awaiter of the prompt raises that exception."""
  sand = Sand()
  log, root = life(sand)
  sand.script[root] = [
    "p = prompt(int, 'ask them', to=OPERATOR)\ntry:\n  await p\nexcept ValueError as no:\n  close(str(no))"
  ]
  one = engine.prompt(str, "delegate", on=root)
  await settle()
  theirs = said(log, "prompt")[-1][1]
  engine.close(ValueError("boom"), theirs)
  await settle()
  assert (await one) == "boom"


async def test_a_run_that_awaits_it_hands_it_to_whoever_steps_the_run() -> None:
  """A run that awaits it hands it to whoever steps the run, since the engine owns the order of every run; the operator, which the engine does not step, waits on its own loop."""
  sand = Sand()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  command = said(log, "bash")[0]
  assert [(a[4], engine.get(a[2])[4]) for a in said(log, "wants")] == [(command[1], command[2])]
  mine = engine.bash("echo again", on=root)
  assert (await mine).code == 0 and [a[4] for a in said(log, "wants")] == [command[1]]
  sand.auto = False
  slow = engine.bash("slow", on=root)
  with pytest.raises(TimeoutError):
    await asyncio.wait_for(slow, 0.01)
  sand.exits(slow, 0)
  assert engine.peek(slow) == Exit(0, Text(f"{slow}/stdout"), Text(f"{slow}/stderr"))
