"""chosen, the filter that a model chooses."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_chosen_lists_each_act_of_the_chain_to_a_model_on_a_chain_of_its_own(tmp_path: Path) -> None:
  """chosen() lists each act of the chain, by its id and its words, in a prompt to a model on a chain of its own, which gives the ids to keep as list[str], and it gives a take of those ids."""
  sand, root, _ = compacting(tmp_path)
  word = "read('a.txt')\nf = await chosen()\nclose([a[1] for a in f([a for a in transcript() if question(a)])])"
  sand.script.update({root: [word], "chain2": ["close(['read1'])"]})
  assert await engine.prompt(list, "choose", on=root) == ["read1"]
  (asked,) = [a for a in engine.transcript("chain2") if a[0] == "prompt"]
  assert asked[4] == "list[str]" and "\nread1 'a.txt'\n" in f"{asked[5]}\n"
