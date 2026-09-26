"""boot, the life: everything that is said in it is said there."""

from collections.abc import Generator

import pytest

from conftest import (
  DOOR,
  WORD,
  WORLD,
  Dead,
  Sand,
  acts,
  gated,
  heads,
  keeping,
  life,
  lived,
  outside,
  pair,
  paragraphs,
  plain,
  ran,
  relived,
  said,
  settle,
  sown,
  tip,
  world_says,
)
from furb import engine
from furb.engine import HIDDEN, OPERATOR, Drift, Exit, Refused, Text, idle, take


def made(log: list[tuple]) -> list[str]:
  """The name of every act of a life, as the log of that life says them."""
  return [a[1] for a in log if engine.question(a)]


def ours(log: list[tuple]) -> dict[str, tuple]:
  """Every act of the life that the operator or an act made, under its name: the World asks its own questions again
  only when it needs them."""
  return {name: a for name, a in acts(log).items() if a[2] != WORLD}


def watching(root: str) -> Generator[tuple | None, tuple]:
  """An ear that the operator drives, which makes a command on the root when the operator says go there."""
  while (yield)[3:] != ([f"#{root} go"],):
    pass
  engine.bash("echo seen", on=root)
  while True:
    yield


async def test_a_life_everything_that_is_said_in_it_is_said_here() -> None:
  """A life: everything that is said in it is said here, so the log of what was said, the generators that listen by their names and the tables of the life are its own, and it binds the names that reach them: say, which says a fact, act, which makes an act, drive, which brings a generator to life, and get, peek and transcript, which read the tables."""
  one = sown()
  _, root = await lived(one)
  heard: list[tuple] = []
  its: list[tuple] = []
  engine.drive(keeping(heard), "keeper")
  told = engine.say("tell", root, [f"#{root} noted here"])
  name = engine.act("note", root, lambda one: keeping(its, ("done", one, "one")), "one")
  assert engine.get(name) == ("note", name, OPERATOR, root, "one") and engine.peek(name) == "one"
  assert [a for a in heard if a[0] in ("tell", "done", "note")] == [told, engine.get(name), ("done", name, name, "one")]
  assert told in engine.transcript(root) and ("done", name, name, "one") in its
  two = sown()
  second, over = await relived(two, [])
  assert root == over and said(second, "note") == [] and engine.get(name) is None
  engine.bash("echo more", on=over)
  assert len(said(second, "bash")) == 1 and said(heard, "bash") == []


