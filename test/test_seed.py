"""seed, the entries of a life from the data of a checkpoint."""

import ast

from conftest import STANDS, Sand, born, life, said, settle, sown, world_says
from furb import engine
from furb.engine import OPERATOR, Exit, Text

SHELL = Exit(0, Text("bash2/stdout", "one\n"), Text("bash2/stderr", ""))
"""What the command that the checkpoint carries came to."""


def chain(i: str, on: str, label: str, **fields: object) -> dict[str, object]:
  """One chain of the data of a checkpoint, with nothing in it but what the test gives."""
  held = {"turns": [], "definitions": "", "values": {}, "lost": [], "cut": [], "again": [], "paused": False}
  return {"id": i, "on": on, "label": label, "source": "", **held, **fields}


DATA = {
  "taken": {"chain": 3, "bash": 2, "prompt": 1, "stand": 1},
  "chains": [
    chain("chain1", "", "root", definitions="def f():\n  return SIZE", values={"SIZE": "6", "shell": "Act('bash2')"}),
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
    {"kind": "bash", "id": "bash2", "on": "chain1", "value": SHELL},
  ],
}
"""The data of a checkpoint: the root and a chain under it that a prompt waits on, paused, the last standing, and a
command that a value of the root names."""


async def seeded() -> tuple[Sand, list[tuple]]:
  """A life booted on the entries that seed gives of the data in a life before it, once the loop gave it room: the
  World and what was said."""
  life(sown())
  entries = engine.seed(DATA)
  sand = sown()
  log, _ = life(sand, entries)
  await settle()
  return sand, log


async def test_seed_gives_the_entries_that_a_life_boots_on_from_the_data_of_a_checkpoint_which_the_open_of_an_engine_hands_it_in_the_place_of_the_checkpoint_before_the_entries_that_follow_it_so_an_engine_opens_a_checkpoint_that_an_earlier_engine_of_its_major_wrote() -> (
  None
):
  """seed gives the entries that a life boots on from the data of a checkpoint, which the open of an engine hands it in the place of the checkpoint, before the entries that follow it, so an engine opens a checkpoint that an earlier engine of its major wrote."""
  life(sown())
  entries = engine.seed(DATA)
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


async def test_each_act_of_the_data_with_a_value_stands_known_on_its_chain_with_that_value_and_runs_nothing() -> None:
  """Each act of the data with a value stands known on its chain with that value and runs nothing."""
  sand, _ = await seeded()
  assert engine.get("bash2") == ("bash", "bash2", "journal", "chain1")
  assert engine.peek("bash2") == SHELL
  assert not [x for x in sand.calls if x[0] == "bash"]


async def test_a_rung_of_the_operator_on_each_chain_binds_its_definitions_and_its_values_says_its_turns_as_its_prefix_and_tells_what_the_checkpoint_lost_and_cut_with_each_word_that_ran_on_it_in_the_data_of_schema_1_and_it_runs_no_word_again_so_the_model_of_the_chain_starts_a_cut_word_again_in_a_turn_that_shows() -> (
  None
):
  """A rung of the operator on each chain says its turns as its prefix, binds its definitions and its values, awaits a rung of each life word whose act was open on it at the tip, says its turns as its prefix again with each fact since but a tell, and tells what the checkpoint lost and cut when it lost or cut one, with each word that ran on it in the data of schema 1, and it runs no word again, so the model of the chain starts a cut word again in a turn that shows."""
  _, log = await seeded()
  root = engine.module("chain1")
  assert (root["f"](), root["shell"], engine.module("chain3").get("SIZE")) == (6, "bash2", None)
  told = [x[3][0] for x in said(log, "tell") if "debugged" in x[3][0] and x[1].startswith("rung")]
  assert any("It lost ['gone'], and it cut ['rung9']." in x for x in told), told
  assert [x for x in said(log, "prefix") if x[1] == "chain3"]
  engine.wake("chain3")
  await settle()
  assert "late" not in engine.module("chain3")
  assert [x for x in engine.transcript("chain3", "rung") if x[2] == "prompt1"]


async def test_each_chain_with_an_open_prompt_opens_paused_while_the_seed_binds_it_and_wakes_after_it_unless_the_data_says_it_was_paused_so_an_open_prompt_asks_its_model_again_at_once() -> (
  None
):
  """Each chain with an open prompt opens paused while the seed binds it, and wakes after it unless the data says it was paused, so an open prompt asks its model again at once."""
  life(sown())
  data = {**DATA, "chains": [chain("chain1", "", "root"), chain("chain3", "chain1", "work")]}
  entries = engine.seed(data)
  kinds = [x[0][:3] for x in entries if x[0][0] in ("pause", "wake")]
  assert kinds == [("pause", "chain3", OPERATOR), ("wake", "chain3", OPERATOR)]
  assert [x[0][0] for x in entries][-1] == "wake"
  log, _ = life(sown(), entries)
  await settle()
  assert not engine.paused("chain3")
  assert not engine.paused("chain1")
  rungs = [x for x in log if x[0] == "rung"]
  assert [x[2] for x in rungs][-1:] == ["prompt1"] and OPERATOR in [x[2] for x in rungs]


