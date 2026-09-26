"""act, the way to make a question."""

from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, keeping, kernel, life, paragraphs, plain, relived, said, settle, sown, watched
from furb import engine
from furb.engine import HIDDEN, OPERATOR, TIMEOUT, Act, Exit, Refused, Text, take


def noting(heard: list[object], how: str = "", *, takes: bool = True):  # noqa: ANN201
  """A life of the suite: it keeps the name it is given and every fact it hears, takes its act when it is told to,
  and says one fact by yielding it when it is given a way."""

  def lives(id: str) -> Generator[tuple | None, tuple]:
    heard.append(id)
    if takes:
      yield "started", id
    if how:
      yield "tell", id, [f"#{id} {how} by yield"]
    while True:
      if (a := (yield)) is not None:
        heard.append(a)

  return lives


def taking(kind: str) -> Generator[tuple | None, tuple]:
  """An ear of the outside that takes every act of one kind it is put, and hears every other fact."""
  while True:
    match a := (yield):
      case (str(one), about, *_) if one == kind and engine.get(about) == a:
        yield "started", about


async def test_the_way_to_make_a_question() -> None:
  """The way to make a question: it takes a name when it is made, it is put to the ears, its ear, when it has one, is brought to life under that name and given the name, and the name is given back, which is the act to whoever holds it."""
  sand = sown()
  log, root = life(sand)
  heard: list[object] = []
  one = engine.act("note", root, noting(heard), "one")
  assert isinstance(one, Act) and one == "note1"
  assert said(log, "note") == [("note", one, OPERATOR, root, "one")]
  assert engine.get(one) == ("note", one, OPERATOR, root, "one")
  assert heard[0] == one
  two = engine.act("read", root, None, "a.txt")
  assert two == "read1" and engine.get(two) == ("read", two, OPERATOR, root, "a.txt")
  assert engine.peek(two) == Text("/w/a.txt", "one\ntwo\n")


async def test_an_act_is_put_to_every_ear_of_the_engine_the_acts_first() -> None:
  """An act is put to every ear of the engine, the acts first, and a busy one hears it once it is done speaking, at its place among what was said."""
  sand = sown()
  _, root = life(sand)
  heard: list[tuple] = []
  made: list[str] = []

  def asking() -> Generator[tuple | None, tuple]:
    while True:
      heard.append(a := (yield))
      if a[0] == "tell" and not made:
        made.append(engine.act("read", root, None, "a.txt"))
        yield "tell", root, ["#chain1 asked"]

  engine.drive(asking(), "asking")
  engine.say("tell", root, ["#chain1 hello"])
  kinds = [(a[0], a[1]) for a in heard if a[0] != "keep"]
  assert kinds[:4] == [("tell", root), ("read", made[0]), ("done", made[0]), ("tell", root)]
  engine.bash("echo hi", on=root)
  assert ("bash", "bash1") not in kinds and [a[1] for a in heard if a[0] == "bash"] == ["bash1"]


async def test_its_own_ear_is_born_after_the_ears_of_the_engine_heard_it() -> None:
  """Its own ear is born after the ears of the engine heard it, so the acts that ear makes stand after it."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["close(1)"]
  asked = engine.prompt(int, "one", on=root)
  assert await asked == 1
  kinds = [(a[0], a[1]) for a in engine.transcript(root) if engine.get(a[1]) == a]
  assert kinds.index(("prompt", asked)) < kinds.index(("rung", "rung1"))


async def test_it_is_then_put_to_the_ears_of_the_outside_in_turn_until_one_takes_it() -> None:
  """It is then put to the ears of the outside in turn, until one takes it, and no ear of the outside after that one hears it."""
  sand = Sand(stands=STANDS)
  log: list[tuple] = []
  after: list[tuple] = []
  root = engine.boot((), **kernel(), probe=watched(log), taker=taking("note"), after=keeping(after), world=sand.hears())
  one = engine.act("note", root, noting([], takes=False))
  assert engine.peek(one, ...) is ... and [a for a in after if a[0] == "note"] == []
  two = engine.act("seen", root, None)
  assert [a for a in after if a[0] == "seen"] == [engine.get(two)]


async def test_to_take_an_act_is_to_say_a_started_or_a_done_about_it() -> None:
  """To take an act is to say a started or a done about it: a done settles it now, and a started says that its done comes later."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS, auto=False)
  log, root = life(sand)
  read = engine.act("read", root, None, "a.txt")
  assert said(log, "done")[-1] == ("done", read, "world", Text("/w/a.txt", "one\n")) and engine.peek(read) == Text(
    "/w/a.txt", "one\n"
  )
  command = engine.bash("slow", on=root)
  assert said(log, "started")[-1] == ("started", command, "world") and engine.peek(command, ...) is ...


