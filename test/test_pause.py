"""pause, which holds what the acts it is over hear until the wake."""

from conftest import STANDS, Sand, attr, life, said, settle, tags
from furb import engine
from furb.engine import WORLD


async def test_a_pause_while_it_stands_nothing_it_is_over_hears() -> None:
  """A pause: while it stands, nothing it is over hears, and what is said meanwhile waits for the wake."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(act)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(act)
  await settle()
  assert (await act) == 0


async def test_a_control_tells_a_tag_of_its_own_name() -> None:
  """A control tells a tag of its own name, so a model reads what was done to its work."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  engine.pause(act)
  assert tags(engine.turns(on=root), "paused") == [("paused", [("over", act)], None)]


async def test_pause_is_given_the_id_of_a_pending_act_or_the_id_of_a_chain() -> None:
  """pause is given the id of a pending act or the id of a chain."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("one", on=root)
  engine.pause(one)
  engine.send("exited", one, 0, by=WORLD)
  await settle()
  assert one not in engine.outcomes
  two = engine.bash("two", on=root)
  engine.pause(root)
  engine.send("exited", two, 0, by=WORLD)
  await settle()
  assert two not in engine.outcomes
  engine.wake(root)
  engine.wake(one)
  await settle()
  assert ((await one).code, (await two).code) == (0, 0)


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
  assert said(log, "run") == [] and act not in engine.outcomes


async def test_a_kind_a_pause_stops_it_starts_its_body() -> None:
  """A kind a pause stops: it starts its body, and while a pause over it stands the body hears nothing, and at the wake it hears everything that was said meanwhile, in order."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  assert [one[1] for one in sand.calls if one[0] == "start"] == [act]
  assert [tag[1] for tag in tags(engine.turns(on=root), "opened")][-1] == [("id", act), ("command", "slow")]
  engine.pause(root)
  engine.send("out", act, "one\n", "stdout", by=WORLD)
  engine.send("out", act, "two\n", "stdout", by=WORLD)
  engine.send("exited", act, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act).stdout.content == "one\ntwo\n"


async def test_a_query_of_that_time_it_never_hears_at_all() -> None:
  """A query of that time it never hears at all, since a query is answered while the one that asked waits, and that one waits no longer."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  engine.pause(act)
  assert engine.read(f"{act}/stdout", on=root) is None
  wanted = said(log, "read")[-1][1]
  engine.wake(act)
  await settle()
  assert [one for one in said(log, "done") if one[1] == wanted] == []
  assert engine.read(f"{act}/stdout", on=root).content == ""


async def test_a_pause_stands_over_what_is_made_after_it_until_the_wake() -> None:
  """A pause stands over what is made after it, until the wake."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  engine.pause(root)
  act = engine.bash("slow", on=root)
  engine.send("exited", act, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act).code == 0


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
  assert [attr(tag, "over") for tag in tags(engine.turns(on=two), "paused")] == [act]
  assert tags(engine.turns(on=root), "paused") == []
