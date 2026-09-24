"""bound, the statement that binds the name of an act."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import OPERATOR, Act


async def test_the_statement_that_binds_the_name_of_an_act_to_the_act() -> None:
  """The statement that binds the name of an act to the act, with the type of what the act comes to, as prompt1: Act[int] = Act('prompt1'), so the gate knows what an await of it gives."""
  assert engine.bound("prompt1", "int") == "prompt1: Act[int] = Act('prompt1')"
  assert engine.bound("chain1") == "chain1: Act[object] = Act('chain1')"
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = prompt(int, 'how many?', to=OPERATOR)\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "ask them", on=root) == 1
  engine.close(5, "prompt2")
  assert await Act("prompt3") is None
  assert engine.bound("prompt2", "int") in [a[4].split("\n")[0] for a in said(log, "rung") if a[2] == root]
  assert engine.modules[root]["prompt2"] == "prompt2"
  assert engine.gate("n = (await prompt2) + 1", on=root) == []
  found = engine.gate("n = (await prompt2).nope", on=root)
  assert len(found) == 1 and found[0].startswith("line 1: error[unresolved-attribute]")
