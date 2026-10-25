"""inherit, the chain that goes on from a chain of another record."""

from pathlib import Path

from conftest import settle
from extensions.conftest import carried, importing
from furb import engine


async def test_inherit_says_a_prefix_of_the_module_the_carried_turns_and_the_acts_of_the_chain(tmp_path: Path) -> None:
  """inherit(record, source) awaits carry(record, source) on the chain of the caller, and says on it a prefix of its module, the turns that the carry gave, each act of the chain and each tell about the chain, so the chain reads the turns of source as its own and keeps its acts."""
  _, root, heard = importing(tmp_path, ())
  before = [a[1] for a in engine.transcript(root) if engine.question(a)]
  assert await engine.rung("close(await inherit('old.jsonl'))", on=root) == root
  (carry,) = [a[1] for a in heard]
  held = engine.transcript(root)
  assert held[0][0] == "module" and held[1:5] == carried(carry)["turns"]
  assert [one for one in before if one not in [a[1] for a in held]] == []
  assert [a for a in held if a[0] == "tell" and a[1] == root] != []
  turns = engine.turns(on=root)
  assert turns[0] == ("user", "#prompt4\nprompt4_message = 'Count.'", None, None)
  assert turns[1] == carried(carry)["turns"][2][3]


async def test_it_opens_each_open_prompt_again_under_a_new_name(tmp_path: Path) -> None:
  """It opens on the chain again each open prompt that the carry gave, with its shape, its message and its actor, under a new name."""
  sand, root, _ = importing(tmp_path)
  sand.script[root] = ["close(3)"]
  await engine.rung("await inherit('old.jsonl')", on=root)
  await settle()
  opened = [a for a in engine.transcript(root) if a[0] == "prompt" and a[5] == "Go on."]
  assert [a[4:] for a in opened] == [("int", "Go on.", "")]
  assert opened[0][1] != "prompt70" and engine.peek(opened[0][1]) == 3


async def test_it_tells_under_the_carry_what_it_carried_and_gives_the_chain(tmp_path: Path) -> None:
  """It tells under the carry the chain and the record that it carried from, the count of the entries of that record, the sha256 of its last entry, whether its log gave the facts, how many facts of turns it carried, that each value of the module, each word that ran and each command that was open was lost, and which new prompt takes the place of which, and gives the chain."""
  _, root, heard = importing(tmp_path)
  assert await engine.rung("close(await inherit('old.jsonl'))", on=root) == root
  (carry,) = [a[1] for a in heard]
  (opened,) = [a[1] for a in engine.transcript(root) if a[0] == "prompt" and a[5] == "Go on."]
  lost = "each value of its module, each word that ran and each command that was open"
  words = {"record": "/old.jsonl", "entries": 9, "digest": "ab" * 32, "logged": True, "turns": 4, "lost": lost}
  said = engine.headed(carry, "carried", source="chain1", **words, prompts={opened: "prompt70"})
  assert [a[3] for a in engine.transcript(root) if a[0] == "tell" and a[1] == carry][-1] == [said]
  assert "k" not in engine.module(root)
