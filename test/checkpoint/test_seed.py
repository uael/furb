"""seed, the entries of a life from the data of a checkpoint."""

import ast

from conftest import STANDS, Sand, born, life, said, settle, sown, world_says
from furb import checkpoint, engine
from furb.engine import OPERATOR, Exit, Refused, Text

SHELL = Exit(0, Text("bash2/stdout", "one\n"), Text("bash2/stderr", ""))
"""What the command that the checkpoint carries came to."""


def chain(i: str, on: str, label: str, **fields: object) -> dict[str, object]:
  """One chain of the data of a checkpoint, with nothing in it but what the test gives."""
  held = {"turns": [], "definitions": [], "values": {}, "lost": [], "cut": [], "again": [], "paused": False}
  return {"id": i, "on": on, "label": label, "source": "", **held, **fields}


DATA = {
  "taken": {"chain": 3, "bash": 2, "prompt": 1, "stand": 1},
  "chains": [
    chain("chain1", "", "root", definitions=["def f():\n  return SIZE"], values={"SIZE": "6", "shell": "Act('bash2')"}),
    chain(
      "chain3",
      "chain1",
      "work",
      paused=True,
      lost=["gone"],
      again=[{"rung": "rung9", "by": "prompt1", "word": "late = 1"}],
    ),
  ],
  "acts": [
    {"kind": "prompt", "id": "prompt1", "on": "chain3", "shape": "str", "message": "Wait.", "to": "", "held": ""},
    {"kind": "stand", "id": "stand1", "on": "chain1", "value": STANDS},
    {"kind": "bash", "id": "bash2", "on": "chain1", "words": ["echo one"], "value": SHELL},
  ],
}
"""The data of a checkpoint: the root and a chain under it that a prompt waits on, paused, the last standing, and a
command that a value of the root names."""


def noted(on: str) -> list[str]:
  """The note of each birth that the journal tells in the turns of a chain."""
  return [x[3][0] for x in engine.reading(on) if x[:3] == ("tell", on, "journal") and isinstance(x[3], list)]


def operated(log: list[tuple], on: str) -> str:
  """The rung of the operator that a birth made on a chain."""
  (seat,) = [x[1] for x in said(log, "rung") if x[2] == OPERATOR and x[3] == on]
  return seat


def debugs(facts: list[tuple], seat: str) -> list[str]:
  """The value of each debug that the rung seat told among the facts."""
  head = f"#{seat} debugged\n{seat}_debug = "
  return [ast.literal_eval(f[3][0].removeprefix(head)) for f in facts if f[0] == "tell" and f[3][0].startswith(head)]


async def seeded() -> tuple[Sand, list[tuple]]:
  """A life booted on the entries that seed gives of the data in a life before it, once the loop gave it room: the
  World and what was said."""
  life(sown())
  entries = checkpoint.seed(vars(engine), DATA)
  sand = sown()
  log, _ = life(sand, entries)
  await settle()
  return sand, log


async def test_seed_gives_the_entries_that_a_life_boots_on_from_the_data_of_a_checkpoint_which_the_open_of_an_engine_hands_it_in_the_place_of_the_checkpoint_before_the_entries_that_follow_it_so_an_engine_opens_a_checkpoint_that_an_earlier_engine_of_its_major_wrote() -> (
  None
):
  """seed gives the entries that a life boots on from the data of a checkpoint, which the open of an engine hands it in the place of the checkpoint, before the entries that follow it, so an engine opens a checkpoint that an earlier engine of its major wrote."""
  life(sown())
  entries = checkpoint.seed(vars(engine), DATA)
  assert [*entries[0][0]] == ["chain", "chain1", OPERATOR, "", "root", ""]
  sand = sown()
  life(sand, [*entries, (("wake", "chain3", OPERATOR),)])
  await settle()
  assert not engine.paused("chain3")
  assert engine.module("chain1")["SIZE"] == 6


