"""debug, what a word tells of itself as it runs."""

import pytest

from conftest import STANDS, Sand, life, said, settle, tags
from furb import engine
from furb.engine import Refused


async def test_what_a_word_tells_of_itself_as_it_runs() -> None:
  """What a word tells of itself as it runs: each interpolation of a template, with its expression and its value."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["n = 42\ndebug(t'{n} and {n + 1}')\nclose(n)"]
  assert await engine.prompt(int, "tell me", on=root) == 42
  await settle()
  step = said(log, "rung")[0][1]
  told = tags(engine.turns(on=root), "debugged")
  assert [tag[1] for tag in told] == [[("id", step), ("n", 42)], [("id", step), ("n + 1", 43)]]


async def test_the_engine_tells_what_a_step_debugged() -> None:
  """The engine tells what a step debugged."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["debug(t'{7}')\nclose(1)"]
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  assert [tag[1][1] for tag in tags(engine.turns(on=root), "debugged")] == [("7", 7)]


async def test_the_debugged_tag_tells_one_interpolation_of_a_debug() -> None:
  """The debugged tag tells one interpolation of a debug, its expression and its value."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("k = 5\ndebug(t'{k} {k * 2}')", on=root)
  told = tags(engine.turns(on=root), "debugged")
  assert [len(tag[1]) for tag in told] == [2, 2]
  assert [tag[1][1] for tag in told] == [("k", 5), ("k * 2", 10)]
  assert [tag[2] for tag in told] == [None, None]


async def test_a_raised_tag_and_a_debugged_tag_stand_at_the_place_in_the_run_where_they_happened() -> None:
  """A raised tag and a debugged tag stand at the place in the run where they happened."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["debug(t'{1}')\nraise ValueError('boom')", "close(1)"]
  assert await engine.prompt(int, "try", on=root) == 1
  await settle()
  got = engine.turns(on=root)
  assert [tag[0] for tag in got[2][1] if isinstance(tag, tuple)] == ["debugged", "raised", "closed", "opened"]


async def test_the_transcript_holds_between_the_entries_what_the_run_of_each_word_raised_and_debugged() -> None:
  """The transcript holds between the entries what the run of each word raised and debugged."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["debug(t'{1}')\nclose(1)"]
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "rung")[0][1]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  told = [i for i, one in enumerate(held) if one[0] == "tell" and any(tag[0] == "debugged" for tag in one[3])]
  opened = [i for i, one in enumerate(held) if one[0] == "rung" and one[1] == step]
  shut = [i for i, one in enumerate(held) if one[0] == "done" and one[1] == step]
  assert len(told) == 1 and opened[0] < told[0] < shut[0]


async def test_debug_is_given_a_python_template_string() -> None:
  """debug is given a python template string."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("who = 'me'\ndebug(t'hello {who!r} now {1 + 1}')", on=root)
  told = tags(engine.turns(on=root), "debugged")
  assert [tag[1][1] for tag in told] == [("who", "me"), ("1 + 1", 2)]


async def test_debug_tells_each_interpolation_of_the_template_with_its_expression_and_its_value() -> None:
  """debug tells each interpolation of the template, with its expression and its value."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("a, b, c = 1, 2, 3\ndebug(t'{a}{b}{c}')", on=root)
  assert [tag[1][1] for tag in tags(engine.turns(on=root), "debugged")] == [("a", 1), ("b", 2), ("c", 3)]


async def test_debug_tells_nothing_but_the_interpolations() -> None:
  """debug tells nothing but the interpolations."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("n = 1\ndebug(t'before {n} after')", on=root)
  told = tags(engine.turns(on=root), "debugged")
  assert [tag[1][1] for tag in told] == [("n", 1)] and [tag[2] for tag in told] == [None]
  assert "before" not in str(told) and "after" not in str(told)
  await engine.rung("debug(t'nothing at all')", on=root)
  assert tags(engine.turns(on=root), "debugged") == told


async def test_debug_enters_nothing_in_the_record() -> None:
  """debug enters nothing in the record."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("n = 1\ndebug(t'{n}')", on=root)
  await settle()
  assert [fact[0] for _, fact, *_ in sand.record] == ["chain", "rung"]


async def test_it_is_no_act_and_it_enters_no_record() -> None:
  """It is no act and it enters no record, and it stands in the turns at the place in the run where it happened."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  made = set(engine.acts)
  act = engine.rung("n = 1\ndebug(t'{n}')", on=root)
  await act
  await settle()
  assert set(engine.acts) - made == {act}
  assert [fact for _, fact, *_ in sand.record if fact[0] == "tell"] == []
  turn = engine.turns(on=root)[-1]
  assert [tag[0] for tag in turn[1] if isinstance(tag, tuple)] == ["opened", "opened", "opened", "debugged", "closed"]


async def test_outside_an_act_there_is_nothing_to_tell_of_so_it_is_refused() -> None:
  """Outside an act there is nothing to tell of, so it is refused."""
  sand = Sand(stands=STANDS)
  _, _ = life(sand)
  with pytest.raises(Refused, match="none outside one"):
    engine.debug(t"{1}")


async def test_the_engine_refuses_debug_outside_an_act() -> None:
  """The engine refuses debug outside an act."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  mark = len(log)
  with pytest.raises(Refused):
    engine.debug(t"{1}")
  assert log[mark:] == [] and tags(engine.turns(on=root), "debugged") == []


async def test_a_tell_is_on_the_scope_of_the_act_it_is_of_so_debug_takes_no_chain_of_its_own() -> None:
  """A tell is on the scope of the act it is of, so debug takes no chain of its own."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  step = engine.rung("debug(t'{1}')", on=two)
  await step
  told = [one for one in said(log, "tell") if any(tag[0] == "debugged" for tag in one[3])]
  assert [one[1] for one in told] == [step] and engine.scope(step) == two
  assert [tag[1][1] for tag in tags(engine.turns(on=two), "debugged")] == [("1", 1)]
  assert tags(engine.turns(on=root), "debugged") == []
