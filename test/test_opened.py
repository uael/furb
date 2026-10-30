"""opened, the paragraph that opens an act."""

from conftest import born, said
from furb import engine


async def test_the_paragraph_that_opens_an_act() -> None:
  """The paragraph that opens an act: its header with no event and the binding of each of its words, then the statement that binds the name of the act to the act, with the type of what the act comes to, as bash1: Act[Exit] = Act('bash1'), so the gate knows what an await of it gives."""
  assert engine.opened("bash1", "Exit", command="echo hi") == (
    "tell",
    "bash1",
    ["#bash1\nbash1_command = 'echo hi'", "bash1: Act[Exit] = Act('bash1')"],
  )
  assert engine.opened("chain1", "Never") == ("tell", "chain1", ["#chain1", "chain1: Act[Never] = Act('chain1')"])
  _, log, root = born("x = bash('echo hi')", "close(1)")
  assert await engine.prompt(int, "run it", on=root) == 1
  assert "bash1: Act[Exit] = Act('bash1')" in [
    line for a in said(log, "run") if a[4].endswith("_told") for line in a[5].split("\n")
  ]
  assert engine.module(root)["bash1"] == "bash1"
  assert engine.gate("code = (await bash1).code", on=root) == []
  found = engine.gate("code = (await bash1).nope", on=root)
  assert len(found) == 1 and found[0].startswith("line 1: error[unresolved-attribute]")