async def test_each_chain_of_the_data_is_made_again_under_its_name_on_its_parent_and_with_its_label_as_the_operator_makes_it_and_each_name_of_a_kind_up_to_the_highest_that_the_data_takes_which_no_entry_carries_stays_taken() -> (
  None
):
  """Each chain of the data is made again under its name, on its parent and with its label, as the operator makes it, and each name of a kind up to the highest that the data takes, which no entry carries, stays taken."""
  await seeded()
  assert engine.get("chain3")[:6] == ("chain", "chain3", OPERATOR, "chain1", "work", "")
  assert engine.get("chain2") is None
  assert str(engine.chain("more", on="chain1")) == "chain4"
  assert str(engine.bash("echo two", on="chain1")) == "bash3"


async def test_each_open_prompt_of_the_data_is_made_again_through_prompt_under_its_name_and_each_chain_that_the_data_says_paused_is_paused() -> (
  None
):
  """Each open prompt of the data is made again through prompt under its name, and each chain that the data says paused is paused."""
  await seeded()
  assert engine.get("prompt1")[:7] == ("prompt", "prompt1", OPERATOR, "chain3", "str", "Wait.", "")
  assert not engine.over("prompt1")
  assert engine.paused("chain3")
  assert not engine.paused("chain1")


async def test_each_act_of_the_data_with_a_value_stands_known_on_its_chain_with_the_words_that_made_it_and_with_that_value_and_runs_nothing() -> (
  None
):
  """Each act of the data with a value stands known on its chain with the words that made it and with that value and runs nothing."""
  sand, _ = await seeded()
  assert engine.get("bash2") == ("bash", "bash2", "journal", "chain1", "echo one")
  assert engine.peek("bash2") == SHELL
  assert not [x for x in sand.calls if x[0] == "bash"]


async def test_the_journal_binds_the_module_of_each_chain_from_its_fact_carried_with_no_act_no_rung_and_no_run_so_a_birth_runs_no_word_of_the_chain_again() -> (
  None
):
  """The journal binds the module of each chain from its fact carried as it says that fact, with no act, no rung and no run, and the chain holds as its prefix the turns of its fact carried but the question, the started and the done of each act on it that seed makes again, so a birth runs no word of the chain again."""
  _, log = await seeded()
  root = engine.module("chain1")
  assert (root["f"](), root["shell"], engine.module("chain3").get("SIZE")) == (6, "bash2", None)
  assert [x for x in said(log, "rung") if x[2] == OPERATOR] == []
  assert [x for x in said(log, "run") if x[2] in ("chain1", "chain3")] == []
  turns = [
    ("tell", "prompt1", "prompt1", ["#prompt1"]),
    ("prompt", "prompt1", OPERATOR, "chain3", "str", "Wait.", ""),
    ("started", "prompt1", "prompt1"),
  ]
  chains = [chain("chain1", "", "root"), chain("chain3", "chain1", "work", turns=turns)]
  held, _ = life(sown(), checkpoint.seed(vars(engine), {**DATA, "chains": chains}))
  await settle()
  heads = lambda kind: [[tuple(f[:2]) for f in x[3]] for x in said(held, kind) if x[1] == "chain3"]  # noqa: E731
  assert heads("carried") == [[("tell", "prompt1"), ("prompt", "prompt1"), ("started", "prompt1")]]
  assert heads("prefix") == [[("tell", "prompt1")]]


async def test_each_module_that_a_born_chain_makes_again_and_the_module_of_each_chain_that_forks_it_takes_that_bind_before_any_word_runs_on_it_so_a_fork_of_a_born_chain_replays_the_bind_and_then_the_words_after_the_birth() -> (
  None
):
  """Each module that a born chain makes again, and the module of each chain that forks it, takes that bind before any word runs on it, so a fork of a born chain replays the bind and then the words after the birth."""
  await seeded()
  kid = engine.chain("kid", source="chain1")
  await settle()
  grandchild = engine.chain("grandchild", source=kid)
  await settle()
  modules = [engine.module(on) for on in ("chain1", kid, grandchild)]
  assert [(one["f"](), one["shell"]) for one in modules] == [(6, "bash2")] * 3
  assert modules[1] is not modules[0] and modules[2] is not modules[1]


