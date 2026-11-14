"""numbered, the name of a new act."""

from conftest import Sand, lived, relived
from furb import engine
from furb.engine import OPERATOR


async def test_numbered_gives_a_new_act_its_name() -> None:
  """numbered gives a new act its name, which is its kind and the number after the highest number of that kind in the life, as act asks it with the kind, the maker, the chain and the words of the act, and the journal binds it again to give an act the name that its record holds."""
  sand, _, root = await lived()
  assert [engine.numbered("note"), engine.numbered("note")] == ["note1", "note2"]
  assert engine.numbered("bash", OPERATOR, root, "echo more") == "bash2"
  assert engine.bash("echo more", on=root) == "bash3"
  record = list(sand.record)
  _, over = await relived(Sand(), record)
  assert over == root and engine.get("bash1")[4] == "echo hi" and engine.get("bash3")[4] == "echo more"
  assert engine.numbered("bash", OPERATOR, root, "echo new") == "bash4"
