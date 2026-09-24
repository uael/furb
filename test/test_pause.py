"""pause, which holds what the acts it is over hear until the wake."""

from conftest import STANDS, Sand, heads, job, life, ran, rows, said, settle
from furb import engine
from furb.engine import WORLD


async def test_a_pause_while_it_stands_nothing_it_is_over_hears() -> None:
  """A pause: while it stands, nothing it is over hears, and what is said meanwhile waits for the wake."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = wait(100)\nawait x\nclose(7)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  waited, step = said(log, "wait")[0][1], said(log, "rung")[0][1]
  engine.pause(act)
  engine.send("done", waited, None, by=WORLD)
  await settle()
  assert act not in engine.outcomes and [one for one in said(log, "sent") if one[1] == step] == []
  engine.wake(act)
  await settle()
  assert (await act) == 7


async def test_a_control_tells_a_header_of_its_own_name() -> None:
  """A control tells a header of its own name, so a model reads what was done to its work."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "how many?", to=engine.OPERATOR, on=root)
  engine.pause(act)
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0], f"#{act} how many?", f"#{act} paused"]


async def test_pause_is_given_the_id_of_a_pending_act_or_the_id_of_a_chain() -> None:
  """pause is given the id of a pending act or the id of a chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = job(root)
  engine.pause(one)
  engine.send("finished", one, 1, by=WORLD)
  await settle()
  assert one not in engine.outcomes
  two = job(root)
  engine.pause(root)
  engine.send("finished", two, 2, by=WORLD)
  await settle()
  assert two not in engine.outcomes
  engine.wake(root)
  engine.wake(one)
  await settle()
  assert ((await one), (await two)) == (1, 2)


async def test_a_pause_stops_no_ask_in_flight_the_ask_returns() -> None:
  """A pause stops no ask in flight: the ask returns."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(root)
  await settle()
  assert [one[1] for one in said(log, "answer")] == [said(log, "ask")[0][1]]
  assert act not in engine.outcomes
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
  assert len(log) == quiet and len(said(log, "ask")) == 1
  assert ran(log) == [f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"]
  assert act not in engine.outcomes


async def test_a_kind_a_pause_stops_it_starts_its_ear() -> None:
  """A kind a pause stops: it starts its ear, and while a pause over it stands the ear hears nothing, and at the wake it hears everything that was said meanwhile, in order."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = job(root)
  await settle()
  assert engine.ask("look", root, act)[1] == "working"
  engine.pause(root)
  engine.send("finished", act, 1, by=WORLD)
  engine.send("finished", act, 2, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 1


async def test_a_query_of_that_time_it_never_hears_at_all() -> None:
  """A query of that time it never hears at all, since a query is answered while the one that asked waits, and that one waits no longer."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = job(root)
  await settle()
  engine.pause(act)
  assert engine.ask("look", root, act)[1] is None
  wanted = said(log, "look")[-1][1]
  engine.wake(act)
  await settle()
  assert [one for one in said(log, "done") if one[1] == wanted] == []
  assert engine.ask("look", root, act)[1] == "working"


async def test_a_pause_stands_over_what_is_made_after_it_until_the_wake() -> None:
  """A pause stands over what is made after it, until the wake."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  engine.pause(root)
  act = job(root)
  engine.send("finished", act, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 0


async def test_a_control_is_on_the_scope_of_what_it_is_over() -> None:
  """A control is on the scope of what it is over, so it takes no chain of its own."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  act = job(two)
  engine.pause(act)
  await settle()
  held = said(log, "pause")[0]
  assert (held[0], held[1], held[2]) == ("pause", act, engine.OPERATOR) and engine.scope(act) == two
  assert heads(engine.turns(on=two))[-1:] == [f"#{act} paused"]
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0]]