async def test_each_chain_with_an_open_prompt_opens_paused_while_the_journal_binds_it_and_wakes_after_it_unless_the_data_says_it_was_paused_so_an_open_prompt_asks_its_model_again_at_once() -> (
  None
):
  """Each chain with an open prompt opens paused while the journal binds it, and wakes after it unless the data says it was paused, so an open prompt asks its model again at once."""
  life(sown())
  data = {**DATA, "chains": [chain("chain1", "", "root"), chain("chain3", "chain1", "work")]}
  entries = checkpoint.seed(vars(engine), data)
  kinds = [x[0][:3] for x in entries if x[0][0] in ("pause", "wake")]
  assert kinds == [("pause", "chain3", OPERATOR), ("wake", "chain3", OPERATOR)]
  assert [x[0][0] for x in entries][-1] == "wake"
  log, _ = life(sown(), entries)
  await settle()
  assert not engine.paused("chain3")
  assert not engine.paused("chain1")
  assert [x[2] for x in log if x[0] == "rung"] == ["prompt1"]


async def test_the_journal_binds_the_definitions_and_the_values_of_each_chain_before_seed_says_the_cut_so_a_value_that_raises_as_it_is_bound_leaves_the_turns_of_the_chain_whole() -> (
  None
):
  """The journal binds the definitions and the values of each chain before seed says the cut of the chain, so a value that raises as it is bound leaves the turns of the chain whole."""
  life(sown())
  data = {
    **DATA,
    "chains": [chain("chain1", "", "root", values={"bad": "nan", "SIZE": "6"}), chain("chain3", "chain1", "work")],
  }
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  note = "The life goes on from a checkpoint. It lost ['bad'], and it cut no act."
  assert noted("chain1") == [note], "the chain keeps the note in its turns"
  assert (engine.module("chain1")["SIZE"], "bad" in engine.module("chain1")) == (6, False)


async def test_the_fact_carried_of_a_chain_holds_its_turns_the_text_of_each_value_and_each_word_of_its_definitions_which_the_journal_binds_in_that_order() -> (
  None
):
  """The fact carried of a chain holds its turns, the text of each value of the chain and each word of its definitions, and the journal binds each value from its text, then each word of the definitions, then each value again."""
  life(sown())
  root = chain("chain1", "", "root", definitions=["def f():\n  return SIZE", "SIZE = SIZE + 1"], values={"SIZE": "6"})
  entries = checkpoint.seed(vars(engine), {**DATA, "chains": [root, chain("chain3", "chain1", "work")]})
  assert [x[0] for x in entries if x[0][0] == "carried"] == [
    ("carried", "chain1", "journal", [], {"SIZE": "6"}, ["def f():\n  return SIZE", "SIZE = SIZE + 1"], {}),
    ("carried", "chain3", "journal", [], {}, [], {}),
  ]
  life(sown(), entries)
  await settle()
  assert engine.module("chain1")["f"]() == 6


async def test_the_fact_carried_of_a_chain_names_after_the_parts_of_its_birth_who_made_the_chain_and_each_act_of_the_data_on_it_so_a_later_gathered_gives_each_act_that_seed_makes_again_the_maker_that_the_data_says() -> (
  None
):
  """The fact carried of a chain names after its definitions who made the chain and each act of the data on it, an act on no chain of the data in the fact carried of the first chain, so a later gathered gives each act that seed makes again as the operator or the journal the maker that the data says."""
  life(sown())
  prompt = {"kind": "prompt", "id": "prompt1", "on": "chain3", "shape": "str", "message": "Wait.", "to": ""}
  acts = [
    {**prompt, "held": "", "by": "rung1"},
    {"kind": "stand", "id": "stand1", "on": "chain1", "value": STANDS, "by": "rung2"},
    {"kind": "bash", "id": "bash2", "on": "chain1", "words": ["echo one"], "value": SHELL, "by": "rung3"},
    {"kind": "bash", "id": "bash1", "on": "chain9", "words": ["echo"], "value": SHELL, "by": "rung7"},
  ]
  chains = [chain("chain1", "", "root", by=OPERATOR), chain("chain3", "chain1", "work", by=OPERATOR)]
  entries = checkpoint.seed(vars(engine), {**DATA, "chains": chains, "acts": acts})
  assert {x[0][1]: x[0][6] for x in entries if x[0][0] == "carried"} == {
    "chain1": {"chain1": OPERATOR, "stand1": "rung2", "bash2": "rung3", "bash1": "rung7"},
    "chain3": {"chain3": OPERATOR, "prompt1": "rung1"},
  }
  life(sown(), entries)
  await settle()
  assert (engine.get("prompt1")[2], engine.get("bash2")[2]) == (OPERATOR, "journal")
  data = checkpoint.gathered(vars(engine), lambda word: word, {})
  assert {x["id"]: x["by"] for x in data["acts"] if x["id"] in ("prompt1", "bash2")} == {
    "prompt1": "rung1",
    "bash2": "rung3",
  }


