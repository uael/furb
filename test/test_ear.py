"""Ear, any generator that hears the facts that it waits for and speaks by yielding one: the World and the Kernel among them."""

import shutil
from asyncio import CancelledError
from collections.abc import Generator
from dataclasses import dataclass, field
from pathlib import Path

import pytest

from conftest import (
  WORLD,
  Py,
  Sand,
  Where,
  World,
  born,
  dones,
  fresh,
  life,
  machine,
  paragraphs,
  prompted,
  ran,
  said,
  scratched,
  settle,
  slow,
  sown,
)
from furb import engine
from furb.engine import OPERATOR, Exit, Refused, Text
from furb_monty import _monty

PING = "def ping(id):\n  while True:\n    yield\n\nclose(await act('ping', '', ping))\n"


class Knows(Sand):
  """A World that knows one act an extension defines, and takes it when it is put to it."""

  def hear(self, a: tuple) -> World:
    """The World of the suite, which also does the act of an extension that it takes."""
    match a:
      case ("ping", about, *_):
        self.calls.append(a)
        yield "done", about, "did ping"
      case _:
        yield from super().hear(a)


@dataclass
class Answering(Sand):
  """A World of the suite that keeps each fact it said by yielding it, while an act was put to it."""

  now: list[tuple] = field(default_factory=list)

  def hear(self, a: tuple) -> World:
    """The World of the suite, which keeps each saying it yields as the bus said it."""
    inner = super().hear(a)
    try:
      saying = next(inner)
      while True:
        self.now.append(fact := (yield saying))
        saying = inner.send(fact)
    except StopIteration:
      return


async def test_the_world_hears_every_fact(tmp_path: Path) -> None:
  """The World, the ears of the outside that serve the machine, one or many, hears every fact that it waits for, every act that no ear before it took, and every chain at its birth: it answers a stand, a clock, a chance, a cd, a read and a write of a path nobody of the engine serves, resolved against the working directory of the chain, or against the scratch directory for a path of scratch://; it takes a command, asks it whether it is merged, answers a write of its doors and feeds it what a write of its stdin gives, and ends it at its timeout and at a cancel; it takes a wait and a prompt to the operator, which it shows; and it takes a reply, which it answers with the turn of the model."""
  sand, log, root = born(files={"/w/a.txt": "one\n"}, auto=False)
  assert engine.cwd(on=root) == ""
  assert engine.clock(on=root) == 1001.0
  assert engine.chance(on=root) == 2 / 7
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\n")
  assert engine.write(Text("b.txt", "two\n"), on=root) == Text("/w/b.txt", "two\n")
  fed = engine.bash("forever", stdin=None, on=root)
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
  assert engine.cd("sub", on=root) == "/w/sub"
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  asking = engine.prompt(Text, "a text?", to=OPERATOR, on=root)
  await settle()
  assert isinstance(engine.peek(asking), Refused)
  where = Where(files={"/x/a.txt": "three\n"})
  _, other = life(where)
  assert engine.cd("/x", on=other) == "/x"
  assert engine.read("a.txt", on=other) == Text("/x/a.txt", "three\n")
  assert (await engine.bash("echo hi", on=other)).code == 0
  assert where.where == ["/x"]
  live, root = machine(tmp_path)
  wrote = engine.write(Text("scratch://plan.md", "plan\n"), on=root)
  live.end()
  assert wrote == Text("scratch://plan.md", "plan\n")
  assert not (tmp_path / "scratch:").exists() and not (tmp_path / "plan.md").exists()


async def test_the_world_performs_any_fact_that_an_extension_defines_and_that_the_world_knows() -> None:
  """The World performs any fact that an extension defines and that the World knows."""
  sand = Knows()
  _, root = life(sand)
  sand.script[root] = [PING]
  assert await engine.prompt(str, "an act of my own", on=root) == "did ping"
  assert [a[1] for a in said(sand.calls, "ping")] == ["ping1"]


