"""BUDGET, the bytes of a catch-up past which it gives the paths alone."""

from conftest import paragraphs
from extensions.conftest import CATCHUP, caught_up, constant
from furb import engine

BUDGET = constant(CATCHUP, "BUDGET")


async def test_budget_is_the_number_of_bytes_of_the_paragraph_of_a_catch_up_past_which_it_gives_the_paths_alone() -> (
  None
):
  """BUDGET is the number of bytes of the paragraph of a catch-up past which the paragraph gives the paths alone."""
  assert BUDGET == 4096
  head = "#chain1 caught up\nchain1_changed = {'/w/a.txt': {1: '"
  edge = "x" * (BUDGET - len(head) - len("'}}"))
  sand, _, root = await caught_up(files={"/w/a.txt": "a\n"})
  engine.read("a.txt", on=root)
  sand.files["/w/a.txt"] = edge + "\n"
  engine.stand()
  assert paragraphs(engine.turns(on=root))[-1] == f"{head}{edge}'}}}}" and len(f"{head}{edge}'}}}}") == BUDGET
  sand.files["/w/a.txt"] = edge + "xx\n"
  engine.stand()
  assert paragraphs(engine.turns(on=root))[-1] == "#chain1 caught up\nchain1_changed = ['/w/a.txt']"