async def test_a_fact_carried_names_no_maker_when_the_data_names_none_as_the_data_of_an_earlier_build_so_that_data_seeds_as_it_did() -> (
  None
):
  """A fact carried names no maker when the data names none, as the data of an earlier build, so that data seeds as it did."""
  life(sown())
  entries = checkpoint.seed(vars(engine), DATA)
  assert [x[0][6] for x in entries if x[0][0] == "carried"] == [{}, {}]
  life(sown(), entries)
  await settle()
  data = checkpoint.gathered(vars(engine), lambda word: word, {})
  assert {x["id"]: x["by"] for x in data["acts"] if x["id"] in ("prompt1", "bash2")} == {
    "prompt1": OPERATOR,
    "bash2": "journal",
  }


async def test_seed_reads_the_definitions_that_a_checkpoint_of_an_earlier_build_holds_as_one_string_as_one_word_of_definitions() -> (
  None
):
  """seed reads the definitions that a checkpoint of an earlier build holds as one string as one word of definitions."""
  life(sown())
  word = "def f():\n  return 1\n\ndef g():\n  return 2"
  chains = [chain("chain1", "", "root", definitions=word), chain("chain3", "chain1", "work", definitions="")]
  entries = checkpoint.seed(vars(engine), {**DATA, "chains": chains})
  assert [x[0][5] for x in entries if x[0][0] == "carried"] == [[word], [""]]
  life(sown(), entries)
  await settle()
  assert (engine.module("chain1")["f"](), engine.module("chain1")["g"]()) == (1, 2)


async def test_a_value_or_a_word_of_definitions_that_raises_as_the_journal_binds_it_stops_nothing_after_it_so_it_loses_only_itself() -> (
  None
):
  """A value or a word of definitions that raises as the journal binds it stops nothing after it, so a value that raises loses only itself, a word of definitions that raises loses only what follows the raise in that word, and the second pass of the values binds a value that the first could not bind, as one that names a class of the definitions, and gives a name that a definition bound again the value of the fact carried, while a value whose object the module still holds stays that object, which a definition took."""
  life(sown())
  definitions = [
    "class Custom(Exception):\n  pass\n\nLIMIT = 1\nRATE = 1\nFLAG = 1\nPAIR = [1, 0]\n\ndef f(x=size):\n  return x\n\ndef q(x=queue):\n  return x\n\ndef keep(x=held):\n  return x\n\ndef n(x=named):\n  return x",
    "def g():\n  return 1\nraise ValueError('v')\ndef h():\n  return 2",
  ]
  values = {
    "size": "6",
    "queue": "[1]",
    "held": "[Act('bash2')]",
    "named": "{'b', 'a', 'c', 'd'}",
    "made": "Custom('x')",
    "LIMIT": "3",
    "RATE": "1.0",
    "FLAG": "True",
    "PAIR": "[1.0, False]",
    "WINDOW": "200000.0",
    "e": "5",
    "bad": "nan",
  }
  root = chain("chain1", "", "root", definitions=definitions, values=values)
  life(sown(), checkpoint.seed(vars(engine), {**DATA, "chains": [root, chain("chain3", "chain1", "work")]}))
  await settle()
  # A word on the chain reads the objects in the sandbox that holds them, where the door keeps no identity.
  engine.rung(
    "lived = [f(), g(), isinstance(made, Custom), made.args[0], q() is queue, keep() is held, n() is named]",
    on="chain1",
  )
  await settle()
  m = engine.module("chain1")
  assert (m["lived"], m["LIMIT"], m["e"], "h" in m, "bad" in m) == (
    [6, 1, True, "x", True, True, True],
    3,
    5,
    False,
    False,
  )
  kinds = [(type(x).__name__, x) for x in (m["RATE"], m["FLAG"], *m["PAIR"], m["WINDOW"])]
  assert kinds == [("float", 1.0), ("bool", True), ("float", 1.0), ("bool", False), ("float", 200000.0)]
  assert noted("chain1") == ["The life goes on from a checkpoint. It lost ['bad'], and it cut no act."]