async def test_the_facts_that_the_world_says_of_its_own_are_for_the_acts_that_complete_later() -> None:
  """The facts that the World says of its own are for the acts that complete later."""
  world = Answering(files={"/w/a.txt": "one\ntwo\n"})
  log, root = life(world)
  world.script[root] = ["t = read('a.txt')\nx = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  _, asked, *_ = said(log, "reply")[0]
  own = [a for a in log if a[2] == WORLD and a[0] in ("out", "done") and a not in world.now]
  assert [(a[0], a[1]) for a in own] == [("done", asked), ("out", command), ("done", command)]
  assert {a[1] for a in own} <= {a[1] for a in world.now if a[0] == "started"}
  assert [(a[0], a[1]) for a in world.now if a[0] == "done"] == [("done", "stand1"), ("done", "read1")]


async def test_an_ear_speaks_by_yielding_a_saying_and_the_work_it_began_speaks_later_by_say() -> None:
  """An ear speaks by yielding a saying, and the work it began speaks later by say, under the site of the ear that began it."""
  _, log, root = born()
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  _, standing, *_ = said(log, "stand")[0]
  facts = [(a[0], a[1]) for a in log if a[2] == WORLD and engine.get(a[1]) != a]
  assert facts == [("done", standing), ("started", act), ("out", act), ("done", act)]


def note(kept: list[tuple]) -> Generator[tuple | None, tuple]:
  """An ear of the outside that answers a read of a door of its own and keeps every fact it hears."""
  while True:
    match a := (yield):
      case ("read", qid, _, _, path) if path.startswith("note://"):
        kept.append(a)
        yield "done", qid, Text(path, "kept")
      case (_, _, _, *_):
        kept.append(a)


async def test_an_ear_is_any_generator_of_that_shape() -> None:
  """An ear is any generator of that shape, so the World and the Kernel are ears, and boot takes an ear of the outside under any name it is to hear by."""
  sand = sown()
  kept: list[tuple] = []
  root = engine.boot((), world=sand.hears(), kernel=Py().kernel(), gate=Py().gating(), note=note(kept))
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  assert engine.read("note://one", on=root) == Text("note://one", "kept")
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "reply", "read", "read"]
  runs = [a[1] for a in kept if a[0] == "started" and engine.question(("run", a[1]))]
  told = fresh(root, prompted("prompt1", "int", "work"), "#rung1 advance on prompt1")
  assert [engine.get(run)[5] for run in runs] == [told, "close(1)"]
  assert [a[4] for a in kept if a[0] == "read"] == ["note://one"]
  assert {"started", "ready", "done"} <= {a[0] for a in kept} and not {"reply", "run"} & {a[0] for a in kept}


async def test_the_chain_has_the_gate_read_and_the_kernel_begin_every_rung() -> None:
  """The chain has the gate read every rung but the ones it wrote itself, and the Kernel begin every rung, by the acts gate and run."""
  sand, log, root = born()
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  told = [fresh(root), f"{prompted('prompt1', 'int', 'count')}\n\n#rung2 advance on prompt1"]
  assert ran(log) == [told[0], "k = 1", told[1], "close(k + 1)"]
  assert [a[4] for a in said(log, "run")] == ["rung1_told", "rung1", "rung2_told", "rung2"]
  assert [one[2] for one in said(log, "done") if one[1].startswith("gate")] == ["gate", "gate"]
  assert [a[4] for a in said(log, "gate")] == ["k = 1", "close(k + 1)"]


async def test_the_kernel_takes_a_run_as_that_run() -> None:
  """The Kernel takes a run as that run, begins its word in the module of the chain when it hears that it took it, makes a wants as the run when the word waits for an act that is not done, carries the word on at the done of that wants, and says the run done with what the word gave."""
  _, log, root = born("x = bash('echo hi')\nout = await x\nclose(out.code)")
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  runs = said(log, "run")
  assert [(a[1], a[4]) for a in runs] == [("run1", "rung1_told"), ("run2", "rung1")]
  assert [(a[0], a[2]) for a in log if a[1] == "run2" and a[0] in ("started", "done")] == [
    ("started", "run2"),
    ("done", "run2"),
  ]
  assert [(a[1], a[2], a[4]) for a in said(log, "wants")] == [("wants1", "run2", "bash1")]
  assert [(a[2], type(a[3]).__name__) for a in dones(log, "wants")] == [("rung1", "Exit")]
  assert [type(engine.peek(a[1])) for a in runs] == [type(None), CancelledError]
  out = engine.module(root)["out"]
  assert isinstance(out, Exit) and out.code == 0


async def test_the_kernel_sets_the_site_to_the_rung_whose_word_it_steps() -> None:
  """The Kernel sets the site to the rung whose word it steps, for as long as it steps it, so what the word says is said by that rung."""
  _, log, root = born("x = bash('echo hi')\nclose((await x).code)")
  assert await engine.prompt(int, "run it", on=root) == 0
  assert [(one[1], one[2]) for one in said(log, "bash")] == [("bash1", "rung1")] and engine.site.get() == OPERATOR


async def test_the_kernel_holds_the_run_of_a_word_while_a_command_that_it_did_not_await_is_open() -> None:
  """The Kernel holds the run of the word of a turn of a model, once that word is over, while a command that the word started and did not await is open, for 30 seconds at the most, by a wait that the run makes, so the chain tells what the command gave before it asks the next reply."""
  sand, log, root = born("x = bash('slow')", "close(1)", "y = bash('slow')", auto=False)
  asking = engine.prompt(int, "run it", on=root)
  while "bash1" not in [a[1] for a in said(log, "bash")]:
    await settle()
  await settle()
  assert [(a[1], a[2], a[4:]) for a in said(log, "wait")] == [("wait1", "run2", (30.0, "bash1"))]
  assert engine.peek("run2", ...) is ... and len(said(log, "reply")) == 1
  sand.exits("bash1", 0)
  assert await asking == 1
  told = [one.split("\n")[0] for one in paragraphs(engine.turns(on=root))]
  assert told[told.index("#bash1 exited 0") + 1] == "#rung2 advance on prompt1"
  again = engine.prompt(int, "run more", on=root)
  while "bash2" not in [a[1] for a in said(log, "bash")]:
    await settle()
  await settle()
  (held,) = [a[1] for a in said(log, "rung") if a[2] == again]
  engine.cancel(held)
  await settle()
  assert [engine.peek(a[1], ...) for a in said(log, "run") if a[4] == held] == [None]


