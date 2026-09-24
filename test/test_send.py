"""send, the bus: the one way a fact is said to the living."""

from conftest import (
  DOOR,
  STANDS,
  Dead,
  Sand,
  findings,
  keeping,
  life,
  lived,
  pair,
  plain,
  ran,
  relived,
  said,
  settle,
  sown,
)
from furb import engine
from furb.engine import OPERATOR, WORLD, Exit, Text

MINE = "def read(path, **kw):\n  return ask('read', __name__, 'mine.txt')[1]"


async def test_the_one_way_to_speak_of_an_act() -> None:
  """The one way to speak of an act: a fact about it is said to the living, whole as the bus made it, and given back."""
  sand = sown()
  log, root = life(sand)
  made = engine.send("tell", root, ["#chain1 noted"])
  assert made == ("tell", "chain1", OPERATOR, ["#chain1 noted"])
  assert said(log, "tell")[-1] == made


async def test_every_verb_of_the_file_speaks_through_the_three_entries_of_the_bus() -> None:
  """Every verb of the file, and every verb of an extension, speaks through the three entries of the bus: send for a fact, ask for a query and act for an act."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = [DOOR, "close(None)"]
  assert await engine.prompt(int, "a door", on=root) == 1
  await settle()
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  engine.close(None, root)
  assert [a[4] for a in said(log, "read")] == ["a.txt"]
  assert [a[4] for a in said(log, "bash")] == ["echo hi"]
  assert said(log, "close")[-1][1] == root
  assert [a[1] for a in said(log, "note")] == ["note1"]


async def test_a_fact_reaches_the_world_the_kernel_and_the_record_only_through_the_bus() -> None:
  """A fact reaches the World, the Kernel and the record only through the bus."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(read('a.txt').content)"]
  assert await engine.prompt(str, "read it", on=root) == "one\ntwo\n"
  assert [a[0] for a in sand.calls] == ["stand", "ask", "read"]
  assert findings(log) == [[]]
  assert ran(log) == [
    "chain1: Act[object] = Act('chain1')\nprompt1: Act[str] = Act('prompt1')",
    "close(read('a.txt').content)",
  ]
  assert [entry[0][4] for entry in sand.record if entry[0][0] == "read"] == ["a.txt"]


async def test_a_rebound_verb_reaches_the_world_only_through_the_bus() -> None:
  """A rebound verb reaches the World only through the bus."""
  sand = Sand(files={"/w/mine.txt": "mine\n"}, stands=STANDS)
  _, root = life(sand)
  assert await engine.rung(MINE, on=root) is None
  sand.script[root] = ["close(read('any.txt').content)"]
  assert await engine.prompt(str, "read it", on=root) == "mine\n"
  assert [a[4] for a in said(sand.calls, "read")] == ["mine.txt"]


async def test_who_says_it_is_whoever_is_speaking() -> None:
  """Who says it is whoever is speaking, unless the one that says it is outside and names itself."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  assert await engine.prompt(int, "start one", on=root) == 1
  await settle()
  step, command = said(log, "rung")[0], said(log, "bash")[0]
  assert (command[2], step[1], step[2]) == ("rung1", "rung1", "prompt1")
  engine.send("out", command[1], "hi\n", "stdout", by=WORLD)
  assert said(log, "out")[-1][2] == WORLD
  engine.read("a.txt", on=root)
  assert said(log, "read")[-1][2] == OPERATOR


async def test_a_fact_said_it_says_its_kind_the_act_it_is_about_who_said_it_and_its_words() -> None:
  """A fact said: it says its kind, the act it is about, who said it and its words, in that order, and nothing else, since the chain it is on is the scope of the act it is about."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert said(log, "bash")[0] == ("bash", act, OPERATOR, root, "echo hi", False, 600.0)
  assert engine.scope(act) == root
  engine.cancel(act)
  over = said(log, "cancel")[0]
  assert over == ("cancel", "bash1", OPERATOR, ["#bash1 cancelled"])
  assert [entry for entry in sand.record if entry[0][0] == "bash"] == [(said(log, "bash")[0],)]


async def test_the_bus_makes_every_fact_whole_from_what_it_is_given() -> None:
  """The bus makes every fact whole from what it is given, so nobody holds a fact that is not whole."""
  sand = sown()
  log, root = life(sand)
  made = engine.send("tell", root, ["#chain1 noted"])
  assert (made[0], made[1], made[2]) == ("tell", "chain1", OPERATOR)
  asking, got = engine.ask("read", root, "a.txt")
  assert asking == ("read", "read@operator.2", OPERATOR, "chain1", "a.txt")
  assert got == Text("/w/a.txt", "one\ntwo\n")
  act = engine.bash("echo hi", on=root)
  assert said(log, "bash")[0] == ("bash", act, OPERATOR, root, "echo hi", False, 600.0)
  assert all(len(a) >= 3 and a[2] for a in log)


async def test_every_generator_the_acts_first_then_those_of_the_engine_then_those_from_the_outside() -> None:
  """Every generator, the acts first, then those of the engine, then those the life was given from the outside, since the engine settles what it knows before the outside reads it or acts on it, and it asks the outside for nothing it can answer itself."""
  sand = sown()
  log, root = await lived(sand)
  _, command, *_ = said(log, "bash")[0]
  before = len(said(sand.calls, "read"))
  assert engine.read(f"{command}/stdout", on=root).content == "ran echo hi\n"
  assert len(said(sand.calls, "read")) == before
  assert engine.read("a.txt", on=root).content == "one\ntwo\n"
  assert len(said(sand.calls, "read")) == before + 1
  dead = Dead(stands=STANDS)
  _, over = await relived(dead, plain(sand.record))
  text = engine.modules[over]["t"]
  assert isinstance(text, Text) and text.content == "one\ntwo\n" and said(dead.calls, "read") == []


async def test_every_generator_hears_every_fact_it_has_not_heard_in_order_until_none_is_left() -> None:
  """Every generator hears every fact it has not heard, in order, until none is left."""
  sand = sown()
  log, root = life(sand)
  heard: list[tuple] = []
  at = len(log)
  engine.drive(keeping(heard), "keeper")
  engine.bash("echo hi", on=root)
  await settle()
  assert heard == log[at:] and len(heard) > 1


async def test_while_one_speaks_nobody_hears() -> None:
  """While one speaks nobody hears, and whoever spoke has everyone hear when it is done, so the facts of one speaker stand together in the log, and no one is having everyone hear while another is."""
  sand = sown()
  log, _ = life(sand)
  engine.drive(pair(), "pair")
  places = [i for i, a in enumerate(log) if a[1].startswith("none://")]
  assert len(places) == 2 and places[1] == places[0] + 1


async def test_a_done_said_of_a_question_that_has_no_outcome_yet_fills_its_outcome() -> None:
  """A done said of a question that has no outcome yet fills its outcome, and a later done of the same question fills nothing."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  assert act not in engine.outcomes
  engine.send("exited", act, 3, by=WORLD)
  await settle()
  got = engine.outcomes[act]
  assert isinstance(got, Exit) and got.code == 3
  engine.send("done", act, 9, by=WORLD)
  assert engine.outcomes[act] == got
