"""The Kernel of this interpreter: what a word is held to before it runs, and how it runs in the module of its chain.

The gate reads a word on the sheet of `furb.sheet` with the gate of the crate, after the program of its chain. The
run is begun and carried by the facts of the engine, so it is driven through a life whose World answers a
standing, takes a wait, and takes nothing else.
"""

import pytest

import furb
import furb.kernel
import furb_monty.engine
from conftest import Py, Sand, born, kernel, plain, relived, swapped
from furb import engine, sheet
from furb.engine import OPERATOR, WINDOW, Drift, Refused
from outside.doubles import booted, worlds

STANDS = [[[OPERATOR, [], WINDOW], ["opus", ["low"], 1000]], "/w", "opus/low"]


def said(word: str, program: tuple[str, ...] = ()) -> list[str]:
  """What the gate finds against a word, read after the program of its chain."""
  return Py().gate(word, list(program))


def test_every_name_the_engine_binds_is_a_name_a_word_may_say() -> None:
  """A word runs in the module of its chain, which holds every name the engine binds, an import of its own among
  them, so the gate reads those as bound; a name the contract alone declares is nobody's."""
  assert said("x = re.compile('a')\nclose(CancelledError)") == []
  assert said("close(Counter())") == []
  assert said("close(Question)")[0].startswith("line 1: error[unresolved-reference]")


def test_a_word_the_interpreter_will_not_take_is_a_finding_like_any_other() -> None:
  """A pin: a body of a module takes no yield where the sheet, which lays the word inside a function of its own,
  takes one, so the gate compiles the word exactly as the run will and says what the compiler said."""
  assert "'yield' outside function" in said("x = yield 1")[0]
  assert "'return' outside function" in said("return 1")[0]
  assert said("x = 1") == []


def test_a_finding_arrives_in_the_numbering_of_the_word_itself() -> None:
  """The sheet above the word and the program below it count for nothing: a finding is handed back in the lines of
  the word, and what ty says of the program is not the word's."""
  found = said("a = 1\ny: int = kept", ("kept = 'text'",))
  assert len(found) == 1
  assert found[0].startswith("line 2: error[invalid-assignment]")
  assert said("x = 1", ("bad: int = 'said'",)) == []
  assert said("y: int = x + 1", ("x = None\nx = 5",)) == []
  assert said("y: str = 1", ("close(1)",))[0].startswith("line 1: error[invalid-assignment]")
  assert said("y: str = 1", ("raise ValueError('boom')",))[0].startswith("line 1: error[invalid-assignment]")
  assert said("y: str = 1", ("while True:\n  pass",))[0].startswith("line 1: error[invalid-assignment]")


def test_the_gate_reads_the_names_of_the_engine_as_a_chain_binds_them() -> None:
  """A name of the engine is a binding of the chain, so a word may read it, subclass it and override it with a def."""
  assert said("got = span(1, 2)\nx: list[int] = picked(got, ['a', 'b'])") == []
  assert said("x: str = picked(span(1, 2), ['a'])")[0].startswith("line 1: error[invalid-assignment]")
  assert said("old = HEAD\nx = TIMEOUT + 1") == []
  assert said("old = HEAD\nHEAD = old")[0].startswith("line 2: error[engine-name]")
  assert said("class Mine(Act): ...\nx: int = 1") == []
  assert said('x = 3\ndebug(t"{x}")') == []


