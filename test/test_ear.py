"""Ear, any generator that hears the facts of a life and speaks by say: the World and the Kernel among them."""

import asyncio
from asyncio import CancelledError
from collections.abc import Generator
from functools import partial

from conftest import STANDS, Py, Sand, Where, World, dones, keeping, kernel, life, ran, said, settle, sown, world_says
from furb import engine
from furb.engine import OPERATOR, WORLD, Exit, Refused, Text

PING = "def ping(id):\n  while True:\n    yield\n\nclose(await act('ping', '', ping))\n"


class Knows(Sand):
  """A World that knows one act an extension defines, and takes it when it is put to it."""

  def hears(self) -> World:
    """The World that answers a standing and a reply, and does the act of an extension that it takes."""
    loop = asyncio.get_running_loop()
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          self.calls.append(a)
          engine.say("done", qid, self.stands or [[], "", ""])
        case ("ping", about, *_):
          self.calls.append(a)
          engine.say("done", about, "did ping")
        case ("reply", about, _, on, _) if self.script.get(on):
          self.calls.append(a)
          engine.say("started", about)
          word = self.script[on].pop(0)
          turn = ("assistant", word, (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
          loop.call_soon(partial(world_says, "done", about, turn))


async def test_the_world_hears_every_fact() -> None:
  """The World hears every fact, and every act that no ear before it took: it answers a stand, a clock, a chance, a read and a write of a path nobody of the engine serves, resolved against the working directory of the chain; it takes a command, asks it whether it is merged, feeds it, and ends it at its timeout and at a cancel; it takes a wait and a prompt to the operator, which it shows; and it takes a reply, which it answers with the turn of the model."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS, auto=False)
  log, root = life(sand)
  assert engine.cwd(on=root) == "/w"
  assert engine.clock(on=root) == 1001.0
  assert engine.chance(on=root) == 2 / 7
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\n")
  assert engine.write(Text("b.txt", "two\n"), on=root) == Text("/w/b.txt", "two\n")
  fed = engine.bash("forever", fed=True, on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  held = engine.transcript(root)
  assert [a[4] for a in said(held, "merged")] == [command]
  assert engine.write(Text(f"{command}/stdin", "go"), on=root) == Text(f"{command}/stdin", "go")
  assert sand.fed == ["go"]
  engine.cancel(fed)
  await settle()
  assert isinstance(engine.peek(fed), CancelledError)
  short = engine.bash("slow", timeout=0.05, on=root)
  assert (await short).code is None
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  asking = engine.prompt(Text, "a text?", to=OPERATOR, on=root)
  await settle()
  assert isinstance(engine.peek(asking), Refused)
  where = Where(files={"/x/a.txt": "three\n"}, stands=STANDS)
  _, other = life(where)
  assert engine.cd("/x", on=other) == "/x"
  assert engine.read("a.txt", on=other) == Text("/x/a.txt", "three\n")
  assert (await engine.bash("echo hi", on=other)).code == 0
  assert where.where == ["/x"]


async def test_the_world_performs_any_fact_that_an_extension_defines_and_that_the_world_knows() -> None:
  """The World performs any fact that an extension defines and that the World knows."""
  sand = Knows(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = [PING]
  assert await engine.prompt(str, "an act of my own", on=root) == "did ping"
  assert [a[1] for a in said(sand.calls, "ping")] == ["ping1"]


async def test_the_facts_that_the_world_says_of_its_own_are_for_the_acts_that_complete_later() -> None:
  """The facts that the World says of its own are for the acts that complete later."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  _, asked, *_ = said(log, "reply")[0]
  own = [a for a in log if a[2] == WORLD and a[0] in ("out", "done") and engine.get(a[1])[0] in ("bash", "reply")]
  assert [(a[0], a[1]) for a in own] == [("done", asked), ("out", command), ("done", command)]
  assert {a[1] for a in own} == {command, asked}


async def test_an_ear_yields_only_to_wait_for_what_it_hears_next() -> None:
  """An ear yields only to wait for what it hears next: it speaks by say and asks by act while it hears, and the work it began speaks later by say, under the site of the ear that began it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  _, standing, *_ = said(log, "stand")[0]
  facts = [(a[0], a[1]) for a in log if a[2] == WORLD and engine.get(a[1]) != a]
  assert facts == [("done", standing), ("started", act), ("out", act), ("done", act)]
  assert [(a[1], a[2], a[4]) for a in said(log, "merged")] == [("merged1", WORLD, act)]


async def test_an_ear_hears_every_fact_that_is_no_question() -> None:
  """An ear hears every fact that is no question, in the order of the log, and a question only while it is offered it."""
  sand = Sand(stands=STANDS)
  before: list[tuple] = []
  after: list[tuple] = []
  root = engine.boot((), **kernel(), before=keeping(before), world=sand.hears(), after=keeping(after))
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  assert [a[1] for a in before if engine.question(a)] == ["stand1", act]
  assert [a for a in before if not engine.question(a)] == after != []
  assert [a for a in after if engine.question(a)] == []


def note(kept: list[tuple]) -> Generator[None, tuple]:
  """An ear of the outside that answers a read of a door of its own and keeps every fact it hears."""
  while True:
    match a := (yield):
      case ("read", qid, _, _, path) if path.startswith("note://"):
        kept.append(a)
        engine.say("done", qid, Text(path, "kept"))
      case (_, _, _, *_):
        kept.append(a)


async def test_an_ear_is_any_generator_of_that_shape() -> None:
  """An ear is any generator of that shape, so the World and the Kernel are ears, and boot takes an ear of the outside under any name it is to hear by but the name of the operator."""
  sand = sown()
  kept: list[tuple] = []
  root = engine.boot((), world=sand.hears(), kernel=Py().kernel(), gate=Py().gating(), note=note(kept))
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  assert engine.read("note://one", on=root) == Text("note://one", "kept")
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "reply", "read", "read"]
  runs = [a[1] for a in kept if a[0] == "started" and engine.question(("run", a[1]))]
  assert [engine.get(run)[5] for run in runs] == [
    "chain1: Act[object] = Act('chain1')\nprompt1: Act[int] = Act('prompt1')",
    "close(1)",
  ]
  assert [a[4] for a in kept if a[0] == "read"] == ["note://one"]
  assert {"started", "ready", "done"} <= {a[0] for a in kept} and not {"reply", "run"} & {a[0] for a in kept}


async def test_the_chain_has_the_gate_read_and_the_kernel_begin_every_rung() -> None:
  """The chain has the gate read every rung but the ones it wrote itself, and the Kernel begin every rung, by the acts gate and run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  assert ran(log) == [
    "k = 1",
    "chain1: Act[object] = Act('chain1')\nprompt1: Act[int] = Act('prompt1')",
    "close(k + 1)",
  ]
  assert [one[2] for one in said(log, "done") if one[1].startswith("gate")] == ["gate", "gate"]
  assert [(one[1], one[2]) for one in said(log, "rung") if one[4]] == [("rung1", "operator"), ("rung3", root)]


async def test_the_kernel_takes_a_run_as_that_run() -> None:
  """The Kernel takes a run as that run, begins its word in the module of the chain when it hears that it took it, makes a wants as the run when the word waits for an act that is not done, carries the word on at the done of that wants, and says the run done with what the word gave."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nout = await x\nclose(out.code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  runs = said(log, "run")
  assert [(a[1], a[4]) for a in runs] == [("run1", "rung2"), ("run2", "rung1"), ("run3", "rung4")]
  assert [(a[0], a[2]) for a in log if a[1] == "run2" and a[0] in ("started", "done")] == [
    ("started", "run2"),
    ("done", "run2"),
  ]
  assert [(a[1], a[2], a[4]) for a in said(log, "wants")] == [("wants1", "run2", "bash1")]
  assert [(a[2], type(a[3]).__name__) for a in dones(log, "wants")] == [("rung1", "Exit")]
  assert [type(engine.peek(a[1])) for a in runs] == [type(None), CancelledError, type(None)]
  out = engine.module(root)["out"]
  assert isinstance(out, Exit) and out.code == 0


async def test_the_kernel_sets_the_site_to_the_rung_whose_word_it_steps() -> None:
  """The Kernel sets the site to the rung whose word it steps, for as long as it steps it, so what the word says is said by that rung."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  assert [(one[1], one[2]) for one in said(log, "bash")] == [("bash1", "rung1")] and engine.site.get() == OPERATOR