async def test_the_opening_of_a_life_gives_the_world_the_scratch_directory_of_its_record(tmp_path: Path) -> None:
  """The opening of a life gives the World the scratch directory of its record, which every chain of that record shares, or, for a life that keeps no record, a scratch directory of its own, which goes when that life ends."""
  one, two = tmp_path / "one.jsonl", tmp_path / "two.jsonl"
  live, root = machine(tmp_path, one)
  other = engine.chain("other", on=root)
  engine.write(Text("scratch://plan.md", "one\n"), on=other)
  shared = engine.read("scratch://plan.md", on=root).content
  live.end()
  live, root = machine(tmp_path, two)
  with pytest.raises(Refused, match=r"^There is no file at scratch://plan\.md\.$"):
    engine.read("scratch://plan.md", on=root)
  live.end()
  live, root = machine(tmp_path)
  await engine.bash('echo own > "$TMPDIR/own.txt"', on=root)
  own = await scratched(root)
  held = (own / "own.txt").read_text()
  live.end()
  assert (shared, held) == ("one\n", "own\n")
  assert not own.exists()


async def test_a_later_life_on_a_record_finds_its_scratch_directory_again(tmp_path: Path) -> None:
  """A later life on a record finds its scratch directory again, and when the directory is missing, as on another machine, the opening makes again in it each file that a write of the record made, in the order of the last write of each, with the text that the journal kept, and no file that a command made; the opening of a new record starts with an empty directory."""
  record = tmp_path / "record.jsonl"
  live, root = machine(tmp_path, record)
  engine.write(Text("scratch://plan.md", "one\n"), on=root)
  await engine.bash('echo made > "$TMPDIR/made.txt"', on=root)
  live.end()
  live, root = machine(tmp_path, record)
  found = engine.read("scratch://made.txt", on=root).content
  engine.write(Text("scratch://a", "a file\n"), on=root)
  await engine.bash('rm "$TMPDIR/a"', on=root)
  engine.write(Text("scratch://a/b.md", "in a folder\n"), on=root)
  engine.write(Text("scratch://plan.md", "two\n"), on=root)
  live.end()
  shutil.rmtree(_monty.scratch(str(record)) or tmp_path / "none")
  live, root = machine(tmp_path, record)
  remade = [engine.read(f"scratch://{path}", on=root).content for path in ("", "plan.md", "a/b.md")]
  live.end()
  record.unlink()
  live, root = machine(tmp_path, record)
  empty = engine.read("scratch://", on=root).content
  live.end()
  assert found == "made\n"
  assert remade == ["a/\nplan.md", "two\n", "in a folder\n"]
  assert empty == ""


def listening(heard: list[tuple], names: set[str]) -> Generator[set[str], tuple]:
  """An ear that waits for these names and keeps each fact it hears."""
  while True:
    heard.append((yield names))


async def test_an_ear_that_yields_a_set_of_names_waits_for_the_facts_that_name_one_of_them() -> None:
  """An ear that yields a set of names waits for the facts that name one of them: by the kind of the fact, the act that it is about, or the kind, the maker or the chain of that act, or the chain of the maker of that act."""
  _, _, root = born()
  other = engine.chain("other", on=root)
  heard: list[tuple] = []
  engine.drive(listening(heard, {other, "clock"}), "listener")
  engine.read("a.txt", on=root)
  engine.read("a.txt", on=other)
  engine.clock(on=root)
  await settle()
  assert [(a[0], engine.get(a[1])[0], engine.scope(a[1])) for a in heard] == [
    ("read", "read", other),
    ("done", "read", other),
    ("clock", "clock", root),
    ("done", "clock", root),
  ]


async def test_an_ear_that_yields_nothing_or_an_empty_set_waits_for_every_fact() -> None:
  """An ear that yields nothing, or an empty set, waits for every fact."""
  _, _, root = born()
  every: list[tuple] = []
  engine.drive(listening(every, set()), "every")
  read = engine.act("read", root, None, "a.txt")
  await settle()
  assert [a[:2] for a in every if a[1] == read] == [("read", read), ("done", read)]
  assert {a[0] for a in every} > {"read", "done"}


async def test_every_ear_hears_every_pause_wake_cancel_and_close() -> None:
  """Every ear hears every pause, wake, cancel and close, whatever it waits for, so a control reaches each act that it is over."""
  _, _, _, act = slow()
  heard: list[tuple] = []
  engine.drive(listening(heard, {"none"}), "listener")
  engine.pause(act)
  engine.wake(act)
  engine.cancel(act)
  await settle()
  assert [a[:2] for a in heard if a[0] in ("pause", "wake", "cancel", "close")] == [
    ("pause", act),
    ("wake", act),
    ("cancel", act),
  ]
