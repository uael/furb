"""debug, what a word tells of itself as it runs."""

import pytest

from conftest import STANDS, Sand, heads, life, paragraphs, rows, said, settle
from furb import engine
from furb.engine import Refused


async def test_what_a_word_tells_of_itself_as_it_runs() -> None:
  """What a word tells of itself as it runs: each interpolation of a template, with its expression and its value."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["n = 42\ndebug(t'{n} and {n + 1}')\nclose(n)"]
  assert await engine.prompt(int, "tell me", on=root) == 42
  await settle()
  step = said(log, "answer")[0][1]
  assert [one for one in paragraphs(engine.turns(on=root)) if " debugged " in one] == [
    f"#{step} debugged n = 42",
    f"#{step} debugged n + 1 = 43",
  ]


async def test_the_engine_tells_what_a_step_debugged() -> None:
  """The engine tells what a step debugged."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["debug(t'{7}')\nclose(1)"]
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "answer")[0][1]
  assert [line for line in heads(engine.turns(on=root)) if " debugged " in line] == [f"#{step} debugged 7 = 7"]


async def test_a_debugged_header_tells_one_interpolation_of_a_debug() -> None:
  """A debugged header tells one interpolation of a debug, its expression and its value."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  step = engine.rung("k = 5\ndebug(t'{k} {k * 2}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-2:] == [f"#{step} debugged k = 5", f"#{step} debugged k * 2 = 10"]


async def test_a_raised_header_and_a_debugged_header_stand_at_the_place_in_the_run_where_they_happened() -> None:
  """A raised header and a debugged header stand at the place in the run where they happened."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["debug(t'{1}')\nraise ValueError('boom')", "close(1)"]
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  await settle()
  one, two = [a[1] for a in said(log, "answer")]
  got = engine.turns(on=root)
  assert heads([got[2]]) == [f"#{one} debugged 1 = 1", f"#{one} raised ValueError('boom')", f"#{two} advance on {act}"]


async def test_the_transcript_holds_between_the_entries_what_the_run_of_each_word_raised_and_debugged() -> None:
  """The transcript holds between the entries what the run of each word raised and debugged."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["debug(t'{1}')\nclose(1)"]
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "answer")[0][1]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  told = [i for i, one in enumerate(held) if one[0] == "tell" and one[3] == [f"#{step} debugged 1 = 1"]]
  opened = [i for i, one in enumerate(held) if one[0] == "rung" and one[1] == step]
  shut = [i for i, one in enumerate(held) if one[0] == "done" and one[1] == step]
  assert len(told) == 1 and opened[0] < told[0] < shut[0]


async def test_debug_is_given_a_python_template_string() -> None:
  """debug is given a python template string."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  step = engine.rung("who = 'me'\ndebug(t'hello {who!r} now {1 + 1}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-2:] == [f"#{step} debugged who = 'me'", f"#{step} debugged 1 + 1 = 2"]


async def test_debug_tells_each_interpolation_of_the_template_with_its_expression_and_its_value() -> None:
  """debug tells each interpolation of the template, with its expression and its value."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  step = engine.rung("a, b, c = 1, 2, 3\ndebug(t'{a}{b}{c}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-3:] == [
    f"#{step} debugged a = 1",
    f"#{step} debugged b = 2",
    f"#{step} debugged c = 3",
  ]


async def test_debug_tells_nothing_but_the_interpolations() -> None:
  """debug tells nothing but the interpolations."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.rung("n = 1\ndebug(t'before {n} after')", on=root)
  await one
  two = engine.rung("debug(t'nothing at all')", on=root)
  await two
  assert paragraphs(engine.turns(on=root))[2:] == [
    f"#{one}\nn = 1\ndebug(t'before {{n}} after')",
    f"#{one} debugged n = 1",
    f"#{two}\ndebug(t'nothing at all')",
  ]


async def test_debug_enters_nothing_in_the_record() -> None:
  """debug enters nothing in the record."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  step = engine.rung("n = 1\ndebug(t'{n}')", on=root)
  await step
  await settle()
  assert [fact[:2] for fact, *_ in sand.record] == [
    ("chain", root),
    ("stand", said(log, "stand")[0][1]),
    ("rung", step),
  ]


async def test_it_is_no_act_and_it_enters_no_record() -> None:
  """It is no act and it enters no record, and it stands in the turns at the place in the run where it happened."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  made = set(engine.acts)
  act = engine.rung("n = 1\ndebug(t'{n}')", on=root)
  await act
  await settle()
  assert set(engine.acts) - made == {act}
  assert [fact for fact, *_ in sand.record if fact[0] == "tell"] == []
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0], f"#{act}", f"#{act} debugged n = 1"]


async def test_outside_an_act_there_is_nothing_to_tell_of_so_it_is_refused() -> None:
  """Outside an act there is nothing to tell of, so it is refused."""
  sand = Sand(stands=STANDS)
  _, _ = life(sand)
  with pytest.raises(Refused, match="no act"):
    engine.debug(t"{1}")


async def test_the_engine_refuses_debug_outside_an_act() -> None:
  """The engine refuses debug outside an act."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  mark = len(log)
  with pytest.raises(Refused):
    engine.debug(t"{1}")
  assert log[mark:] == []
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0]]


async def test_a_tell_is_on_the_scope_of_the_act_it_is_of_so_debug_takes_no_chain_of_its_own() -> None:
  """A tell is on the scope of the act it is of, so debug takes no chain of its own."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  step = engine.rung("debug(t'{1}')", on=two)
  await step
  assert [one for one in said(log, "tell") if one[1] == step] == [
    ("tell", step, step, [f"#{step}", "debug(t'{1}')"]),
    ("tell", step, step, [f"#{step} debugged 1 = 1"]),
  ]
  assert engine.scope(step) == two
  assert paragraphs(engine.turns(on=two))[-1] == f"#{step} debugged 1 = 1"
  assert heads(engine.turns(on=root)) == [f"#{root} root", rows(root)[0]]
