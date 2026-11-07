"""condense, a compaction that the caller asks for now."""

from pathlib import Path

from conftest import settle
from furb import engine
from furb.engine import Text
from skills.conftest import compacting


async def test_condense_compacts_the_chain_at_once(tmp_path: Path) -> None:
  """condense() asks choice() of the chain of the caller, and at once compacts that chain to the acts that the model chose, each act said on the chain after it asked, of each path only the last read or write, and the summary of the model, and gives the chain."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("write(Text('c.txt', 'c'))\nwrite(Text('d.txt', 'd'))", on=root)
  held = {str(a[4].path): str(a[1]) for a in engine.transcript(root) if a[0] == "write" and isinstance(a[4], Text)}
  chooser = f"close(([{held['c.txt']!r}], 'gist'))"
  sand.script.update({root: ["close(await condense())"], "chain2": [chooser]})
  assert await engine.prompt(str, "go", on=root) == root
  await settle()
  told = {a[1] for a in engine.transcript(root) if a[0] == "tell"}
  assert engine.get("chain2")[3:] == (root, "chosen", "")
  assert held["c.txt"] in told and held["d.txt"] not in told
  assert engine.module(root)["summary"] == "gist"


async def test_when_the_model_gives_no_tuple_it_compacts_the_chain_with_filters(tmp_path: Path) -> None:
  """When the model gives no tuple of a list and a str, it compacts the chain with FILTERS and no summary of the model."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("FILTERS = [lambda acts: []]\nread('a.txt')", on=root)
  (one,) = [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  sand.script.update({root: ["close(await condense())"], "chain2": ["close((1, 2))"]})
  assert await engine.prompt(str, "go", on=root) == root
  await settle()
  assert [str(a[4]) for a in engine.transcript(root) if a[0] == "rung" and str(a[4]).endswith("reground()")] == [
    "reground()"
  ]
  assert [a for a in engine.transcript(root) if a[0] == "tell" and a[1] == one] == []
  assert "summary" not in engine.module(root)


async def test_it_chooses_in_a_chain_whose_module_binds_list(tmp_path: Path) -> None:
  """It reads no builtin from the module of the chain when it asks choice(), as a record from before the gate refused such a name may bind it to a str."""
  sand, root, _ = compacting(tmp_path)
  # The gate refuses that binding in a new word, so the test binds it as a record from before the rule holds it.
  assert await engine.rung("globals()['list'] = 'ls -la'\nwrite(Text('c.txt', 'c'))", on=root) is None
  held = {str(a[4].path): str(a[1]) for a in engine.transcript(root) if a[0] == "write" and isinstance(a[4], Text)}
  sand.script.update({root: ["close(await condense())"], "chain2": [f"close(([{held['c.txt']!r}], 'gist'))"]})
  assert await engine.prompt(str, "go", on=root) == root
  await settle()
  assert engine.module(root)["summary"] == "gist"
