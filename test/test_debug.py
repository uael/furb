"""debug, what a word tells of itself as it runs."""

import pytest

from conftest import acts, born, chained, heads, paragraphs, prompted, said, settle, written
from furb import engine
from furb.engine import Refused


async def test_what_a_word_tells_of_itself_as_it_runs() -> None:
  """What a word tells of itself as it runs: each interpolation of a template, with its expression and its value."""
  _, log, root = born("n = 42\ndebug(t'{n} and {n + 1}')\nclose(n)")
  assert await engine.prompt(int, "tell me", on=root) == 42
  await settle()
  step = said(log, "reply")[0][2]
  assert [one for one in paragraphs(engine.turns(on=root)) if " debugged " in one] == [
    f"#{step} debugged n n + 1\n{step}_debug = ['42', '43']"
  ]


async def test_the_engine_tells_what_a_step_debugged() -> None:
  """The engine tells what a step debugged."""
  _, log, root = born("debug(t'{7}')\nclose(1)")
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "reply")[0][2]
  assert [line for line in heads(engine.turns(on=root)) if " debugged " in line] == [f"#{step} debugged 7"]


async def test_a_debugged_header_names_the_expression_of_each_interpolation_of_a_debug() -> None:
  """A debugged header names the expression of each interpolation of a debug, in order, and the paragraph binds the repr of each value in the list of the rung, as #rung5 debugged i len(y) and rung5_debug = ['3', '7']."""
  _, _, root = born()
  step = engine.rung("i, y = 3, 'letters'\ndebug(t'{i} {len(y)}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged i len(y)\n{step}_debug = ['3', '7']"
  assert engine.module(root)[f"{step}_debug"] == ["3", "7"]


async def test_a_raised_header_and_a_debugged_header_stand_at_the_place_in_the_run_where_they_happened() -> None:
  """A raised header and a debugged header stand at the place in the run where they happened."""
  _, log, root = born("debug(t'{1}')\nraise ValueError('boom')", "close(1)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  await settle()
  one, two = [a[2] for a in said(log, "reply")]
  got = engine.turns(on=root)
  assert heads([got[2]]) == [f"#{one} debugged 1", f"#{one} raised", f"#{two} advance on {act}"]


async def test_the_transcript_holds_between_the_entries_what_the_run_of_each_word_raised_and_debugged() -> None:
  """The transcript holds between the entries what the run of each word raised and debugged."""
  _, log, root = born("debug(t'{1}')\nclose(1)")
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "reply")[0][2]
  held = engine.transcript(root)
  told = [
    i for i, one in enumerate(held) if one[0] == "tell" and one[3] == [f"#{step} debugged 1\n{step}_debug = ['1']"]
  ]
  opened = [i for i, one in enumerate(held) if one[0] == "rung" and one[1] == step]
  shut = [i for i, one in enumerate(held) if one[0] == "done" and one[1] == step]
  assert len(told) == 1 and opened[0] < told[0] < shut[0]


async def test_debug_is_given_a_python_template_string() -> None:
  """debug is given a python template string."""
  _, _, root = born()
  step = engine.rung("who = 'me'\ndebug(t'hello {who!r} now {1 + 1}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged who 1 + 1\n{step}_debug = [\"'me'\", '2']"


async def test_debug_tells_each_interpolation_of_the_template_with_its_expression_and_its_value() -> None:
  """debug tells each interpolation of the template, with its expression and its value."""
  _, _, root = born()
  step = engine.rung("a, b, c = 1, 2, 3\ndebug(t'{a}{b}{c}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged a b c\n{step}_debug = ['1', '2', '3']"


async def test_a_rung_binds_one_list_of_what_it_debugged() -> None:
  """A rung binds one list of what it debugged, rungN_debug, in the order debugged across all its debug calls: debug adds the repr of each value to the list in the module of the chain, and its paragraph binds the part that it added, as rung5_debug[2:] = ['9'] after two values."""
  sand, log, root = born()
  step = engine.rung("a, b = 1, 2\ndebug(t'{a}{b}')\ndebug(t'{a + 8}')", on=root)
  await step
  told = paragraphs(engine.turns(on=root))[-2:]
  assert told == [
    f"#{step} debugged a b\n{step}_debug = ['1', '2']",
    f"#{step} debugged a + 8\n{step}_debug[2:] = ['9']",
  ]
  assert engine.module(root)[f"{step}_debug"] == ["1", "2", "9"]
  sand.script[root] = ["close(1)"]
  again = engine.prompt(int, "again", on=root)
  assert await again == 1
  (asking,) = [a[1] for a in said(log, "rung") if a[2] == again]
  opens = [prompted(again, "int", "again"), f"#{asking} advance on {again}"]
  assert said(log, "run")[-2][5] == "\n\n".join([*told, *opens])
  assert engine.module(root)[f"{step}_debug"] == ["1", "2", "9"]


async def test_rungn_debug_holds_the_repr_of_each_value_as_a_str() -> None:
  """rungN_debug holds the repr of each value as a str, since a debugged value is a live object of the chain and a tell holds only wire values, so the binding and the list in the module hold the same strings."""
  sand, log, root = born()
  word = "class P:\n  def __repr__(self):\n    return '<p>'\n\np = P()\ndebug(t'{p}')\ndebug(t'{1}{p}')"
  step = engine.rung(word, on=root)
  await step
  told = paragraphs(engine.turns(on=root))[-2:]
  assert told == [
    f"#{step} debugged p\n{step}_debug = ['<p>']",
    f"#{step} debugged 1 p\n{step}_debug[1:] = ['1', '<p>']",
  ]
  assert engine.module(root)[f"{step}_debug"] == ["<p>", "1", "<p>"]
  sand.script[root] = ["close(1)"]
  again = engine.prompt(int, "again", on=root)
  assert await again == 1
  (asking,) = [a[1] for a in said(log, "rung") if a[2] == again]
  opens = [prompted(again, "int", "again"), f"#{asking} advance on {again}"]
  assert said(log, "run")[-2][5] == "\n\n".join([*told, *opens])
  assert engine.module(root)[f"{step}_debug"] == ["<p>", "1", "<p>"]


async def test_debug_tells_nothing_but_the_interpolations() -> None:
  """debug tells nothing but the interpolations."""
  _, _, root = born()
  one = engine.rung("n = 1\ndebug(t'before {n} after')", on=root)
  await one
  two = engine.rung("debug(t'nothing at all')", on=root)
  await two
  assert paragraphs(engine.turns(on=root))[2:] == [
    f"#{one}\n<s:{one}_word>\nn = 1\ndebug(t'before {{n}} after')</s:{one}_word>",
    f"#{one} debugged n\n{one}_debug = ['1']",
    written(two, "debug(t'nothing at all')"),
  ]


async def test_debug_enters_nothing_in_the_record() -> None:
  """debug enters nothing in the record."""
  sand, _, root = born()
  step = engine.rung("n = 1\ndebug(t'{n}')", on=root)
  await step
  await settle()
  assert [fact[:2] for fact, *_ in sand.record] == [
    ("chain", root),
    ("stand", "stand1"),
    ("done", "stand1"),
    ("rung", step),
    ("gate", "gate1"),
    ("done", "gate1"),
  ]


async def test_it_is_no_act_and_it_enters_no_record() -> None:
  """It is no act and it enters no record, and it stands in the turns at the place in the run where it happened."""
  sand, log, root = born()
  made = set(acts(log))
  act = engine.rung("n = 1\ndebug(t'{n}')", on=root)
  await act
  await settle()
  assert {a[0] for a in acts(log).values() if a[1] not in made} == {"rung", "gate", "run"}
  assert [fact for fact, *_ in sand.record if fact[0] == "tell"] == []
  assert heads(engine.turns(on=root)) == [f"#{root}", f"#{root} standing", f"#{act}", f"#{act} debugged n"]


async def test_outside_an_act_there_is_nothing_to_tell_of_so_it_is_refused() -> None:
  """Outside an act there is nothing to tell of, so it is refused."""
  _, _, _ = born()
  with pytest.raises(Refused, match="no act"):
    engine.debug(t"{1}")


async def test_the_engine_refuses_debug_outside_an_act() -> None:
  """The engine refuses debug outside an act."""
  _, log, root = born()
  mark = len(log)
  with pytest.raises(Refused):
    engine.debug(t"{1}")
  assert log[mark:] == []
  assert heads(engine.turns(on=root)) == [f"#{root}", f"#{root} standing"]


async def test_a_tell_is_on_the_scope_of_the_act_it_is_of_so_debug_takes_no_chain_of_its_own() -> None:
  """A tell is on the scope of the act it is of, so debug takes no chain of its own."""
  _, log, root = born()
  two = await chained("two")
  step = engine.rung("debug(t'{1}')", on=two)
  await step
  assert [one for one in said(log, "tell") if one[1] == step] == [
    ("tell", step, step, [written(step, "debug(t'{1}')")]),
    ("tell", step, step, [f"#{step} debugged 1\n{step}_debug = ['1']"]),
  ]
  assert engine.scope(step) == two
  assert paragraphs(engine.turns(on=two))[-1] == f"#{step} debugged 1\n{step}_debug = ['1']"
  assert heads(engine.turns(on=root)) == [f"#{root}", f"#{root} standing"]