def test_a_word_that_binds_a_name_of_the_engine_at_its_top_is_refused_and_a_def_overrides_it() -> None:
  """A name of the engine is a binding of the chain that a word reads and a def overrides, but a word that binds it
  at its top is refused, since each later word reads it."""
  assert said("read = 1\nclose(read)")[0].startswith("line 1: error[engine-name]")
  assert said("x = read('a')\nread = 1\nclose(x)")[0].startswith("line 2: error[engine-name]")
  assert said("def read(path: str) -> str:\n  return path\nclose(read('a'))") == []


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_reads_a_name_of_the_engine_as_the_program_bound_it_last(which: object) -> None:
  """The gate finds what the run finds, on both engines: a rung that bound a name of the engine again leaves that
  value to the word after it, so a word that calls it raises when it runs, and the gate refuses a word that calls it
  where it can see it."""
  swapped(which)
  try:
    root = engine.boot((), world=worlds(STANDS), **kernel())
    assert await engine.rung("def read(path: str) -> int:\n  return len(path)", on=root) is None
    assert await engine.rung("close(read('ab'))", on=root) == 2
    assert engine.gate("x: str = read('a')\nclose(x)", on=root)[0].startswith("line 1: error[invalid-assignment]")
    with pytest.raises(Refused):
      await engine.rung("x: str = read('a')\nclose(x)", on=root)
  finally:
    swapped(furb.python)


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_that_the_interpreter_cannot_compile_leaves_the_gate_to_the_next_word(which: object) -> None:
  """A pin of the life that asked its models six thousand times: monty compiles no class with a keyword of its
  metaclass and raises NotImplementedError, which ended the ear of the gate, so every later word was refused. The
  gate now gives the fault as a finding of the word, and reads the next word."""
  swapped(which)
  try:
    root = engine.boot((), world=worlds(STANDS), **kernel())
    found = engine.gate("class P(dict, total=False):\n  x: int\nclose(1)", on=root)
    assert found == [] if which is furb.python else found[0].startswith("line 1: ")
    assert engine.gate("close(1)", on=root) == []
    assert await engine.rung("close(2)", on=root) == 2
  finally:
    swapped(furb.python)


def test_a_fault_of_the_checker_raises_out_of_the_ear_of_the_gate() -> None:
  """A fault of the checker is no finding of the word, so the ear of the gate raises it: on monty, __lasting of the
  crate refuses that gate and starts a fresh gate, and the engine of this interpreter, which runs the suite and the
  rigs, has no such guard, so the fault surfaces."""

  def checked(_: str) -> list[tuple[int, str]]:
    raise RuntimeError("the checker broke")

  names: dict[str, object] = {
    "program": lambda _: {},
    "question": lambda _: True,
    "transcript": lambda _: [],
    "Refused": Refused,
  }
  ear = sheet.gating(names, checked)
  assert next(ear) == {"gate"}
  with pytest.raises(RuntimeError, match="the checker broke"):
    ear.send(("gate", "gate1", "chain1", "chain1", "x = 1"))


def test_a_gate_turned_off_answers_each_word_with_no_finding_and_reads_none() -> None:
  """A gate that gated turns off, as the key gate of a config set to false, answers each word with no finding, so the
  word runs, and reads no sheet: a checker that would break is never called."""

  def checked(_: str) -> list[tuple[int, str]]:
    raise RuntimeError("the checker broke")

  ear = sheet.gating({}, checked, lambda: False)
  assert next(ear) == {"gate"}
  assert ear.send(("gate", "gate1", "chain1", "chain1", "x: int = 'one'")) == ("done", "gate1", [])


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_compares_sets_as_python_does_on_both_engines(which: object) -> None:
  """A set and a frozenset compare by the subset order, by their members and by the methods of sets, on both engines,
  as python compares them."""
  swapped(which)
  try:
    root = engine.boot((), world=worlds(STANDS), **kernel())
    word = (
      "a, b, c = {1}, {1, 2}, frozenset({1, 2})\n"
      "close([a < b, b < c, a <= b, b <= c, b > a, c > b, b >= a, c >= b, a == b, b == c, a != b,"
      " a.issubset(c), c.issuperset(a), a.isdisjoint({3})])"
    )
    assert await engine.rung(word, on=root) == [
      True,
      False,
      True,
      True,
      True,
      False,
      True,
      True,
      False,
      True,
      True,
      True,
      True,
      True,
    ]
  finally:
    swapped(furb.python)


def test_a_word_that_imports_the_engine_by_its_package_is_refused() -> None:
  """The gate gives the checker the engine under a name that the sheet alone says, so a word that imports the
  engine by the name of its package is refused, as it is where the engine runs in the sandbox."""
  assert said("import furb\nclose(furb)") == [
    "line 1: error[unresolved-import] (column 1) ModuleNotFoundError: No module named 'furb', in 'import furb'"
  ]
  assert said("from furb.engine import read\nclose(read)")[0].startswith("line 1: error[unresolved-import]")


