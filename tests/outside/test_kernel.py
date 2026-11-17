"""The Kernel of this interpreter: how a word runs in the module of its chain.

The run is begun and carried by the facts of the engine, so it is driven through a life whose World answers a
standing, takes a wait, and takes nothing else.
"""

import asyncio
import re

import pytest

import furb
import furb.checkpoint
import furb.kernel
import furb.record
import furb_monty.engine
from conftest import Sand, World, born, journaled, kernel, life, plain, relived, said, settle, swapped
from furb import engine
from furb.engine import OPERATOR, WINDOW, Drift, Refused
from outside.doubles import booted, worlds

STANDS = [[[OPERATOR, [], WINDOW], ["opus", ["low"], 1000]], "/w", "opus/low"]


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_reads_a_name_of_the_engine_as_the_program_bound_it_last(which: object) -> None:
  """A rung that bound a name of the engine again leaves that value to the word after it, on both engines, so a word
  that calls it reads that value when it runs."""
  swapped(which)
  try:
    root = engine.boot(journaled(), world=worlds(STANDS), **kernel())
    assert await engine.rung("def read(path: str) -> int:\n  return len(path)", on=root) is None
    assert await engine.rung("close(read('ab'))", on=root) == 2
  finally:
    swapped(furb.python)


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_compares_sets_as_python_does_on_both_engines(which: object) -> None:
  """A set and a frozenset compare by the subset order, by their members and by the methods of sets, on both engines,
  as python compares them."""
  swapped(which)
  try:
    root = engine.boot(journaled(), world=worlds(STANDS), **kernel())
    word = (
      "a, b, c = {1}, {1, 2}, frozenset({1, 2})\n"
      "close([a < b, b < c, a <= b, b <= c, b > a, c > b, b >= a, c >= b, a == b, b == c, a != b,"
      " a.issubset(c), c.issuperset(a), a.isdisjoint({3})])"
    )
    assert await engine.rung(word, on=root) == [
      True,
      False,
      True,
      True,
      True,
      False,
      True,
      True,
      False,
      True,
      True,
      True,
      True,
      True,
    ]
  finally:
    swapped(furb.python)


async def test_a_word_awaits_an_act_and_nothing_else() -> None:
  """A rung awaits an act, a future of the loop of its word or the next turn of that loop, and nothing else, so
  anything else it awaits is refused where it waited, and a word that catches the refusal carries on."""
  root = booted(worlds(STANDS))
  odd = "class Odd:\n  def __await__(self):\n    yield 1\n"
  with pytest.raises(Refused, match="a rung awaits an act"):
    await engine.rung(odd + "await Odd()", on=root)
  word = odd + "try:\n  await Odd()\nexcept Refused as no:\n  why = str(no)\nclose(why)"
  assert "a rung awaits an act" in str(await engine.rung(word, on=root))
  assert await engine.rung("import asyncio\nawait asyncio.sleep(0)\nclose('next turn')", on=root) == "next turn"


async def test_a_word_the_interpreter_cannot_compile_is_what_the_run_came_to() -> None:
  """A pin: the compile stands inside the run, so a word the interpreter will not take is what the run came to and
  no more, and never an error out of the life that began the frame."""
  root = booted(worlds(STANDS))
  with pytest.raises(SyntaxError, match="'yield' outside function"):
    await engine.rung("x = yield 1", on=root)
  assert await engine.rung("close('alive')", on=root) == "alive"


async def test_a_word_that_awaits_an_act_already_over_is_carried_on_where_it_stands() -> None:
  """An act that is over when the word comes to await it hands its value straight in, so the run is never suspended
  for a done that already stands; a cancelled one raises where the word waited."""
  root = booted(worlds(STANDS))
  assert (
    await engine.rung("a = wait(0, __name__)\nawait wait(0.02, __name__)\nawait a\nclose('both')", on=root) == "both"
  )
  word = "a = wait(30, __name__)\ncancel(a)\nawait wait(0.02, __name__)\ntry:\n  await a\nexcept BaseException:\n  close('cut')"
  assert await engine.rung(word, on=root) == "cut"


async def test_a_run_whose_end_the_record_keeps_steps_no_more_when_the_act_that_it_awaited_ends() -> None:
  """A run that is over, as one whose end the record keeps, steps no more: the act that it awaited ends after the
  record, and the rest of its word never runs."""
  sand, _, root = born()
  await engine.rung("x = await wait(0.01, __name__)\ny = 2", on=root)
  await settle()
  at = next(i for i, e in enumerate(sand.record) if e[0][:2] == ("done", "wait1"))
  life(Sand(), [*sand.record[:at], (("done", "run2", "", Refused("cut")),)])
  assert engine.over("run2") and not engine.over("wait1")
  engine.cancel("wait1")
  await settle()
  assert engine.over("wait1") and str(engine.peek("run2")) == "cut"
  assert "x" not in engine.module(root) and "y" not in engine.module(root)