async def test_the_cut_of_each_chain_tells_in_a_note_of_the_journal_what_the_checkpoint_lost_and_cut_and_a_birth_that_holds_each_such_name_and_cut_no_act_tells_no_note() -> (
  None
):
  """The cut of each chain tells in a note of the journal each name that the checkpoint lost, each value of the fact carried that the born life does not hold and each act that the checkpoint cut, and a birth that holds each such name and cut no act tells no note."""
  life(sown())
  chains = [
    chain("chain1", "", "root", definitions=["AGAIN = [1]"], lost=["gone", "AGAIN"], values={"bad": "nan", "k": "1"}),
    chain("chain3", "chain1", "work", definitions=["BOUND = [2]"], lost=["BOUND"]),
  ]
  life(sown(), checkpoint.seed(vars(engine), {**DATA, "chains": chains}))
  await settle()
  assert [noted(on) for on in ("chain1", "chain3")] == [
    ["The life goes on from a checkpoint. It lost ['gone', 'bad'], and it cut no act."],
    [],
  ]
  assert (engine.module("chain1")["AGAIN"], engine.module("chain3")["BOUND"]) == ([1], [2])
  life(sown())
  life(sown(), checkpoint.seed(vars(engine), DATA))
  await settle()
  assert noted("chain3") == ["The life goes on from a checkpoint. It lost ['gone'], and it cut ['rung9']."]


async def test_a_birth_leaves_no_name_of_its_own_in_the_module_of_a_chain_whatever_raised() -> None:
  """A birth leaves no name of its own in the module of a chain, whatever raised, so no object of a birth stays held."""
  life(sown())
  values = {"k": "1", "bad": "nan"}
  root = chain(
    "chain1", "", "root", definitions=["raise ValueError('v')"], values=values, lives=["raise KeyError('k')"]
  )
  log, _ = life(sown(), checkpoint.seed(vars(engine), {**DATA, "chains": [root, chain("chain3", "chain1", "work")]}))
  await settle()
  kid = engine.chain("kid", source="chain1")
  await settle()
  births = ("__carried", "__bound", "__part", "__lost", "__raised")
  assert [[n for n in births if n in engine.module(on)] for on in ("chain1", "chain3", kid)] == [[], [], []]
  assert (engine.module("chain1")["k"], engine.module(kid)["k"]) == (1, 1)
  assert debugs(said(log, "tell"), operated(log, "chain1")) == ["KeyError('k')"]


HELD = [["held", "/w/held.py", "held()", False]]
"""The extensions of a standing whose one life word makes an act that lives until a control ends it."""


HELD_FILES = {
  "/w/a.txt": "one\ntwo\n",
  "/w/DOCTRINE.md": "Read first.\nThen act.\n",
  "/w/held.py": "def held(on=''):\n  def ear(id):\n    yield 'started', id\n    while True:\n      yield {id}\n  return act('held', on, controlled(ear))\n",
}
"""The files of a World that gives HELD: the doctrine and the file that defines held."""