async def test_an_act_that_no_ear_takes_is_refused() -> None:
  """An act that no ear takes and that the record does not hold is refused: the life says it done with a refusal that names its kind."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  one = engine.act("note", root, noting([], takes=False))
  got = engine.peek(one)
  assert isinstance(got, Refused) and str(got) == "nothing takes note"
  assert said(log, "done")[-1] == ("done", one, one, got)
  with pytest.raises(Refused, match="nothing takes nothing"):
    engine.ask("nothing", root)


async def test_an_act_said_it_is_begun_and_what_the_call_gives_is_its_name() -> None:
  """An act said: it is begun, and what the call gives is its name, which is awaited for what the act comes to."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert isinstance(one, Act) and engine.peek(one) is None
  assert paragraphs(engine.turns(on=root))[-1] == "#bash1 echo hi\nbash1: Act[Exit] = Act('bash1')"
  assert (await one).code == 0


async def test_an_act_said_twice_under_one_name_is_one_act() -> None:
  """An act said twice under one name is one act, and the second saying brings no second ear and gives the name back."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  first = engine.rung("close(bash('echo hi'))", on=root)
  again = engine.rung("close(bash('echo hi'))", retells=first, on=root)
  assert (await first) == "bash1" and (await again) is None
  assert len(said(log, "bash")) == 1
  assert [a[1] for a in sand.calls if a[0] == "bash"] == ["bash1"]


async def test_two_acts_that_say_the_same_words_under_one_name_are_one_act() -> None:
  """Two acts that say the same words under one name are one act."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  first = engine.rung("close(bash('echo hi'))", on=root)
  again = engine.rung("close(bash('echo hi'))", retells=first, on=root)
  one = await first
  assert isinstance(one, str)
  assert (await again) is None
  assert engine.get(one) == ("bash", one, first, root, "echo hi", False, TIMEOUT)
  await settle()
  got = engine.peek(one)
  assert got == engine.peek(one) and isinstance(got, Exit) and got.code == 0
  assert [e[0][1] for e in sand.record if e[0][0] == "bash"] == [one]


async def test_the_engine_refuses_an_act_said_from_outside_a_run_that_names_no_chain() -> None:
  """The engine refuses an act said from outside a run that names no chain, a chain apart."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  heard: list[object] = []
  with pytest.raises(Refused, match="no chain"):
    engine.act("note", "", noting(heard))
  assert said(log, "note") == [] and heard == []
  assert engine.chain("two") == "chain2"


async def test_the_chain_an_act_is_on_is_the_chain_named_to_the_call() -> None:
  """The chain an act is on is the chain named to the call, or the scope of the one that made it when the call names none."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  heard: list[object] = []
  named = engine.act("note", two, noting(heard), "one")
  assert engine.get(named)[3] == two
  sand.script[root] = ["close(bash('echo hi'))"]
  unsaid = await engine.prompt(str, "start one", on=root)
  assert engine.get(unsaid)[3] == root


async def test_the_ear_of_an_act_is_given_the_name_of_the_act_and_hears_every_fact_said_after_its_birth() -> None:
  """The ear of an act is given the name of the act and hears every fact said after its birth, and it speaks by yielding a saying."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  heard: list[object] = []
  one = engine.act("note", root, noting(heard, "spoke"), "one")
  two = engine.bash("echo hi", on=root)
  await settle()
  assert heard[0] == one and ("bash", two, OPERATOR, root, "echo hi", False, TIMEOUT) in heard[1:]
  assert engine.get(one) not in heard
  assert paragraphs(engine.turns(on=root))[2:] == [
    "#note1 spoke by yield",
    "#bash1 echo hi\nbash1: Act[Exit] = Act('bash1')",
    "#bash1 exited 0\n# bash1/stdout, 0 known\n# 1 ran echo hi",
  ]


async def test_an_act_carries_the_words_of_its_kind() -> None:
  """An act carries the words of its kind, which are the plain arguments the verb was given, in the order of the verb, and a show or a filter is none of them."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", True, 5.0, HIDDEN, HIDDEN, on=root)
  assert engine.get(one) == ("bash", one, OPERATOR, root, "echo hi", True, 5.0)
  two = engine.chain("two", source=root, filter=take(root))
  assert engine.get(two) == ("chain", two, OPERATOR, "", "two", root)
  sand.files["/w/a.txt"] = "one\n"
  sand.script[root] = ["write(read('a.txt').replace('one', 'uno'))\nclose(1)"]
  assert await engine.prompt(int, "edit", on=root) == 1
  (edit,) = said(sand.calls, "write")
  assert edit[4] == Text("/w/a.txt", "uno\n") and edit[4].before is None
  await relived(Sand(files={"/w/a.txt": "one\n"}, stands=STANDS), plain(sand.record))
  assert engine.peek(edit[1]) == Text("/w/a.txt", "uno\n")
