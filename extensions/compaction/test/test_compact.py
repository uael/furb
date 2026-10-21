"""compact, the chain that goes on with less."""

from pathlib import Path

from conftest import settle
from extensions.conftest import compacting
from furb import engine


async def test_compact_makes_a_chain_whose_source_is_the_chain_of_the_caller(tmp_path: Path) -> None:
  """compact() makes a chain whose source is the chain of the caller, whose filter applies the filters in order, FILTERS when it is given none, and whose first word is reground of that chain, and gives that chain, which it does not prompt."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["made = compact()\nFILTERS = [lambda acts: []]\nclose([made, compact(), compact([quiet()])])"]
  act = engine.prompt(list, "compact", on=root)
  first, bare, loud = await act
  await settle()
  assert [engine.get(one)[3:] for one in (first, bare, loud)] == [(root, "compacted", root)] * 3
  for one in (first, bare, loud):
    made = [a for a in engine.transcript(one) if a[0] == "rung" and (engine.get(a[2]) or ("",))[0] == "rung"]
    assert [a[4] for a in made] == ["reground('chain1')"]
    assert [a for a in engine.transcript(one) if a[0] == "prompt" and a[3] == one] == []
  kept = [[a[1] for a in engine.transcript(one) if a[0] == "prompt"] for one in (first, bare, loud)]
  assert kept == [[act], [], [act]]