async def test_the_rung_binds_the_definitions_and_the_values_of_the_chain_after_it_says_the_prefix_and_tells_what_the_checkpoint_lost_and_cut_so_a_value_that_raises_as_it_is_bound_leaves_the_turns_of_the_chain_whole() -> (
  None
):
  """The rung binds the definitions and the values of the chain after it says the prefix, so a value that raises as it is bound leaves the turns of the chain whole."""
  life(sown())
  data = {
    **DATA,
    "chains": [chain("chain1", "", "root", values={"bad": "nan", "SIZE": "6"}), chain("chain3", "chain1", "work")],
  }
  log, _ = life(sown(), engine.seed(data))
  await settle()
  assert [x for x in said(log, "prefix") if x[1] == "chain1"]
  told = [x[3][0] for x in said(log, "tell") if x[1].startswith("rung")]
  assert any("It lost no name" in x for x in told) and any(" raised" in x.split("\\n")[0] for x in told), told
  assert "SIZE" not in engine.module("chain1")


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
    life(sown(stands=stands, files=HELD_FILES), engine.seed(engine.gathered(defined, {})))
    await settle()
    held = [a for a in engine.transcript("chain1") if a[0] == "held" and not engine.over(a[1])]
    assert "__kept" not in engine.module("chain1")
    assert (engine.turns(on="chain1"), [engine.get(a[2])[4] for a in held]) == (before, ["held()"])


TICKED = [["ticked", "/w/ticked.py", "ticked()", False], ["tocked", "/w/ticked.py", "tocked()", False]]
"""The extensions of a standing whose two life words each ask an act that no ear takes, as a life that inspects a
record hears the clock and the files silent, and then make an act that lives until a control ends it: a birth makes
the first again while the journal still owes the rung of the second."""


TICKED_FILES = {
  **HELD_FILES,
  "/w/ticked.py": "def tick(on=''):\n  return act('tick', on, None)\ndef ticked(on='', kind='ticked'):\n  tick(on=on)\n  def ear(id):\n    yield 'started', id\n    while True:\n      yield {id}\n  return act(kind, on, controlled(ear))\ndef tocked(on=''):\n  return ticked(on, 'tocked')\n",
}
"""The files of a World that gives TICKED: the doctrine and the file that defines tick and ticked."""


async def test_a_birth_makes_again_a_life_word_that_asks_an_act_that_no_ear_takes_and_the_journal_reads_no_drift() -> (
  None
):
  """A birth makes again a life word that asks an act that no question of the record matches and that no ear takes, as the clock of a life that inspects its record: that act ends as it did in the life before, and the journal reads no drift, because the record holds no fact of it."""
  stands = [*STANDS, "/w/DOCTRINE.md", TICKED]
  born(stands=stands, files=TICKED_FILES)
  await settle()
  before = engine.turns(on="chain1")

  def ends() -> list[str]:
    return [repr(engine.peek(a[1], None)) for a in engine.transcript("chain1") if a[0] == "tick"]

  assert ends() == ["Refused('nothing takes tick')"] * 2
  for _ in (1, 2):
    life(sown(stands=stands, files=TICKED_FILES), engine.seed(engine.gathered(defined, {})))
    await settle()
    assert engine.turns(on="chain1") == before
    assert ends()[-2:] == ["Refused('nothing takes tick')"] * 2, ends()


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
  """A birth makes again each life word whose act left an act of its own open or keeps a lasting ear after its done."""
  stands = [*STANDS, "/w/DOCTRINE.md", DEEP]
  born(stands=stands, files=DEEP_FILES)
  await settle()
  for _ in (1, 2):
    data = engine.gathered(defined, {})
    assert [x["lives"] for x in data["chains"] if x["id"] == "chain1"] == [["deep()", "pinged()"]]
    life(sown(stands=stands, files=DEEP_FILES), engine.seed(data))
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
  data = engine.gathered(defined, {})
  assert [x["paused"] for x in data["chains"]] == [False]
  life(sown(stands=stands, files=HELD_FILES), engine.seed(data))
  await settle()
  assert not engine.paused(root)
  engine.rung("after = 1", on=root)
  await settle()
  assert engine.module(root)["after"] == 1
  assert [a for a in engine.transcript(root) if a[0] == "held" and not engine.over(a[1])]