async def test_a_record_whose_acts_hold_a_callable_or_an_address_replays_on_the_engine_of_this_interpreter() -> None:
  """A record keeps a callable or an object of a word as a text that names an address of its life, which a later
  life cannot make again, so a host of this engine gives boot what recorded makes of the record: each such text
  stands for each value that shows the same with no address, and the act that the word makes again is the act of the
  record."""
  word = "class A:\n  pass\ny = bash(f'echo {A()}')\nawait y\nclose(1)"
  sand, _, root = born(word)
  assert await engine.prompt(int, "go", on=root) == 1
  addressed = lambda one: re.sub(r" at 0x[0-9a-f]+", " at 0x1", one) if isinstance(one, str) else one  # noqa: E731
  record: list = [(tuple(map(addressed, e)), *rest) for (e, *rest) in plain(sand.record)]
  assert any(" at 0x1" in str(e) for e in record)
  with pytest.raises(Drift, match=r"^bash1 drifts"):
    engine.boot(record, **kernel(), world=Sand().hears())
  log, again = await relived(Sand(), record)
  assert again == root and [a[4][:6] for a in log if a[0] == "bash"] == ["echo <"]
  held = furb.record.Recorded("<f at 0x1>")
  assert furb.record.Recorded() == len and held == "<f at 0x2>" and held != "<g at 0x1>"
  assert (
    held == furb.record.Recorded("<f at 0x1>") and repr(furb.record.Recorded()) == "<a callable of an earlier life>"
  )
  coroutine = type("Coroutine", (), {"__repr__": lambda _: "<coroutine object c at 0x1f>"})()
  shown = furb.record.Recorded("Repr('<coroutine object c>')")
  assert shown == coroutine and shown == "Repr('<coroutine object c>')" and shown != "<coroutine object c>"
  wrote = (
    ("write", "write1", "rung1", "chain1", engine.Text("a", "<A at 0x1>"), ["<B at 0x1>"], {"c": "<C at 0xAB>"}),
  )
  (stood,) = furb.record.recorded(vars(furb.python), [wrote])
  assert stood[0][4:] == (engine.Text("a", "<A at 0x2>"), ["<B at 0x2>"], {"c": "<C at 0x2f>"})


async def test_a_word_that_awaits_each_command_pays_one_wants_for_each_and_nothing_more() -> None:
  """The cost of an await, counted as facts, which no load changes: a word that awaits each of its commands says one
  wants for each, which its rung starts and ends and the journal keeps once, and each command says the same facts as
  one that the word does not await."""

  async def said(word: str) -> list[tuple]:
    _, log, root = born(word)
    asked = engine.prompt(int, "go", on=root)
    for _ in range(100):
      await settle()
      if engine.peek(asked) is not None:
        break
    return log

  made = await said("for _ in range(10):\n  bash('x')\nclose(1)")
  awaited = await said("for _ in range(10):\n  await bash('x')\nclose(1)")
  wants = {a[1] for a in awaited if a[0] == "wants"}
  assert len(wants) == 10
  assert sorted(a[0] for a in awaited if a[1] in wants) == ["done"] * 10 + ["started"] * 10 + ["wants"] * 10
  assert sum(a[0] == "keep" and a[3][0][1] in wants for a in awaited) == 10

  def of_commands(log: list[tuple]) -> list[tuple]:
    commands = {a[1] for a in log if a[0] == "bash"}
    return [a[:2] for a in log if a[1] in commands or (a[0] == "keep" and a[3][0][1] in commands)]

  assert sorted(of_commands(awaited)) == sorted(of_commands(made))


async def test_a_word_reads_the_futures_and_the_tasks_of_its_loop_as_cpython_reads_them() -> None:
  """A word reads a future or a task of the loop of its word as in CPython: its loop, its result, the exception that
  it raised where it is awaited, a gather of a task that is done, a callback of a task that is done, which the next
  turn of the loop runs, and a cancel of a future that is done, which does nothing; and the loop refuses what a
  future cannot do yet or again, and what the loop of a word does not do."""
  root = booted(worlds(STANDS))
  word = (
    "import asyncio\n"
    "loop = asyncio.get_running_loop()\n"
    "async def gives():\n"
    "  return 1\n"
    "async def fails():\n"
    "  raise ValueError('no')\n"
    "given, failing = asyncio.create_task(gives()), asyncio.create_task(fails())\n"
    "try:\n"
    "  given.exception()\n"
    "except RuntimeError as no:\n"
    "  early = str(no)\n"
    "both = await asyncio.gather(given, gives())\n"
    "try:\n"
    "  await failing\n"
    "except ValueError as no:\n"
    "  raised = str(no)\n"
    "again = await asyncio.gather(given)\n"
    "seen = []\n"
    "given.add_done_callback(lambda task: seen.append(task.result()))\n"
    "await asyncio.sleep(0)\n"
    "ready = loop.create_future()\n"
    "ready.set_result(3)\n"
    "try:\n"
    "  ready.set_result(4)\n"
    "except RuntimeError as no:\n"
    "  twice = str(no)\n"
    "try:\n"
    "  loop.time()\n"
    "except AttributeError as no:\n"
    "  timeless = str(no)\n"
    "close([given.get_loop() is loop, early, both, raised, again, seen, given.cancel(), ready.cancel(), twice, timeless,"
    " loop.is_running(), loop.is_closed()])"
  )
  assert await engine.rung(word, on=root) == [
    True,
    "the future is not done",
    [1, 1],
    "no",
    [1],
    [1],
    False,
    False,
    "the future is done already",
    "the loop of a word has no time: a word awaits acts, as wait(seconds, act)",
    True,
    False,
  ]


