"""chosen, the filter that a model chooses."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_chosen_asks_choice_and_gives_a_take_of_the_ids_that_it_comes_to(tmp_path: Path) -> None:
  """chosen() asks choice() and gives a take of the ids that it comes to."""
  sand, root, _ = compacting(tmp_path)
  word = "read('a.txt')\nf = await chosen()\nclose([a[1] for a in f([a for a in transcript() if question(a)])])"
  picked = "close(([x[1] for x in transcript('chain1') if x[0] == 'read' and x[4] == 'a.txt'], ''))"
  sand.script.update({root: [word], "chain2": [picked]})
  read = await engine.prompt(list, "choose", on=root)
  assert read == [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  assert [a[4] for a in engine.transcript("chain2") if a[0] == "prompt"] == ["tuple[list[str], str]"]
