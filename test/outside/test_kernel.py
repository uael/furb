"""The Kernel of this interpreter: what a word is held to before it runs, and how it runs in the module of its chain.

The gate reads a word on a sheet with ty, against the ladder of its chain and the shape its close must carry.
The run is begun and carried by the facts of the engine, so it is driven through a life whose World answers a
standing and refuses everything else.
"""

import asyncio
from pathlib import Path

import pytest

from furb import engine
from furb.engine import OPERATOR, WINDOW, Refused
from furb.kernel import ANY, CLOSE, HEAD, NAMES, Native, compiled, declared, sheet
from outside.doubles import settle, stood, tags, worlds

STANDS = (((OPERATOR, (), WINDOW), ("opus", ("low",), 1000)), "/w", "opus/low")


def said(word: str, ladder: tuple[str, ...] = (), shape: str = "object") -> list[str]:
  """What the gate finds against a word, read against the ladder of its chain and the shape it must give."""
  return Native().gate(word, list(ladder), shape)


def test_the_sheet_binds_what_the_contract_declares() -> None:
  """The globals of a chain hold every name the contract declares, so the sheet binds the same names, one to a line,
  and ty reads a word in the vocabulary it will have when it runs."""
  assert [*declared(), "actor", "raised"] == NAMES
  assert {"bash", "Text", "HEAD", "read", "prompt", "actor", "raised"} <= set(NAMES)
  assert {"World", "Kernel"}.isdisjoint(NAMES)
  assert all(f"  {name} = __engine.{name}\n" in HEAD for name in NAMES if name != CLOSE)
  laid, lines, above = sheet("k = 1", "close(k)", "int")
  assert above > lines.stop > lines.start == HEAD.count("\n") + 1
  assert laid.endswith("  close(k)\n")
  assert "    k = 1\n" in laid
  assert "def close(value: int" in laid
  assert f"  {CLOSE} = __engine.{CLOSE}\n" not in HEAD


def test_the_reading_of_a_word_comes_before_ty() -> None:
  """A word refused for what it says is a word no tool has anything to add about."""
  assert "SyntaxError: invalid syntax" in said("def (")[0]
  assert said("x = __engine.WINDOW") == ["__engine is a name of the gate"]
  assert said("close(1)", shape="<class 'str'>") == ["<class 'str'> is no shape"]


def test_a_word_the_interpreter_will_not_take_is_a_finding_like_any_other() -> None:
  """A pin: a body of a module takes no yield where the sheet, which lays the word inside a function of its own,
  takes one, so the gate compiles the word exactly as the run will and says what the compiler said."""
  assert "SyntaxError: 'yield' outside function" in said("x = yield 1")[0]
  assert "SyntaxError: 'return' outside function" in said("return 1")[0]
  assert said("x = 1") == []
  assert compiled("close(1)", "<word>").co_filename == "<word>"


def test_a_word_is_read_against_the_shape_it_must_give() -> None:
  """The gate refuses a close whose value does not have the shape, and reads a word against no shape at all when
  the shape is left unsaid, which is what a word its caller wrote is read against."""
  assert said("close(1)", shape="int") == []
  assert said("close('a')", shape="int")[0].startswith("line 1: [invalid-argument-type]")
  assert said("close(1)", shape="str")[0].startswith("line 1: [invalid-argument-type]")
  assert said("close(1)", shape="") == []
  assert said("close(None)", shape="None") == []
  assert said("close(1)", shape="None") == []
  assert said("close(nowhere())")[0].startswith("line 1: [unresolved-reference]")
  assert said("close('a', 'rung://elsewhere')", shape="int") == []
  assert said("k = 1", shape="int") == []
  assert ANY == "object"


def test_a_shape_that_names_nothing_is_no_shape() -> None:
  """A shape the sheet cannot resolve holds a word to nothing at all, since ty reads what it cannot resolve as any
  value, so the gate says the shape is no shape rather than let every word through.

  A name of a module is such a shape: the globals of a chain hold the names of the engine and no module, so a name
  under one is a name of nobody, here and on the chain alike. A name that a rung of the chain made is no such
  thing: the ladder stands on the sheet, so the shape resolves against it.
  """
  assert said("close('a string')", shape="Text")[0].startswith("line 1: [invalid-argument-type]")
  assert said("close('a string')", shape="Text | None")[0].startswith("line 1: [invalid-argument-type]")
  assert said("close('a string')", shape="Nope") == ["Nope is no shape"]
  assert said("close('a string')", shape="furb.engine.Text") == ["furb.engine.Text is no shape"]
  assert said("close('a string')", shape="furb.engine.Text | None") == ["furb.engine.Text | None is no shape"]
  made = ("class Report:\n  n: int = 1",)
  assert said("close(Report())", made, "Report") == []
  assert said("close('a string')", made, "Report")[0].startswith("line 1: [invalid-argument-type]")