def defined(word: str) -> str:
  """The defs and the classes of a word, as the host gives them to gathered."""
  try:
    body = ast.parse(word).body
  except SyntaxError:
    return ""
  return "\n\n".join(
    ast.get_source_segment(word, one) or "" for one in body if isinstance(one, ast.FunctionDef | ast.ClassDef)
  )


async def test_a_birth_tells_no_turn_of_its_own_and_each_act_that_a_life_word_made_lives_again_under_a_rung_whose_word_is_that_life_word() -> (
  None
):
  """A birth tells no turn of its own: the turns of a chain after it are its turns at the tip, and each act that a life word made lives again under a rung whose word is that life word."""
  stands = [*STANDS, "/w/DOCTRINE.md", HELD]
  born(stands=stands, files=HELD_FILES)
  await settle()
  before = engine.turns(on="chain1")
  for _ in (1, 2):
    life(
      sown(stands=stands, files=HELD_FILES),
      checkpoint.seed(vars(engine), checkpoint.gathered(vars(engine), defined, {})),
    )
    await settle()
    held = [a for a in engine.transcript("chain1") if a[0] == "held" and not engine.over(a[1])]
    assert "__kept" not in engine.module("chain1")
    assert (engine.turns(on="chain1"), [engine.get(a[2])[4] for a in held]) == (before, ["held()"])


TICKED = [["ticked", "/w/ticked.py", "ticked()", False], ["tocked", "/w/ticked.py", "tocked()", False]]
"""The extensions of a standing whose two life words each ask the clock and then make an act that lives until a
control ends it, as catchup does: a birth makes the first again while the journal still owes the rung of
the second."""


TICKED_FILES = {
  **HELD_FILES,
  "/w/ticked.py": "def ticked(on='', kind='ticked'):\n  act('clock', on, None)\n  def ear(id):\n    yield 'started', id\n    while True:\n      yield {id}\n  return act(kind, on, controlled(ear))\ndef tocked(on=''):\n  return ticked(on, 'tocked')\n",
}
"""The files of a World that gives TICKED: the doctrine and the file that defines ticked and tocked."""


async def test_a_birth_makes_again_each_life_word_that_asks_the_clock_and_a_life_that_inspects_its_record_reads_no_drift() -> (
  None
):
  """A birth makes again each life word that asks the clock, and a life that inspects its record hears the clock silent: that clock ends as nothing takes it and the journal reads no drift, because the record holds no fact of it, so the turns are those of a life that answers the clock."""
  stands = [*STANDS, "/w/DOCTRINE.md", TICKED]
  born(stands=stands, files=TICKED_FILES)
  await settle()
  before = engine.turns(on="chain1")
  data = checkpoint.gathered(vars(engine), defined, {})
  assert [x["lives"] for x in data["chains"]] == [["ticked()", "tocked()"]]
  for quiet, ends in ((frozenset(), float), (frozenset({"clock"}), Refused)):
    life(sown(stands=stands, files=TICKED_FILES, quiet=quiet), checkpoint.seed(vars(engine), data))
    await settle()
    clocks = [engine.peek(a[1], None) for a in engine.transcript("chain1") if a[0] == "clock" and a[2] != "journal"]
    assert len(clocks) == 2 and all(isinstance(x, ends) for x in clocks), clocks
    assert engine.turns(on="chain1") == before


DEEP = [["deep", "/w/deep.py", "deep()", False], ["pinged", "/w/pinged.py", "pinged()", False]]
"""The extensions of a standing whose life words make an act that ends at once: deep() leaves an act of its own open,
as compacted leaves the prompt of its chooser, and pinged() keeps a lasting ear that hears each ping, as mcp_index
hears mcp_changed after its done."""


