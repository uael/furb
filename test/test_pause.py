"""pause, which holds what the acts it is over hear until the wake."""

from conftest import STANDS, Sand, heads, life, paragraphs, ran, rows, said, settle, world_says
from furb import engine


async def test_a_pause_while_it_stands_nothing_it_is_over_hears() -> None:
  """A pause: while it stands, nothing it is over hears, and what is said meanwhile waits for the wake."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(act)
  sand.exits(command, 0)
  await settle()
  assert engine.peek(act, ...) is ...
  engine.wake(act)
  await settle()
  assert (await act) == 0


async def test_a_control_tells_a_header_of_its_own_name() -> None:
  """A control tells a header of its own name, so a model reads what was done to its work."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  engine.pause(act)
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0], f"#{act} echo hi", f"#{act} paused"]


async def test_pause_is_given_the_id_of_a_pending_act_or_the_id_of_a_chain() -> None:
  """pause is given the id of a pending act or the id of a chain."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("one", on=root)
  engine.pause(one)
  sand.exits(one, 0)
  await settle()
  assert f"#{one} exited 0" not in heads(engine.turns(on=root))
  two = engine.bash("two", on=root)
  engine.pause(root)
  sand.exits(two, 0)
  await settle()
  assert f"#{two} exited 0" not in heads(engine.turns(on=root))
  engine.wake(root)
  engine.wake(one)
  await settle()
  assert heads(engine.turns(on=root))[-3:] == [f"#{root} woke", f"#{one} exited 0", f"#{two} exited 0"]


async def test_a_pause_stops_no_reply_in_flight_the_reply_returns() -> None:
  """A pause stops no reply in flight: the reply returns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  assert [one[1] for one in said(log, "done") if one[1].startswith("reply")] == [said(log, "reply")[0][1]]
  assert engine.peek(act, ...) is ...
  engine.wake(root)
  await settle()
  assert (await act) == 7


async def test_a_paused_chain_goes_quiet_as_its_in_flight_work_returns() -> None:
  """A paused chain goes quiet as its in-flight work returns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  quiet = len(log)
  await settle()
  assert len(log) == quiet and len(said(log, "reply")) == 1
  assert ran(log) == [f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"]
  assert engine.peek(act, ...) is ...


async def test_a_kind_a_pause_stops_it_starts_its_ear() -> None:
  """A kind a pause stops: it starts its ear, and while a pause over it stands the ear hears nothing, and at the wake it hears everything that was said meanwhile, in order."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  assert [one[1] for one in sand.calls if one[0] == "bash"] == [act]
  assert paragraphs(engine.turns(on=root))[-1] == f"#{act} slow\n{act}: Act[Exit] = Act({act!r})"
  engine.pause(root)
  world_says("out", act, "one\n", "stdout")
  world_says("out", act, "two\n", "stdout")
  sand.exits(act, 0)
  await settle()
  assert paragraphs(engine.turns(on=root))[-1] == f"#{root} paused"
  engine.wake(root)
  await settle()
  assert paragraphs(engine.turns(on=root))[-1] == f"#{act} exited 0\n# {act}/stdout, 0 known\n# 1 one\n# 2 two"


async def test_an_act_made_in_that_time_it_hears_at_once() -> None:
  """An act made in that time it hears at once, since an act is offered to the ears while it is made and to no ear after the one that takes it, so a paused rung takes the wants of its run."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  engine.pause(act)
  assert engine.read(f"{act}/stdout", on=root).content == ""
  wanted = said(log, "read")[-1][1]
  assert [one[2] for one in said(log, "done") if one[1] == wanted] == [act]
  step = engine.rung("x = bash('two')\nclose((await x).code)", on=root)
  await settle()
  engine.pause(step)
  sand.exits(said(log, "bash")[-1][1], 5)
  await settle()
  assert [a[2] for a in said(log, "started") if a[1] in [w[1] for w in said(log, "wants")]] == [step]
  engine.wake(step)
  assert await step == 5


async def test_a_pause_stands_over_what_is_made_after_it_until_the_wake() -> None:
  """A pause stands over what is made after it, until the wake."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  engine.pause(root)
  act = engine.bash("slow", on=root)
  sand.exits(act, 0)
  await settle()
  assert f"#{act} exited 0" not in heads(engine.turns(on=root))
  engine.wake(root)
  await settle()
  assert heads(engine.turns(on=root))[-1] == f"#{act} exited 0"


async def test_a_control_is_on_the_scope_of_what_it_is_over() -> None:
  """A control is on the scope of what it is over, so it takes no chain of its own."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  act = engine.bash("slow", on=two)
  engine.pause(act)
  await settle()
  held = said(log, "pause")[0]
  assert (held[0], held[1], held[2]) == ("pause", act, engine.OPERATOR) and engine.scope(act) == two
  assert heads(engine.turns(on=two))[-2:] == [f"#{act} slow", f"#{act} paused"]
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0]]