def test_a_finding_arrives_in_the_numbering_of_the_word_itself() -> None:
  """The sheet above the word and the ladder below it count for nothing: a finding is handed back in the lines of
  the word, and what ty says of the ladder is not the word's."""
  found = said("a = 1\ny: int = kept", ("kept = 'text'",))
  assert len(found) == 1
  assert found[0].startswith("line 2: [invalid-assignment]")
  assert said("x = 1", ("bad: int = 'said'",)) == []
  assert said("y: int = x + 1", ("x = None\nx = 5",)) == []
  assert said("y: str = 1", ("close(1)",))[0].startswith("line 1: [invalid-assignment]")
  assert said("y: str = 1", ("raise ValueError('boom')",))[0].startswith("line 1: [invalid-assignment]")
  assert said("close(1)", ("while True:\n  pass",), "str")[0].startswith("line 1: [invalid-argument-type]")


def test_the_gate_reads_the_names_of_the_engine_as_a_chain_binds_them() -> None:
  """A name of the engine is a binding of the chain, so a word may read it, subclass it and rebind it."""
  assert said("got = span(1, 2)\nx: list[int] = got(['a', 'b'])") == []
  assert said("x: str = span(1, 2)(['a'])")[0].startswith("line 1: [invalid-assignment]")
  assert said("old = HEAD\nHEAD = old\nx = TIMEOUT + 1") == []
  assert said("class Mine(Act): ...\nx: int = 1") == []
  assert said('x = 3\ndebug(t"{x}")') == []


def test_a_class_the_ladder_defined_is_a_shape_the_gate_reads() -> None:
  """A shape a word of the chain defined resolves on the sheet, since the ladder stands above the word."""
  ladder = ("class Conf:\n  def __init__(self, a: int):\n    self.a = a",)
  assert said("close(Conf(1))", ladder, "Conf") == []
  assert said("close(1)", ladder, "Conf")[0].startswith("line 1: [invalid-argument-type]")


def test_no_ty_on_the_path_ends_the_life_rather_than_refusing_the_word(monkeypatch: pytest.MonkeyPatch) -> None:
  """A tool that did not run has said nothing about the word, which is not the same as having found nothing in it,
  so the life ends there rather than refuse a word that nobody read and ask the model again for ever."""

  def nowhere(name: str) -> None:
    assert name == "ty"

  monkeypatch.setattr("furb.kernel.shutil.which", nowhere)
  with pytest.raises(RuntimeError, match="the gate did not run"):
    said("x = 1")


