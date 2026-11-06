"""seed, the entries of a life from the data of a checkpoint."""

from conftest import STANDS, Sand, life, said, settle, sown
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
    chain(
      "chain1",
      "",
      "root",
      definitions="def f():\n  return SIZE",
      values={"SIZE": "6", "shell": "Act('bash2')"},
      plays=["played = f() + 1"],
    ),
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


async def test_a_rung_of_the_operator_on_each_chain_binds_its_definitions_and_its_values_says_its_turns_as_its_prefix_tells_what_the_checkpoint_lost_and_cut_with_each_word_that_ran_on_it_in_the_data_of_schema_1_and_runs_each_life_word_of_its_plays_and_it_runs_no_other_word_again_so_the_model_of_the_chain_starts_a_cut_word_again_in_a_turn_that_shows() -> (
  None
):
  """A rung of the operator on each chain binds its definitions and its values, says its turns as its prefix, tells what the checkpoint lost and cut, with each word that ran on it in the data of schema 1, and runs each life word of its plays, and it runs no other word again, so the model of the chain starts a cut word again in a turn that shows."""
  _, log = await seeded()
  root = engine.module("chain1")
  assert (root["f"](), root["shell"], engine.module("chain3").get("SIZE")) == (6, "bash2", None)
  assert (root["played"], "played" in engine.module("chain3")) == (7, False)
  told = [x[3][0] for x in said(log, "tell") if "debugged" in x[3][0] and x[1].startswith("rung")]
  assert any("It lost ['gone'], and it cut ['rung9']." in x for x in told), told
  assert [x for x in said(log, "prefix") if x[1] == "chain3"]
  engine.wake("chain3")
  await settle()
  assert "late" not in engine.module("chain3")
  assert [x for x in engine.transcript("chain3", "rung") if x[2] == "prompt1"]