DEEP_FILES = {
  **HELD_FILES,
  "/w/deep.py": "def deep(on=''):\n  def inner(id):\n    yield 'started', id\n    while True:\n      yield {id}\n  def ear(id):\n    yield 'started', id\n    act('inner', on, controlled(inner))\n    yield 'done', id, None\n  return act('deep', on, controlled(ear))\n",
  "/w/pinged.py": "def pinged(on=''):\n  def ear(id):\n    yield 'started', id\n    yield 'done', id, None\n    while True:\n      if (yield {id, 'ping'})[0] == 'ping':\n        yield told(id, 'pinged')\n  return act('pinged', on, controlled(ear, lasting=True))\n",
}
"""The files of a World that gives DEEP: the doctrine and the files that define deep and pinged."""


async def test_a_birth_makes_again_each_life_word_whose_act_left_an_act_of_its_own_open_or_keeps_a_lasting_ear_after_its_done() -> (
  None
):
  """A birth makes again each life word whose act left an act of its own open or keeps a lasting ear after its done, through a rung of the operator that awaits a rung of each such life word, tells in a debug what each raises, and says a cut that keeps each fact that the chain reads but what that rung told and did not debug, so a life word that raises nothing shows in no turn."""
  stands = [*STANDS, "/w/DOCTRINE.md", DEEP]
  born(stands=stands, files=DEEP_FILES)
  await settle()
  for _ in (1, 2):
    data = checkpoint.gathered(vars(engine), defined, {})
    assert [x["lives"] for x in data["chains"] if x["id"] == "chain1"] == [["deep()", "pinged()"]]
    life(sown(stands=stands, files=DEEP_FILES), checkpoint.seed(vars(engine), data))
    await settle()
    inner = [a for a in engine.transcript("chain1") if a[0] == "inner" and not engine.over(a[1])]
    assert [engine.get(engine.get(a[2])[2])[4] for a in inner] == ["deep()"]

    def heard() -> list[tuple]:
      return [a for a in engine.transcript("chain1") if a[0] == "tell" and engine.question(("pinged", a[1]))]

    before = heard()
    world_says("ping", "chain1")
    await settle()
    assert len(heard()) == len(before) + 1, heard()


async def test_a_done_prompt_that_a_module_names_pauses_no_chain_at_a_birth_so_a_word_on_it_runs_with_no_wake() -> None:
  """A done prompt that a module names pauses no chain at a birth, so a word on it runs with no wake."""
  stands = [*STANDS, "/w/DOCTRINE.md", HELD]
  _, _, root = born(stands=stands, files=HELD_FILES)
  await engine.rung("asked = prompt(str, 'Which?', to=OPERATOR)", on=root)
  await settle()
  engine.close("this one", engine.module(root)["asked"])
  await settle()
  data = checkpoint.gathered(vars(engine), defined, {})
  assert [x["paused"] for x in data["chains"]] == [False]
  life(sown(stands=stands, files=HELD_FILES), checkpoint.seed(vars(engine), data))
  await settle()
  assert not engine.paused(root)
  engine.rung("after = 1", on=root)
  await settle()
  assert engine.module(root)["after"] == 1
  assert [a for a in engine.transcript(root) if a[0] == "held" and not engine.over(a[1])]


async def test_each_act_of_the_data_with_a_value_is_made_before_each_open_prompt_so_a_message_on_a_done_prompt_of_a_paused_chain_is_asked_after_a_wake() -> (
  None
):
  """Each act of the data with a value is made before each open prompt, so a message that the operator sent on a done prompt while its chain was paused reads that prompt, and its model is asked once a wake comes in the later life."""
  _, _, root = born("close('done')", auto=False)
  one = engine.prompt(str, "fix it", on=root)
  assert await one == "done"
  engine.pause(root, "held")
  more = engine.prompt(str, "continue", on=one)
  await settle()
  log, root = life(
    sown(script={"chain1": ["close('on')"]}),
    checkpoint.seed(vars(engine), checkpoint.gathered(vars(engine), lambda word: word, {})),
  )
  await settle()
  assert engine.paused(more) and said(log, "reply") == []
  engine.wake(root)
  assert await more == "on"
  assert len(said(log, "reply")) == 1


