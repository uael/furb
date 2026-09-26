"""bound, the statement that binds the name of an act."""

from conftest import Sand, life, said
from furb import engine
from furb.engine import Act


async def test_the_statement_that_binds_the_name_of_an_act_to_the_act() -> None:
  """The statement that binds the name of an act to the act, with the type of what the act comes to, as bash1: Act[Exit] = Act('bash1'), so the gate knows what an await of it gives."""
  assert engine.bound("bash1", "Exit") == "bash1: Act[Exit] = Act('bash1')"
  assert engine.bound("chain1") == "chain1: Act[object] = Act('chain1')"
  sand = Sand()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  assert await Act("prompt2") is None
  assert engine.bound("bash1", "Exit") in [a[4].split("\n")[0] for a in said(log, "rung") if a[2] == root]
  assert engine.module(root)["bash1"] == "bash1"
  assert engine.gate("code = (await bash1).code", on=root) == []
  found = engine.gate("code = (await bash1).nope", on=root)
  assert len(found) == 1 and found[0].startswith("line 1: error[unresolved-attribute]")
