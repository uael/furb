"""debug, what a word tells of itself as it runs."""

from conftest import acts, born, chained, heads, paragraphs, prompted, said, settle, written
from furb import engine


async def test_what_a_word_tells_of_itself_as_it_runs() -> None:
  """What a word tells of itself as it runs: the text of a template, which debug makes as python makes the f-string of the same template."""
  _, log, root = born("n = 42\ndebug(t'{n} and {n + 1}')\nclose(n)")
  assert await engine.prompt(int, "tell me", on=root) == 42
  await settle()
  step = said(log, "reply")[0][2]
  n = 42
  assert [one for one in paragraphs(engine.turns(on=root)) if " debugged" in one] == [
    f"#{step} debugged\n{step}_debug = {f'{n} and {n + 1}'!r}"
  ]


async def test_the_engine_tells_what_a_step_debugged() -> None:
  """The engine tells what a step debugged."""
  _, log, root = born("debug(t'{7}')\nclose(1)")
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "reply")[0][2]
  assert [line for line in heads(engine.turns(on=root)) if " debugged" in line] == [f"#{step} debugged"]


async def test_a_debugged_paragraph_binds_the_text_as_rungn_debug_under_the_header_debugged() -> None:
  """A debugged paragraph binds the text as rungN_debug under the header debugged, as #rung5 debugged and rung5_debug = 'i=3 7', and a text of more than one line as a quote."""
  _, _, root = born()
  step = engine.rung("i, y = 3, 'letters'\ndebug(t'{i=} {len(y)}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged\n{step}_debug = 'i=3 7'"
  lines = engine.rung("y = 'one\\ntwo'\ndebug(t'{y}')", on=root)
  await lines
  assert paragraphs(engine.turns(on=root))[-1] == f"#{lines} debugged\n<s:{lines}_debug>\none\ntwo</s:{lines}_debug>"


async def test_a_raised_header_and_a_debugged_header_stand_at_the_place_in_the_run_where_they_happened() -> None:
  """A raised header and a debugged header stand at the place in the run where they happened."""
  _, log, root = born("debug(t'{1}')\nraise ValueError('boom')", "close(1)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  await settle()
  one, two = [a[2] for a in said(log, "reply")]
  got = engine.turns(on=root)
  assert heads([got[2]]) == [f"#{one} debugged", f"#{one} raised", f"#{two} advance on {act}"]


async def test_the_transcript_holds_between_the_entries_what_the_run_of_each_word_raised_and_debugged() -> None:
  """The transcript holds between the entries what the run of each word raised and debugged."""
  _, log, root = born("debug(t'{1}')\nclose(1)")
  assert await engine.prompt(int, "tell me", on=root) == 1
  await settle()
  step = said(log, "reply")[0][2]
  held = engine.transcript(root)
  told = [i for i, one in enumerate(held) if one[0] == "tell" and one[3] == [f"#{step} debugged\n{step}_debug = '1'"]]
  opened = [i for i, one in enumerate(held) if one[0] == "rung" and one[1] == step]
  shut = [i for i, one in enumerate(held) if one[0] == "done" and one[1] == step]
  assert len(told) == 1 and opened[0] < told[0] < shut[0]


async def test_debug_is_given_a_python_template_string() -> None:
  """debug is given a python template string."""
  _, _, root = born()
  step = engine.rung("who = 'me'\ndebug(t'hello {who!r} now {1 + 1}')", on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged\n{step}_debug = \"hello 'me' now 2\""


async def test_debug_makes_each_interpolation_as_an_f_string_makes_it() -> None:
  """debug makes each interpolation as an f-string makes it, with its conversion, !r, !s or !a, and its format spec, so {x=} makes the expression, an equals sign and the repr of the value."""
  _, _, root = born()
  step = engine.rung("x, s = 5, '\u00e9'\ndebug(t'{x=} {x!r:>4} {x:03} {s!s} {s!a} {s=} {x = } {x=:>3}')", on=root)
  await step
  x, s = 5, "\u00e9"
  made = f"{x=} {x!r:>4} {x:03} {s!s} {s!a} {s=} {x = } {x=:>3}"
  assert made == "x=5    5 005 \u00e9 '\\xe9' s='\u00e9' x = 5 x=  5"
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged\n{step}_debug = {made!r}"


async def test_each_debug_of_a_rung_binds_rungn_debug_to_its_own_text() -> None:
  """Each debug of a rung binds rungN_debug to its own text, and a later one binds it again, as python binds a name again, so the name holds the last text and no text is told twice."""
  sand, log, root = born()
  step = engine.rung("a, b = 1, 2\ndebug(t'{a=}')\ndebug(t'{b=}')", on=root)
  await step
  told = paragraphs(engine.turns(on=root))[-2:]
  assert told == [f"#{step} debugged\n{step}_debug = 'a=1'", f"#{step} debugged\n{step}_debug = 'b=2'"]
  sand.script[root] = ["close(1)"]
  again = engine.prompt(int, "again", on=root)
  assert await again == 1
  (asking,) = [a[1] for a in said(log, "rung") if a[2] == again]
  opens = [prompted(again, "int", "again"), f"#{asking} advance on {again}"]
  assert said(log, "run")[-2][5] == "\n\n".join([*told, *opens])
  assert engine.module(root)[f"{step}_debug"] == "b=2"


async def test_debug_binds_nothing_in_the_module_itself() -> None:
  """debug binds nothing in the module itself, since the told rung binds what its paragraph tells, rungN_debug among it."""
  _, _, root = born()
  word = "class P:\n  def __repr__(self):\n    return '<p>'\n\np = P()\ndebug(t'{p!r}')"
  step = engine.rung(word, on=root)
  await step
  assert paragraphs(engine.turns(on=root))[-1] == f"#{step} debugged\n{step}_debug = '<p>'"
  assert f"{step}_debug" not in engine.module(root)
  assert await engine.rung(f"close({step}_debug)", on=root) == "<p>"
  assert engine.module(root)[f"{step}_debug"] == "<p>"


async def test_debug_tells_nothing_but_the_text_of_its_template() -> None:
  """debug tells nothing but the text of its template, and nothing at all when that text is empty."""
  _, _, root = born()
  one = engine.rung("n = 1\ndebug(t'before {n} after')", on=root)
  await one
  two = engine.rung("debug(t'nothing at all')", on=root)
  await two
  three = engine.rung("debug(t'')", on=root)
  await three
  assert paragraphs(engine.turns(on=root))[2:] == [
    f"#{one} by operator\n<s:{one}_word>\nn = 1\ndebug(t'before {{n}} after')</s:{one}_word>",
    f"#{one} debugged\n{one}_debug = 'before 1 after'",
    written(two, "debug(t'nothing at all')"),
    f"#{two} debugged\n{two}_debug = 'nothing at all'",
    written(three, "debug(t'')"),
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
  assert heads(engine.turns(on=root)) == [f"#{root}", f"#{root} standing", f"#{act} by operator", f"#{act} debugged"]


async def test_outside_an_act_there_is_nothing_to_tell_of_so_it_tells_nothing_as_a_tell_outside_a_run() -> None:
  """Outside an act there is nothing to tell of, so it tells nothing, as a tell outside a run."""
  _, log, root = born()
  mark = len(log)
  assert engine.debug(t"{1}") is None
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
    ("tell", step, step, [f"#{step} debugged\n{step}_debug = '1'"]),
  ]
  assert engine.scope(step) == two
  assert paragraphs(engine.turns(on=two))[-1] == f"#{step} debugged\n{step}_debug = '1'"
  assert heads(engine.turns(on=root)) == [f"#{root}", f"#{root} standing"]