def test_a_gate_that_ran_and_came_back_angry_says_so(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
  """A tool that ran and failed has said nothing about the word either, and what it said on the way out is told."""

  def folder(name: str) -> str:
    assert name == "ty"
    return str(tmp_path)

  monkeypatch.setattr("furb.kernel.shutil.which", folder)
  with pytest.raises(RuntimeError, match="the gate did not run"):
    said("x = 1")


def test_a_return_inside_a_word_is_the_scope_it_stands_in() -> None:
  """A word answers by a close, so a return at its top level is no python; one inside a def is that def's own."""
  assert said("def f():\n  return 1\nclose(f())", shape="int") == []
  assert "SyntaxError" in said("if True:\n  return 2")[0]


async def test_a_word_runs_in_the_module_of_its_chain_and_what_it_binds_stays_bound() -> None:
  """Every rung of a chain runs in the globals of the chain, and the last rung to bind a name wins."""
  root = stood(worlds(STANDS))
  assert await engine.rung("k = 6 * 7", on=root) is None
  assert await engine.rung("close(k + 1)", on=root) == 43
  assert engine.modules[root]["k"] == 42
  assert await engine.rung("def f():\n  return 1\nclose(f() + 1)", on=root) == 2
  assert await engine.rung("k = k", on=root) is None


async def test_a_word_stops_at_the_act_it_awaits_and_is_carried_on_at_its_done() -> None:
  """The Kernel says wants for the act a run waits for, takes a sent of what that act came to, and says ran with
  what the word gave, so the engine owns the order of every run."""
  root = stood(worlds(STANDS))
  assert await engine.rung("x = wait(0.01)\nawait x\nclose('after')", on=root) == "after"
  assert await engine.rung("k = 1\nawait wait(0)\nclose(k)", on=root) == 1


async def test_a_word_awaits_an_act_and_nothing_else() -> None:
  """A rung awaits an act and nothing else, so anything else it awaits is refused where it waited, and a word that
  catches the refusal carries on."""
  root = stood(worlds(STANDS))
  with pytest.raises(Refused, match="a rung awaits an act"):
    await engine.rung("import asyncio\nawait asyncio.sleep(0)", on=root)
  word = "import asyncio\ntry:\n  await asyncio.sleep(0)\nexcept Refused as no:\n  why = str(no)\nclose(why)"
  assert "a rung awaits an act" in str(await engine.rung(word, on=root))


async def test_what_a_word_raises_is_what_the_run_came_to() -> None:
  """It is done with what the word gave, the exception for a raise, which it tells with its type and its message."""
  root = stood(worlds(STANDS))
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("kept = 1\nraise ValueError('boom')", on=root)
  assert engine.modules[root]["kept"] == 1
  assert [one[1][1:] for one in tags(root, "raised")] == [[("type", "ValueError"), ("message", "boom")]]


async def test_a_word_the_interpreter_cannot_compile_is_what_the_run_came_to() -> None:
  """A pin: the compile stands inside the run, so a word the interpreter will not take is what the run came to and
  no more, and never an error out of the life that began the frame."""
  root = stood(worlds(STANDS), gated=False)
  with pytest.raises(SyntaxError, match="'yield' outside function"):
    await engine.rung("x = yield 1", on=root)
  assert await engine.rung("close('alive')", on=root) == "alive"


async def test_a_cancel_drops_the_frame_of_the_run_it_is_over() -> None:
  """A cancel of a rung is the Kernel's to do, since the Kernel is the one running the word."""
  root = stood(worlds(STANDS))
  waits = engine.rung("await wait(30)\nclose(1)", on=root)
  await settle()
  engine.cancel(waits)
  with pytest.raises(asyncio.CancelledError):
    await waits
  assert await engine.rung("close('alive')", on=root) == "alive"


async def test_the_kernel_speaks_from_the_run_it_steps() -> None:
  """The Kernel sets the site to the rung whose word it steps, for as long as it steps it, so what the word says
  is said by that rung: a debug of a word stands in the turns under the name of its own rung."""
  root = stood(worlds(STANDS), gated=False)
  waits = engine.rung('x = 3\ndebug(t"{x}")', on=root)
  await waits
  assert [one[1] for one in tags(root, "debugged")] == [[("id", waits), ("x", 3)]]


def test_a_name_the_contract_declares_by_a_plain_assignment_is_a_name_of_the_engine(tmp_path: Path) -> None:
  """The contract names its own by an annotation or by a plain assignment, and either way a chain binds it."""
  stub = tmp_path / "said.pyi"
  stub.write_text("WIDE = 3\nnarrow: int = 4\ndef verb() -> None: ...\nclass Shape: ...\ntype Alias = int\n")
  assert declared(stub) == ["WIDE", "narrow", "verb", "Shape", "Alias"]


def test_a_gate_whose_tool_came_back_angry_ends_the_life(monkeypatch: pytest.MonkeyPatch) -> None:
  """A tool that ran and failed has said nothing about the word, so the life ends with what it said on the way out."""

  def cross(name: str) -> str:
    assert name == "ty"
    return "/usr/bin/false"

  monkeypatch.setattr("furb.kernel.shutil.which", cross)
  with pytest.raises(RuntimeError, match="the gate did not run"):
    said("x = 1")


async def test_a_word_that_awaits_and_then_ends_gives_nothing() -> None:
  """The Kernel gives nothing when the rung ended without a close, whether it awaited on the way there or not."""
  root = stood(worlds(STANDS))
  assert await engine.rung("await wait(0)\nk = 1", on=root) is None
  assert engine.modules[root]["k"] == 1


async def test_a_word_that_awaits_an_act_already_over_is_carried_on_where_it_stands() -> None:
  """An act that is over when the word comes to await it hands its value straight in, so the run is never suspended
  for a done that already stands; a cancelled one raises where the word waited."""
  root = stood(worlds(STANDS))
  assert await engine.rung("a = wait(0)\nawait wait(0.02)\nawait a\nclose('both')", on=root) == "both"
  word = "a = wait(30)\ncancel(a)\nawait wait(0.02)\ntry:\n  await a\nexcept BaseException:\n  close('cut')"
  assert await engine.rung(word, on=root) == "cut"