async def test_the_prefix_of_a_born_chain_holds_no_question_started_or_done_of_an_act_on_that_chain_that_seed_makes_again_so_each_act_of_the_data_stands_once_in_the_transcript_of_its_chain() -> (
  None
):
  """The prefix of a born chain holds no question, started or done of an act on that chain that seed makes again, so each act of the data stands once in the transcript of its chain: after the facts of the prefix, or in the prefix for an act that seed does not make again, whose thread the journal does not know when the act comes, as an act with a value on an open prompt and each act on such an act."""
  _, _, root = born()
  work = engine.chain("work", on=root)
  await settle()
  word = (
    "cd('/w')\nread('/w/a.txt')\nwrite(Text('/w/b.txt', 'b'))\nchance()\nshell = bash('echo one')\n"
    "await wait(0, shell)\njob = prompt(int, 'Job.', to=OPERATOR)\n"
    "made = [x[1] for k in ('cd', 'read', 'write', 'chance', 'wait') for x in transcript(of=k)]"
  )
  await engine.rung(word, on=work)
  await engine.rung(
    f"clock()\nstand()\nwaiting = prompt(int, 'Wait.', on={work!r})\nsub = prompt(str, 'Sub?', on=waiting)", on=root
  )
  engine.close(7, engine.module(work)["job"])
  note = engine.prompt(None, "Note.", on=engine.module(root)["waiting"])
  await settle()
  deep, shut = engine.prompt(str, "Deep?", on=note), engine.prompt(str, "Shut?", on=note)
  await settle()
  engine.close("x", shut)
  await settle()
  data = checkpoint.gathered(vars(engine), lambda one: one, {})
  carried = {a["id"]: (a["kind"], a["on"]) for a in data["acts"]}
  opened = {a["id"] for a in data["acts"] if "value" not in a}
  inert = {note, deep, shut}
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  assert {"stand", "clock", "prompt", "bash", "read", "write", "wait", "cd", "chance"} <= {
    x for x, _ in carried.values()
  }
  assert carried[note][1] in opened and carried[engine.module(root)["sub"]][1] in opened
  assert (carried[deep][1], carried[shut][1], deep in opened, shut in opened) == (note, note, True, False)

  def home(on: str) -> str:
    return on if on in (root, work) else home(carried[on][1])

  for one, (kind, on) in carried.items():
    facts = engine.transcript(home(on))
    asked = [n for n, x in enumerate(facts) if x[:2] == (kind, one)]
    ended = [x for x in facts if x[:2] == ("done", one)]
    told = [n for n, x in enumerate(facts) if x[:2] == ("tell", one)]
    assert (len(asked), len(ended)) == (1, int(one not in opened)), one
    assert (engine.get(one) is None) == (one in inert), one
    assert one in opened or one in inert or asked[0] > max(told, default=-1), one


async def test_a_born_chain_with_a_source_keeps_in_its_prefix_each_fact_of_an_act_that_it_inherited_so_a_chain_whose_source_is_the_born_chain_reads_that_act() -> (
  None
):
  """A born chain with a source keeps in its prefix each fact that its fact carried holds of an act that it inherited, since seed makes that act again on the chain that it is on alone, so a chain whose source is the born chain reads that act."""
  _, _, root = born()
  await engine.rung("shell = bash('echo one')\nawait shell", on=root)
  fork = engine.chain("fork", root)
  await settle()
  shell = str(engine.module(root)["shell"])

  def held(on: str) -> list[tuple]:
    return [tuple(x[:2]) for x in engine.transcript(on) if x[1] == shell and x[0] in ("bash", "started", "done")]

  assert held(fork) == [("bash", shell), ("started", shell), ("done", shell)]
  life(sown(), checkpoint.seed(vars(engine), checkpoint.gathered(vars(engine), lambda one: one, {})))
  await settle()
  assert held(fork) == [("bash", shell), ("started", shell), ("done", shell)]
  assert [x for x in held(root) if x[0] != "started"] == [("bash", shell), ("done", shell)]
  assert engine.scope(shell) == root
  again = engine.chain("again", fork)
  await settle()
  assert [t[1] for t in engine.turns(on=again) if f"#{shell} exited 0" in str(t[1])] != []
