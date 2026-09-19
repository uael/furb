"""World, the interface to the disk, the machine, the actors and the record."""

import asyncio
from asyncio import CancelledError
from functools import partial

from conftest import STANDS, Sand, Where, World, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD, Refused, Text

PING = "def ping(id):\n  while True:\n    yield\n\nclose(await act('ping', '', started(ping)))\n"


class Knows(Sand):
  """A World that knows one act an extension defines, and does it when a start names it."""

  def hears(self) -> World:
    """The World that answers a standing and an ask, and does the act of an extension that a start names."""
    made: dict[str, tuple] = {}
    loop = asyncio.get_running_loop()
    while True:
      a = yield
      match a:
        case (_, qid, *_) if engine.question(a) and qid in engine.acts:
          made[qid] = a
        case ("stand", qid, *_):
          self.calls.append(a)
          yield "done", qid, self.stands or ((), "", "")
        case ("start", about, _):
          self.calls.append(a)
          yield "done", about, f"did {made[about][0]}"
        case ("ask", rung, _, on, _, _) if self.script.get(on):
          self.calls.append(a)
          word = self.script[on].pop(0)
          turn = ("assistant", [word], (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
          loop.call_soon(partial(engine.send, "answer", rung, turn, by=WORLD))


async def test_the_world_hears_every_fact() -> None:
  """The World hears every fact: it answers a stand, a clock, a chance, a read and a write of a path nobody of the engine serves, resolved against the working directory it asks the chain for; it starts a command it is started with, asking it whether it is merged, feeds it, ends it at its timeout and at a cancel; it answers an ask with the turn of the model; and it shows a prompt to the operator."""
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
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a[4] for a in said(held, "merged")] == [command]
  assert engine.write(Text(f"{command}/stdin", "go"), on=root) == Text(f"{command}/stdin", "go")
  assert sand.fed == ["go"]
  engine.cancel(fed)
  await settle()
  assert isinstance(engine.outcomes[fed], CancelledError)
  short = engine.bash("slow", timeout=0.05, on=root)
  assert (await short).code is None
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  asking = engine.prompt(Text, "a text?", to=OPERATOR, on=root)
  await settle()
  assert isinstance(engine.outcomes[asking], Refused)
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
  assert [a[1] for a in said(sand.calls, "start")] == ["ping://operator.2.1.1"]


async def test_the_facts_that_the_world_says_of_its_own_are_for_the_acts_that_complete_later() -> None:
  """The facts that the World says of its own are for the acts that complete later."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  _, asked, *_ = said(log, "ask")[0]
  own = [a for a in log if a[2] == WORLD and a[0] != "done"]
  assert [a[0] for a in own] == ["answer", "out", "exited"]
  assert {a[1] for a in own} == {command, asked}


async def test_the_world_speaks_by_yielding_a_fact_or_by_calling_send_under_its_own_name() -> None:
  """The World speaks by yielding a fact, or by calling send under its own name when it speaks from its loop."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  _, standing, *_ = said(log, "stand")[0]
  assert [(a[0], a[1]) for a in log if a[2] == WORLD] == [("done", standing), ("out", act), ("exited", act)]