async def test_a_task_that_is_cancelled_while_it_waits_for_a_future_cancels_that_future_and_raises_cancellederror() -> (
  None
):
  """A task that is cancelled while it waits for a future of the loop of its word cancels that future, and raises
  CancelledError where it is awaited, as in CPython."""
  root = booted(worlds(STANDS))
  word = (
    "import asyncio\n"
    "ready = asyncio.get_running_loop().create_future()\n"
    "async def waits():\n"
    "  return await ready\n"
    "task = asyncio.create_task(waits())\n"
    "await asyncio.sleep(0)\n"
    "task.cancel()\n"
    "try:\n"
    "  await task\n"
    "except asyncio.CancelledError:\n"
    "  close(ready.cancelled())"
  )
  assert await engine.rung(word, on=root) is True


async def test_a_task_that_is_cancelled_while_it_waits_for_an_act_raises_cancellederror_and_the_act_goes_on() -> None:
  """A task that is cancelled while it waits for an act raises CancelledError where it is awaited, and the act goes
  on, since only a cancel that the transcript shows ends an act."""
  word = (
    "import asyncio\n"
    "slow = bash('slow')\n"
    "async def waits():\n"
    "  return (await slow).code\n"
    "task = asyncio.create_task(waits())\n"
    "await asyncio.sleep(0)\n"
    "task.cancel()\n"
    "try:\n"
    "  await task\n"
    "except asyncio.CancelledError:\n"
    "  going = peek(slow, ...) is ...\n"
    "close([going, (await slow).code])"
  )
  sand, log, root = born(word, auto=False)
  asked = engine.prompt(list, "go", on=root)
  await settle()
  (slow,) = [a[1] for a in said(log, "bash")]
  sand.exits(slow, 0)
  await settle()
  assert engine.peek(asked) == [True, 0]


async def test_a_task_that_awaits_an_act_that_is_over_while_its_word_waits_goes_on_with_what_the_act_came_to() -> None:
  """A task that awaits an act that is over while another job of its run waits through a wants goes on at once with
  what the act came to."""
  word = (
    "import asyncio\n"
    "first = bash('one')\n"
    "await first\n"
    "async def reads():\n"
    "  return (await first).code\n"
    "task = asyncio.create_task(reads())\n"
    "await bash('two')\n"
    "close(await task)"
  )
  sand, log, root = born(word, auto=False)
  asked = engine.prompt(int, "go", on=root)
  await settle()
  commands = lambda: {a[4]: a[1] for a in said(log, "bash")}  # noqa: E731
  sand.exits(commands()["one"], 0)
  await settle()
  sand.exits(commands()["two"], 0)
  await settle()
  assert engine.peek(asked) == 0


async def test_a_control_that_ends_a_rung_drops_each_task_of_its_word_and_the_run_that_waits_for_them() -> None:
  """A control that ends a rung drops each task of its word that waits, and the run whose word is over and that waits
  for its tasks is done with CancelledError."""
  word = "import asyncio\nasync def waits():\n  await bash('slow')\nkept = asyncio.create_task(waits())"
  _, log, root = born(word, auto=False)
  engine.prompt(str, "go", on=root)
  await settle()
  (run,) = [a for a in said(log, "run") if "kept" in a[5]]
  assert engine.peek(run[1], ...) is ...
  engine.cancel(run[4])
  await settle()
  assert isinstance(engine.peek(run[1], ...), asyncio.CancelledError)


async def test_an_ear_that_asks_the_running_loop_while_a_word_makes_an_act_is_answered_by_its_own_loop() -> None:
  """What an ear asks of the running loop while a word makes an act goes to the loop that ran before, which is that
  ear's own: its tasks and its clock."""

  class Asking(Sand):
    def hear(self, a: tuple) -> World:
      if a[0] == "bash":
        running = asyncio.get_running_loop()
        self.asked = (running.create_task(asyncio.sleep(0)), running.time())
      yield from Sand.hear(self, a)

  sand = Asking(files={"/w/a.txt": "one\ntwo\n"})
  _, root = life(sand)
  sand.script[root] = ["close((await bash('x')).code)"]
  assert await engine.prompt(int, "go", on=root) == 0
  task, time = sand.asked
  assert isinstance(task, asyncio.Task) and isinstance(time, float)
