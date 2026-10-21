"""compacted, a chain that compacts itself."""

from pathlib import Path

from conftest import settle
from extensions.conftest import FULL, compacting
from furb import engine
from furb.engine import OPERATOR


async def test_compacted_wraps_a_chain(tmp_path: Path) -> None:
  """compacted() wraps a chain: after an answer that reads more than SHARE of the window of its actor, at the next reply on the chain, it compacts the chain, moves each open prompt of a model to the new chain, closes the old prompt with what the new one comes to, and wraps the new chain in turn."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  sand.cost = FULL
  sand.script.update({root: ["x = 1"], "chain2": ["y = 2"], "chain3": ["close(x + y)"]})
  assert await engine.prompt(int, "go", on=root) == 3
  await settle()
  assert [engine.get(one)[3:] for one in ("chain2", "chain3")] == [
    (root, "compacted", root),
    ("chain2", "compacted", "chain2"),
  ]
  moved = [a[4:] for one in ("chain2", "chain3") for a in engine.transcript(one) if a[0] == "prompt" and a[3] == one]
  assert moved == [("int", "go", "")] * 2
  assert [one for one in ("chain2", "chain3") if engine.peek(f"prompt{one[-1]}", "open") == "open"] == []


async def test_it_moves_no_prompt_to_the_operator_and_no_prompt_that_made_it(tmp_path: Path) -> None:
  """It moves no prompt to the operator and no prompt that made it, and it never completes."""
  sand, root, _ = compacting(tmp_path)
  sand.cost = FULL
  sand.script[root] = ["w = compacted()", "x = 1", "close(1)"]
  asked = engine.prompt(int, "ask me", to=OPERATOR, on=root)
  assert await engine.prompt(int, "go", on=root) == 1
  await settle()
  watcher = engine.module(root)["w"]
  assert engine.get("chain2") is None
  assert [engine.peek(watcher, "open"), engine.peek(asked, "open")] == ["open", "open"]
