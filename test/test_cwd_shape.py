"""Cwd, the question of the working directory of a chain."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_cwd_gives_the_working_directory_of_the_chain_it_is_on() -> None:
  """A cwd gives the working directory of the chain it is on."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  engine.cd("/x", on=root)
  engine.cd("/y", on=two)
  assert (engine.cwd(on=root), engine.cwd(on=two)) == ("/x", "/y")
  asked = [a for a in engine.asked.values() if a[0] == "cwd"]
  assert asked == [("cwd", "cwd@operator.5", OPERATOR, root), ("cwd", "cwd@operator.6", OPERATOR, two)]
  answered = [a for a in said(log, "done") if a[1].startswith("cwd@")]
  assert answered == [("done", "cwd@operator.5", root, "/x"), ("done", "cwd@operator.6", two, "/y")]