def test_a_word_may_use_the_async_forms_at_its_top_level() -> None:
  """The body of the sheet is async, so a word may await, iterate and enter a context asynchronously at its top
  level, as the interpreter runs it."""
  word = "async def g():\n  yield 1\nasync for x in g():\n  close([y async for y in g()])"
  assert said(word) == []
  assert said("close((await bash('ls')).code)") == []


def test_a_rung_that_raised_leaves_what_it_bound_and_the_rungs_after_it_to_the_word() -> None:
  """Each rung of the program stands in a try of its own, so the word reads what every rung bound, the one that
  raised and the ones after it alike."""
  assert said("y: int = a + b", ("a = 1\nraise ValueError('x')", "b = 2")) == []
  assert said("y: str = b", ("raise ValueError('x')", "b = 2"))[0].startswith("line 1: error[invalid-assignment]")


async def test_a_word_awaits_an_act_and_nothing_else() -> None:
  """A rung awaits an act and nothing else, so anything else it awaits is refused where it waited, and a word that
  catches the refusal carries on."""
  # The gate refuses a read of asyncio in a word first, so the Kernel meets it only past the gate.
  root = booted(worlds(STANDS), gated=False)
  with pytest.raises(Refused, match="a rung awaits an act"):
    await engine.rung("import asyncio\nawait asyncio.sleep(0)", on=root)
  word = "import asyncio\ntry:\n  await asyncio.sleep(0)\nexcept Refused as no:\n  why = str(no)\nclose(why)"
  assert "a rung awaits an act" in str(await engine.rung(word, on=root))


async def test_a_word_the_interpreter_cannot_compile_is_what_the_run_came_to() -> None:
  """A pin: the compile stands inside the run, so a word the interpreter will not take is what the run came to and
  no more, and never an error out of the life that began the frame."""
  root = booted(worlds(STANDS), gated=False)
  with pytest.raises(SyntaxError, match="'yield' outside function"):
    await engine.rung("x = yield 1", on=root)
  assert await engine.rung("close('alive')", on=root) == "alive"


async def test_a_word_that_awaits_an_act_already_over_is_carried_on_where_it_stands() -> None:
  """An act that is over when the word comes to await it hands its value straight in, so the run is never suspended
  for a done that already stands; a cancelled one raises where the word waited."""
  root = booted(worlds(STANDS))
  assert (
    await engine.rung("a = wait(0, __name__)\nawait wait(0.02, __name__)\nawait a\nclose('both')", on=root) == "both"
  )
  word = "a = wait(30, __name__)\ncancel(a)\nawait wait(0.02, __name__)\ntry:\n  await a\nexcept BaseException:\n  close('cut')"
  assert await engine.rung(word, on=root) == "cut"


async def test_a_record_whose_acts_hold_a_callable_or_an_address_replays_on_the_engine_of_this_interpreter() -> None:
  """A record keeps a callable or an object of a word as a text that names an address of its life, which a later
  life cannot make again, so a host of this engine gives boot what recorded makes of the record: each such text
  stands for each value that shows the same with no address, and the act that the word makes again is the act of the
  record."""
  word = "x = bash('one', ok=lambda e: True)\nclass A:\n  pass\ny = bash(f'echo {A()}')\nclose(1)"
  sand, _, root = born(word)
  assert await engine.prompt(int, "go", on=root) == 1
  record = plain(sand.record)
  assert any(" at 0x" in str(e) for e in record)
  with pytest.raises(Drift, match=r"^bash1 drifts"):
    engine.boot(record, **kernel(), world=Sand().hears())
  log, again = await relived(Sand(), record)
  assert again == root and [a[4] for a in log if a[0] == "bash"][:1] == ["one"]
  held = furb.kernel.Recorded("<f at 0x1>")
  assert furb.kernel.Recorded() == len and held == "<f at 0x2>" and held != "<g at 0x1>"
  assert (
    held == furb.kernel.Recorded("<f at 0x1>") and repr(furb.kernel.Recorded()) == "<a callable of an earlier life>"
  )
  wrote = (("write", "write1", "rung1", "chain1", engine.Text("a", "<A at 0x1>"), ["<B at 0x1>"]),)
  (stood,) = furb.kernel.recorded(vars(furb.python), [wrote])
  assert stood[0][4:] == (engine.Text("a", "<A at 0x2>"), ["<B at 0x2>"])