async def test_a_verb_from_a_rung_takes_the_chain_of_the_rung_when_the_call_leaves_on_unsaid() -> None:
  """A verb from a rung takes the chain of the rung when the call leaves on unsaid."""
  sand = sown()
  log, _ = life(sand)
  two = engine.chain("two")
  sand.script[two] = ["x = bash('echo hi')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=two) == 1
  assert [a[3] for a in said(log, "bash")] == [two]


async def test_the_kind_of_an_act_is_the_verb_that_made_the_act_or_the_kind_an_ear_made_it_with() -> None:
  """The kind of an act is the verb that made the act, or the kind an ear made it with."""
  sand = sown()
  log, _ = await lived(sand)
  kinds = {"chain", "stand", "prompt", "rung", "reply", "gate", "run", "read", "bash", "wants", "merged"}
  assert {engine.get(name)[0] for name in made(log)} == kinds and engine.get("merged1")[2] == WORLD
  assert [name for name in made(log) if name.rstrip("0123456789") != engine.get(name)[0]] == []


async def test_an_act_says_who_made_it() -> None:
  """An act says who made it, and the one that made it says its own maker in turn, which is how the life knows every ancestor of an act."""
  sand = sown()
  log, _ = await lived(sand)
  one, step, command = said(log, "prompt")[0], said(log, "rung")[0], said(log, "bash")[0]
  assert (one[2], step[2], command[2]) == (OPERATOR, one[1], step[1])
  assert engine.under(command[1], one[1]) and engine.under(command[1], OPERATOR)


async def test_an_act_that_the_operator_made_has_the_operator_for_its_maker() -> None:
  """An act that the operator made has the operator for its maker."""
  sand = sown()
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert (root, one) == ("chain1", "bash1")
  assert engine.get(root)[2] == engine.get(one)[2] == OPERATOR


async def test_a_name_is_never_reused_in_the_record() -> None:
  """A name is never reused in the record."""
  sand = sown()
  await lived(sand)
  names = [e[0][1] for e in sand.record if engine.question(e[0])]
  assert names and len(names) == len(set(names))


async def test_a_later_life_gives_the_same_names() -> None:
  """A later life gives the same names, since the same acts make them again."""
  sand = sown()
  first, root = await lived(sand)
  was = ours(first)
  again, over = await relived(Sand(), list(sand.record))
  assert ours(again) == {**was, "stand2": ("stand", "stand2", OPERATOR, root)} and over == root


async def test_the_root_has_no_parent() -> None:
  """The root has no parent."""
  sand = sown()
  log, root = await lived(sand)
  assert said(log, "chain")[0][2] == OPERATOR
  assert [name for name in made(log) if name != root and engine.under(root, name)] == []


async def test_a_question_goes_to_the_living_acts_of_the_engine_before_it_goes_to_the_world() -> None:
  """A question goes to the living acts of the engine before it goes to the World, since the engine settles what it knows before the outside reads it, and asks the outside for nothing that the engine can answer itself."""
  sand = sown()
  log, root = await lived(sand)
  command = said(log, "bash")[0][1]
  before = len([a for a in sand.calls if a[0] == "read"])
  assert engine.read(f"{command}/stdout", on=root).content == "ran echo hi\n"
  assert len([a for a in sand.calls if a[0] == "read"]) == before
  dead = Dead()
  _, over = await relived(dead, list(sand.record))
  assert engine.module(over)["k"] == 2 and [a for a in dead.calls if a[0] == "read"] == []


async def test_a_later_life_makes_a_door_again_from_the_word_of_the_rung_that_defined_it() -> None:
  """A later life makes a door again from the word of the rung that defined it."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [DOOR]
  assert await engine.prompt(int, "a door", on=root) == 1
  assert engine.read("note://a", on=root).content == "kept"
  _, over = await relived(Sand(), list(sand.record))
  assert engine.read("note://a", on=over).content == "kept"


async def test_the_record_that_boot_is_given_enters_nothing_in_the_record() -> None:
  """The record that boot is given enters nothing in the record, since the record is what boot is given."""
  sand = sown()
  _, root = await lived(sand)
  later = Sand()
  await relived(later, list(sand.record))
  assert later.record == tip("stand2", root)


async def test_an_act_takes_its_name_when_the_act_opens_and_the_name_says_what_made_the_act() -> None:
  """An act takes its name when the act opens, and the name says what made the act."""
  sand = sown()
  log, root = await lived(sand)
  command, step = said(log, "bash")[0], said(log, "rung")[0]
  assert command[2] == step[1] and engine.under(command[1], step[1])
  assert f"#{command[1]} echo hi" in heads(engine.turns(on=root))


async def test_the_engine_derives_the_transcripts_the_turns_the_globals_and_the_working_directories() -> None:
  """The engine derives the transcripts, the turns, the globals and the working directories from the record."""
  sand = sown()
  _, root = await lived(sand)
  await engine.rung("cd('/deep')", on=root)
  held = [x for x in made(engine.transcript(root)) if engine.get(x)[2] != WORLD]
  told = paragraphs(engine.turns(on=root))
  _, over = await relived(Sand(), list(sand.record))
  assert [x for x in made(engine.transcript(over)) if engine.get(x)[2] != WORLD] == [*held, "stand2"]
  assert all(one in paragraphs(engine.turns(on=over)) for one in told)
  assert engine.module(over)["k"] == 2 and engine.cwd(on=over) == "/deep"


async def test_a_fact_the_journal_says_again_is_the_journals_own() -> None:
  """A fact the journal says again is the journal's own, as is an answer it says again when an act is made again."""
  sand = sown()
  await lived(sand)
  again, _ = await relived(Sand(), list(sand.record))
  kept = [(e[0][0], e[0][1], *e[0][3:]) for e in sand.record if e[0][:2] in (("out", "bash1"), ("done", "bash1"))]
  copies = [a for a in again if a[:2] in (("out", "bash1"), ("done", "bash1"))]
  assert [(a[0], a[1], *a[3:]) for a in copies] == kept and [a[2] for a in copies] == ["journal"] * 2
  answers = [a for a in again if a[0] in ("started", "done") and a[1] in ("read1", "reply1", "gate1")]
  assert [(a[0], a[1], a[2]) for a in answers] == [
    ("done", "reply1", "journal"),
    ("done", "gate1", "journal"),
    ("done", "read1", "journal"),
  ]


async def test_what_the_module_holds_is_not_in_the_record() -> None:
  """What the module holds is not in the record."""
  sand = sown()
  _, root = await lived(sand)
  assert engine.module(root)["k"] == 2
  assert [e for e in sand.record if 2 in e[0][3:]] == []


async def test_the_root_is_the_first_act_of_the_record() -> None:
  """The root is the first act of the record."""
  sand = sown()
  _, root = await lived(sand)
  assert sand.record[0][0][:2] == ("chain", root)


async def test_a_later_life_on_a_kept_record_makes_the_root_again_and_enters_no_second_root() -> None:
  """A later life on a kept record makes the root again and enters no second root."""
  sand = sown()
  _, root = await lived(sand)
  later = Sand()
  again, over = await relived(later, list(sand.record))
  assert over == root and [a[1] for a in said(again, "chain")] == [root]
  assert [e for e in later.record if e[0][0] == "chain"] == []


async def test_the_engine_appends_after_the_last_entry_of_the_record_boot_was_given() -> None:
  """The engine appends after the last entry of the record boot was given."""
  sand = sown()
  await lived(sand)
  old = list(sand.record)
  later = Sand(files={"/w/a.txt": "one\ntwo\n"}, record=list(old))
  _, over = await relived(later, old)
  later.script[over] = ["close(9)"]
  assert await engine.prompt(int, "more", on=over) == 9
  await settle()
  assert later.record[: len(old)] == old and len(later.record) > len(old)


async def test_what_the_word_of_a_rung_made_or_computed_a_later_life_makes_again_by_running_the_word() -> None:
  """What the word of a rung made or computed, a later life makes again by running the word."""
  sand = sown()
  await lived(sand)
  _, over = await relived(Dead(), list(sand.record))
  assert engine.module(over)["k"] == 2 and engine.module(over)["t"] == Text("/w/a.txt", "one\ntwo\n")


async def test_the_engine_makes_a_chain_from_the_record_and_in_no_other_way() -> None:
  """The engine makes a chain from the record and in no other way, by running its rungs again."""
  sand = sown()
  log, root = await lived(sand)
  was = dict(engine.program(root))
  again, over = await relived(Sand(), list(sand.record))
  assert ran(again) == ran(log)
  assert [one for one in ran(again) if ": Act[" not in one] == [WORD, "close(None)"]
  assert engine.program(over) == was


async def test_the_rungs_of_the_ladder_run_in_record_order() -> None:
  """The rungs of a chain run in record order."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = 1", "b = a + 1", "close(b)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  again, _ = await relived(Sand(), list(sand.record))
  bound = f"{root}: Act[object] = Act('{root}')\nprompt1: Act[int] = Act('prompt1')"
  assert ran(again) == [bound, "a = 1", "b = a + 1", "close(b)"]


async def test_each_act_a_rung_makes_again_is_the_act_the_record_holds_at_that_place() -> None:
  """Each act a rung makes again is the act the record holds at that place, with its result."""
  sand = sown()
  log, _ = await lived(sand)
  command = said(log, "bash")[0][1]
  again, _ = await relived(Sand(), list(sand.record))
  assert [a[1] for a in said(again, "bash")] == [command]
  got = engine.peek(command)
  assert isinstance(got, Exit) and got.code == 0


async def test_a_replay_makes_the_same_acts_in_the_same_order_and_gives_them_the_same_ids() -> None:
  """A replay makes the same acts in the same order and gives them the same ids."""
  sand = sown()
  first, root = await lived(sand)
  was = ours(first)
  again, _ = await relived(Sand(), list(sand.record))
  assert ours(again) == {**was, "stand2": ("stand", "stand2", OPERATOR, root)}


async def test_the_journal_answers_what_the_record_holds_an_answer_for_the_gate_among_them() -> None:
  """The journal answers what the record holds an answer for, the gate among them, so the outside is asked nothing it answered once."""
  sand = sown()
  await lived(sand)
  later = Sand()
  again, _ = await relived(later, list(sand.record))
  assert [a[0] for a in later.calls] == ["stand"] and gated(again) == [WORD, "close(None)"]
  assert [a[2] for a in said(again, "done") if a[1].startswith("gate")] == ["journal", "journal"]


async def test_a_later_life_on_a_kept_record_starts_nothing_and_keeps_the_ids_of_the_earlier_life() -> None:
  """A later life on a kept record starts nothing and keeps the ids of the earlier life."""
  sand = sown()
  log, _ = await lived(sand)
  later = Sand()
  again, _ = await relived(later, list(sand.record))
  assert [a for a in said(again, "started") if a[2] == WORLD] == [] and [a[0] for a in later.calls] == ["stand"]
  assert [a[1] for a in said(again, "bash")] == [a[1] for a in said(log, "bash")]


async def test_in_a_later_life_the_ladder_of_every_chain_runs_again() -> None:
  """In a later life the rungs of every chain run again from the record that the World kept."""
  sand = sown()
  _, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = ["here = 1\nclose(1)", "close(None)"]
  sand.script[two] = ["there = 2\nclose(2)", "close(None)"]
  assert await engine.prompt(int, "one", on=root) == 1
  assert await engine.prompt(int, "two", on=two) == 2
  await settle()
  again, _ = await relived(Sand(), list(sand.record))
  assert sorted(ran(again)) == sorted(
    [
      f"{root}: Act[object] = Act('{root}')\nprompt1: Act[int] = Act('prompt1')",
      f"{two}: Act[object] = Act('{two}')\nprompt2: Act[int] = Act('prompt2')",
      "here = 1\nclose(1)",
      "there = 2\nclose(2)",
    ]
  )
  assert engine.module(root)["here"] == 1 and engine.module(two)["there"] == 2


async def test_the_engine_serves_the_doors_of_the_file_itself_and_asks_the_world_for_nothing() -> None:
  """The engine serves the doors of the file itself, and asks the World for nothing."""
  sand = sown()
  log, root = await lived(sand)
  command = said(log, "bash")[0][1]
  before = len(sand.calls)
  assert engine.read(f"{command}/stdout", on=root).content == "ran echo hi\n"
  assert engine.read(said(log, "prompt")[0][1], on=root).content == WORD
  assert engine.transcript(root)
  assert len(sand.calls) == before


async def test_a_later_life_reads_the_same_text_from_a_door() -> None:
  """A later life reads the same text from a door."""
  sand = sown()
  log, root = await lived(sand)
  command = said(log, "bash")[0][1]
  was = engine.read(f"{command}/stdout", on=root)
  _, over = await relived(Dead(), list(sand.record))
  assert engine.read(f"{command}/stdout", on=over) == was


async def test_it_is_given_what_the_world_kept_of_the_life_before_it() -> None:
  """It is given what the World kept of the life before it, and the generators of the outside, the Kernel, the gate and the World among them, each under the name it is to hear by, and it brings them to life with its own."""
  sand = sown()
  log, _ = await lived(sand)
  assert [a[2] for a in said(log, "done") if a[1] == "reply1"] == [WORLD]
  assert [a[2] for a in said(log, "done") if a[1].startswith("gate")][:1] == ["gate"]
  later = Sand()
  again, _ = await relived(later, list(sand.record))
  assert [a[1] for a in said(again, "bash")] == [a[1] for a in said(log, "bash")]


async def test_it_opens_the_root_the_first_act_of_any_record() -> None:
  """It opens the root, the first act of any record, which every life opens under the one name, and which a record that holds it already gives back, and that root is what it gives back."""
  sand = sown()
  log, root = await lived(sand)
  assert root == "chain1" and said(log, "chain")[0][1] == root
  again, over = await relived(Sand(), list(sand.record))
  assert over == root and [a[1] for a in said(again, "chain")] == [root]


async def test_what_the_record_holds_of_an_act_made_again_keeps_that_act_from_the_outside() -> None:
  """What the record holds of an act made again keeps that act from the outside: a done that is its first answer the journal says at once, and an act that the outside started the journal holds with no fact, since it cannot run it, so no ear of the outside hears that act, and a chain with a source which asks again what its origin asked is answered from the record too."""
  sand = sown()
  _, root = await lived(sand)
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.module(twin)["k"] == 2
  dead = Dead()
  again, _ = await relived(dead, list(sand.record))
  assert engine.module(root)["k"] == engine.module(twin)["k"] == 2
  assert [a[0] for a in dead.calls] == ["stand"] and dead.calls[0][2] == OPERATOR
  assert [(a[0], a[2]) for a in again if a[1] in ("bash1", "read1") and a[0] in ("started", "done")] == [
    ("done", "journal"),
    ("done", "journal"),
  ]


async def test_the_record_a_life_was_opened_from_from_which_the_journal_answers() -> None:
  """The record a life was opened from, from which the journal answers what it holds of an act, so that an act the World did once is done no more."""
  sand = sown()
  log, _ = await lived(sand)
  later = Sand(files={"/w/a.txt": "one\ntwo\n"})
  again, _ = await relived(later, list(sand.record))
  kept = [(e[0][0], e[0][1], *e[0][3:]) for e in sand.record if e[0][0] == "out"]
  assert [a[0] for a in later.calls] == ["stand"]
  assert [(a[0], a[1], *a[3:]) for a in said(again, "out")] == kept
  got = engine.peek(said(log, "bash")[0][1])
  assert isinstance(got, Exit) and got.code == 0


async def test_the_journal_an_ear_of_the_engine_which_hears_everything() -> None:
  """The journal: an ear of the engine, which hears everything and keeps every act that no act of the life made and every fact that no act of the life said, the act it is about before it, and nothing that a generator an act brought to life made or said, nor anything it says itself, since a later life makes again everything that an act made or said and brings that generator to life again, and what the journal says the record holds already."""
  sand = sown()
  log, _ = await lived(sand)
  assert {e[0][0] for e in sand.record} == {
    "chain",
    "stand",
    "prompt",
    "reply",
    "started",
    "gate",
    "done",
    "read",
    "bash",
    "merged",
    "out",
  }
  assert [e for e in sand.record if e[0][0] == "tell" or e[0][2] in (*acts(log), "journal")] == [
    e for e in sand.record if engine.question(e[0]) and e[0][2] in acts(log)
  ]
  names = [e[0][1] for e in sand.record if engine.question(e[0])]
  for i, e in enumerate(sand.record):
    if not engine.question(e[0]):
      assert names.index(e[0][1]) < len([one for one in sand.record[:i] if engine.question(one[0])])
  quiet = Sand(auto=False)
  log, root = life(quiet)
  child = outside()
  step = engine.rung("x = bash('sleep 9')", on=root)
  await settle()
  asked = engine.prompt(int, "count", on=root)
  await settle()
  command, reply = said(log, "bash")[0][1], said(log, "reply")[0][1]
  assert [e[0][:2] for e in quiet.record] == [
    ("chain", "chain1"),
    ("stand", "stand1"),
    ("done", "stand1"),
    ("chain", child),
    ("rung", step),
    ("gate", "gate1"),
    ("done", "gate1"),
    ("bash", command),
    ("started", command),
    ("merged", "merged1"),
    ("prompt", asked),
    ("reply", reply),
    ("started", reply),
  ]
  seen = Sand()
  log, root = life(seen)
  watcher = (
    "def w():\n"
    "  while (yield)[:2] != ('done', 'bash1'):\n"
    "    pass\n"
    "  yield 'tell', 'bash1', ['#bash1 seen']\n"
    "  while True:\n"
    "    yield\n"
    "\n"
    "drive(w(), 'watcher')\n"
    "b = bash('echo hi')"
  )
  assert await engine.rung(watcher, on=root) is None
  await settle()
  assert [a[1:3] for a in said(log, "tell") if a[3] == ["#bash1 seen"]] == [("bash1", "watcher")]
  assert [e for e in seen.record if e[0][2] == "watcher"] == []
  again, over = await relived(Sand(), plain(seen.record))
  assert [a[1:3] for a in said(again, "tell") if a[3] == ["#bash1 seen"]] == [("bash1", "watcher")]
  assert paragraphs(engine.turns(on=over)).count("#bash1 seen") == 1


async def test_a_read_the_operator_makes_is_kept_like_any_other_act_of_the_operator() -> None:
  """A read the operator makes is kept like any other act of the operator, since what it reads may change, and a later life makes it again at its place."""
  sand = sown()
  _, root = life(sand)
  engine.read("a.txt", on=root)
  assert [e[0][:3] for e in sand.record][-2:] == [("read", "read1", OPERATOR), ("done", "read1", WORLD)]
  with pytest.raises(Refused):
    engine.read("gone.txt", on=root)
  later = Sand(files={"/w/a.txt": "changed\n"})
  again, _ = await relived(later, list(sand.record))
  assert [a[1:3] for a in said(again, "read")] == [("read1", OPERATOR), ("read2", OPERATOR)]
  assert [(a[0], a[4:]) for a in later.calls] == [("read", ("gone.txt",)), ("stand", ())]
  assert engine.peek("read1") == Text("/w/a.txt", "one\ntwo\n") and isinstance(engine.peek("read2"), Refused)


async def test_what_it_keeps_it_says() -> None:
  """What it keeps it says, so that the World holds the record and the journal alone says what belongs in it; a World that is durable keeps what it is told, one that is not keeps nothing, and either way what the World holds is what the life after it is given."""
  sand = sown()
  log, root = await lived(sand)
  assert [a[3] for a in said(log, "keep")] == sand.record
  dead = Dead()
  await relived(dead, list(sand.record))
  assert dead.record == []
  third, once = await relived(Sand(), dead.record)
  assert said(third, "bash") == [] and once == root


async def test_given_at_its_birth_what_the_world_kept_of_an_earlier_life() -> None:
  """Given at its birth what the World kept of an earlier life, it says those entries again in the order it was given them, each once every fact said before it has been heard: an act whose maker is neither an act nor an ear that boot was given it makes again through its verb under the site of its maker, once the chain it is on has been made again, since that maker makes nothing again of what the record holds; an act whose maker is an ear that boot was given or an act it makes not, since that ear, or the word of that act, makes it again when it needs it; and any other fact it says once the act it is about has been made again, so that its controls and its dones land where they landed."""
  sand = sown()
  log, root = life(sand)
  step = engine.rung("await wait(100.0)\nclose(7)", on=root)
  await settle()
  wait = said(log, "wait")[0][1]
  engine.pause(step)
  world_says("done", wait, None)
  await settle()
  engine.wake(step)
  assert await step == 7
  child = outside()
  await settle()

  def order(heard: list[tuple]) -> list[tuple]:
    """The pause and the wake of the rung and the done of its wait, in the order the life said them."""
    return [(a[0], a[1]) for a in heard if a[1] in (step, wait) and a[0] in ("pause", "wake", "done")]

  again, _ = await relived(Sand(), list(sand.record))
  assert order(again) == order(log) == [("pause", step), ("done", wait), ("wake", step), ("done", step)]
  assert [a[1] for a in again if engine.question(a) and a[2] == OPERATOR] == [root, "stand1", step, "stand2"]
  assert [(a[1], a[2]) for a in said(again, "chain")] == [(root, OPERATOR), (child, "outside")]
  assert said(again, "wait") == said(log, "wait") and [a for a in said(again, "started") if a[2] == WORLD] == []
  assert engine.peek(step) == 7


async def test_an_entry_whose_act_this_life_has_not_made_again_the_journal_steps_over() -> None:
  """An entry whose act this life has not made again when every fact said before it has been heard, which for an act whose maker is neither an act nor an ear that boot was given is the chain it is on, the journal steps over, since this life will not make that act at that place, and it keeps the name of that act taken, so the entries after it go on and their acts keep their names."""
  first = sown()
  _, root = life(first)
  sub = engine.rung("sub = chain('sub')", on=root)
  await sub
  gone = str(engine.module(root)["sub"])
  lost = engine.grant(1.0, on=gone)
  kept = engine.grant(2.0, on=root)
  after = engine.bash("echo after", on=root)
  await settle()
  second = sown()
  heard, _ = await relived(second, [e for e in first.record if e[0][1] != sub])
  assert engine.get(gone) is None and engine.get(lost) is None
  assert [(a[1], a[3], a[4]) for a in said(heard, "grant")] == [(kept, root, 2.0)] and second.record == tip("stand2")
  assert [a[1] for a in said(heard, "bash")] == [after] and isinstance(engine.peek(after), Exit)
  third = sown()
  again, _ = await relived(third, [*[e for e in first.record if e[0][1] != sub], *second.record])
  assert [(a[1], a[3]) for a in said(again, "grant")] == [(kept, root)] and third.record == tip("stand3")


async def test_the_journal_says_the_whole_record_again_before_boot_returns() -> None:
  """The journal says the whole record again before boot returns, so no entry waits for an act that a host makes after boot."""
  first = sown()
  _, root = life(first)
  child = outside()
  ceiling = engine.grant(1.0, on=child)
  command = engine.bash("echo one", on=root)
  await settle()
  second = sown()
  heard, over = life(second, list(first.record))
  assert [(a[1], a[3]) for a in said(heard, "grant")] == [(ceiling, child)]
  assert isinstance(engine.peek(command), Exit)
  mine = engine.grant(2.0, on=over)
  await settle()
  assert mine not in (ceiling, command) and second.record[2:] == [(("grant", mine, OPERATOR, root, 2.0, None),)]
  assert second.record[:2] == tip("stand2")
  third = sown()
  again, _ = await relived(third, [*first.record, *second.record])
  assert [(a[1], a[3]) for a in said(again, "grant")] == [(ceiling, child), (mine, root)]


async def test_one_said_again_takes_the_name_it_had() -> None:
  """One said again takes the name it had, since a later life makes its acts again in the order of the record, and the life counts each kind on from there, so that no later act takes a name that is taken."""
  sand = sown()
  log, _ = await lived(sand)
  later = Sand()
  _, over = await relived(later, list(sand.record))
  later.script[over] = ["close(9)"]
  one = engine.prompt(int, "more", on=over)
  assert [a[1] for a in said(log, "prompt")] == ["prompt1", "prompt2"]
  assert await one == 9 and one == "prompt3"
  assert one not in made(log)


async def test_a_boot_is_a_life_a_second_boot_is_a_second_life_and_the_first_is_gone() -> None:
  """A boot is a life; a second boot is a second life, and the first is gone."""
  sand = sown()
  first, root = await lived(sand)
  command = said(first, "bash")[0][1]
  was = len(first)
  second, over = life(Sand())
  assert over == root
  with pytest.raises(Refused, match=r"^nothing takes read$"):
    engine.read(f"{command}/stdout", on=over)
  engine.bash("echo more", on=over)
  assert len(first) == was and len(said(second, "bash")) == 1


async def test_the_names_operator_and_journal_are_the_lifes_own() -> None:
  """The names operator and journal are the life's own, and boot refuses a generator of the outside under either name."""
  heard: list[tuple] = []
  with pytest.raises(Refused, match=r"^operator hears$"):
    engine.boot(probe=keeping(heard), operator=pair())
  assert heard == []
  with pytest.raises(Refused, match=r"^journal hears$"):
    engine.boot(journal=keeping([]))
  sand = sown()
  _, root = await lived(sand)
  log, over = await relived(Sand(), list(sand.record))
  assert over == root and said(log, "out") == [a for a in log if a[0] == "out" and a[2] == "journal"] != []


def test_a_life_settles_an_await_of_its_acts_from_outside_a_run_in_the_loop_it_is_opened_in() -> None:
  """A life settles an await of its acts from outside a run in the loop it is opened in, so boot outside a running loop raises before it makes anything."""
  woke: list[str] = []

  def telling() -> Generator[tuple | None, tuple]:
    """A generator of the outside that says it was brought to life."""
    woke.append("born")
    yield

  with pytest.raises(RuntimeError, match="loop"):
    engine.boot(probe=telling())
  assert woke == []


async def test_an_act_whose_maker_is_neither_an_act_nor_an_ear_boot_was_given_is_said_again_through_its_verb() -> None:
  """An act whose maker is neither an act nor an ear that boot was given is said again through its verb, with the words the record holds and the chain it names, so its ear is the verb's, a host finds it made when boot returns and makes it not again, and a show or a filter it was given is not said again, since the record holds none."""
  sand = sown()
  log, root = life(sand)
  one = engine.bash("echo hi", show=HIDDEN, on=root)
  assert (await one).code == 0
  engine.chain("two", source=root, filter=take(root))
  child = outside()
  await settle()
  assert [head for head in heads(engine.turns(on=root)) if head.startswith(f"#{one}")] == [f"#{one}"]
  later = Sand()
  again, over = await relived(later, list(sand.record))
  assert said(again, "bash") == said(log, "bash") and said(again, "chain") == said(log, "chain")
  assert [name for name, a in acts(log).items() if a[2] == "outside"] == [child]
  assert engine.read(f"{one}/stdout", on=over).content == "ran echo hi\n"
  assert [head for head in heads(engine.turns(on=over)) if head.startswith(f"#{one}")] == [
    f"#{one} echo hi",
    f"#{one} exited 0",
  ]
  first = sown()
  _, root = life(first)
  engine.drive(watching(root), "watcher")
  engine.say("tell", root, [f"#{root} go"])
  await settle()
  made = said([e[0] for e in first.record], "bash")
  assert [a[1:4] for a in made] == [("bash1", "watcher", root)]
  again, _ = await relived(Sand(), plain(first.record))
  assert said(again, "bash") == made


async def test_a_later_life_says_such_an_act_again_only_through_a_verb_in_the_globals_of_its_chain() -> None:
  """A later life says an act whose maker is neither an act nor an ear that boot was given again only through a verb in the globals of its chain, and an act whose kind no verb binds is a drift."""
  sand = sown()
  _, root = life(sand)
  word = "def remind(text, on=''):\n  def ear(id):\n    yield 'started', id\n    yield from idle(id)\n\n"
  await engine.rung(word + "  return act('remind', on, ear, text)\n", on=root)
  remind = engine.module(root)["remind"]
  assert callable(remind)
  note = remind("soon", on=root)
  await settle()
  second = Sand()
  again, over = await relived(second, list(sand.record))
  assert over == root and said(again, "remind") == [("remind", note, OPERATOR, root, "soon")]
  bare = engine.act("note", root, idle, "one")
  await settle()
  with pytest.raises(Drift, match=f"{bare} drifts"):
    life(Sand(), [*sand.record, *second.record])


async def test_the_journal_says_again_the_first_answer_of_an_act_when_the_act_is_made_again() -> None:
  """The journal says again a done that is the first answer of an act when the act is made again, and every later fact of it but a started at the place where the record holds it."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nawait wait()\nclose(t.content)", "close(None)"]
  one = engine.prompt(str, "read and wait", on=root)
  assert await one == "one\ntwo\n"
  await settle()
  assert [e[0][:2] for e in sand.record if e[0][1] in ("read1", "wait1")] == [
    ("read", "read1"),
    ("done", "read1"),
    ("wait", "wait1"),
    ("started", "wait1"),
    ("done", "wait1"),
  ]
  dead = Dead()
  again, over = await relived(dead, list(sand.record))
  assert engine.module(over)["t"] == Text("/w/a.txt", "one\ntwo\n")
  assert engine.peek(said(again, "wait")[0][1]) is None and engine.peek(one) == "one\ntwo\n"
  assert [a[0] for a in dead.calls] == ["stand"]
  assert said(log, "wait")[0] == said(again, "wait")[0]
  waited = said(again, "wait")[0]
  done = [a for a in said(again, "done") if a[1] == waited[1]]
  assert [a[2] for a in done] == ["journal"] and again.index(waited) < again.index(done[0])
  assert [a for a in said(again, "started") if a[1] == waited[1]] == []


async def test_what_the_record_shows_started_and_not_done_when_boot_returns_is_pending() -> None:
  """What the record shows started and not done when boot returns is pending: the journal holds it from the outside until a wake that this life says."""
  first = sown()
  log, root = life(first)
  first.script[root] = ["a = 1"]
  engine.prompt(int, "count", on=root)
  await settle()
  child = outside()
  engine.grant(1.0, on=child)
  engine.bash("echo one", on=root)
  await settle()
  second = sown()
  second.script[root] = ["b = bash('echo new')", "close(5)"]
  heard, over = life(second, list(first.record))
  await settle()
  pending = [a for a in acts(log).values() if a[0] == "reply"][-1]
  assert [a for a in second.calls if a[0] in ("bash", "reply")] == []
  engine.wake(over)
  await settle()
  assert [a[1] for a in second.calls if a[0] in ("bash", "reply")][:2] == [pending[1], "bash2"]
  assert [(a[1], a[2]) for a in said(heard, "bash")] == [("bash1", OPERATOR), ("bash2", pending[2])]
