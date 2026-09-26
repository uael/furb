"""site, who is speaking."""

import asyncio
from collections.abc import Generator

from conftest import keeping, life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, WORLD


async def test_who_is_speaking_is_the_site_which_every_fact_is_said_from() -> None:
  """Who is speaking is the site, which every fact is said from: the generator while it speaks, the run while it is stepped, the operator otherwise."""
  sand = sown()
  log, root = life(sand)
  assert engine.site.get() == OPERATOR
  sand.script[root] = ["here = site.get()\nread('a.txt')\nclose(here)"]
  step = await engine.prompt(str, "who speaks", on=root)
  assert step == said(log, "rung")[0][1]
  assert said(log, "read")[0][2] == step
  assert [a[2] for a in said(log, "done") if a[1] == "reply1"] == [WORLD]
  engine.read("a.txt", on=root)
  assert said(log, "read")[-1][2] == OPERATOR
  engine.drive(keeping([], ("done", "none://one", None)), "keeper")
  assert said(log, "done")[-1][2] == "keeper"
  assert engine.site.get() == OPERATOR


async def test_work_that_an_ear_begins_while_it_speaks_keeps_the_site_of_that_ear() -> None:
  """Work that an ear begins while it speaks keeps the site of that ear, so what the work says later is said by that ear."""
  sand = sown()
  log, root = life(sand)
  loop = asyncio.get_running_loop()

  def later() -> Generator[None, tuple | None]:
    while True:
      if (a := (yield)) is not None and a[:2] == ("tell", root) and a[3] == ["#chain1 now"]:
        loop.call_soon(engine.say, "tell", root, ["#chain1 later"])

  engine.drive(later(), "later")
  engine.say("tell", root, ["#chain1 now"])
  await settle()
  assert [a[2] for a in said(log, "tell") if a[3] == ["#chain1 later"]] == ["later"]
  assert [a[2] for a in said(log, "out")] == [] and engine.site.get() == OPERATOR
