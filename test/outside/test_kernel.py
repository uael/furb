"""The Kernel of this interpreter: what a word is held to before it runs, and how it runs in the module of its chain.

The gate reads a word on the sheet of `furb.sheet` with the gate of the crate, after the program of its chain. The
run is begun and carried by the facts of the engine, so it is driven through a life whose World answers a
standing and refuses everything else.
"""

import asyncio
from pathlib import Path

import pytest

import furb_monty
from furb import engine, sheet
from furb.engine import OPERATOR, WINDOW, Refused
from furb.kernel import NAMES, Native, checked, declared
from outside.doubles import settle, stood, tags, worlds

STANDS = (((OPERATOR, (), WINDOW), ("opus", ("low",), 1000)), "/w", "opus/low")


def said(word: str, program: tuple[str, ...] = ()) -> list[str]:
  """What the gate finds against a word, read after the program of its chain."""
  return gate(word, list(program))


def test_the_sheet_lays_the_engine_as_the_first_rung() -> None:
  """The module of a chain is the engine run as a word, so the sheet lays the engine itself first, whole and line
  for line, then the two names the chain binds, then the program, then the word, and ty reads a word in the
  vocabulary it will have when it runs."""
  laid, above = sheet.sheet(ENGINE, ["k = 1"], "close(k)")
  assert laid.startswith("async def __body():\n  import re\n")
  assert "".join(line[2:] + "\n" if line.strip() else "\n" for line in laid.split("\n")[1:]).startswith(ENGINE)
  assert (
    '  actor = ""\n  raised: BaseException | None = None\n  try:\n    lineage("")\n    k = 1\n  except BaseException:\n'
    "    pass\n  close(k)\n"
  ) in laid
  assert above == laid.count("\n") - 1
  assert laid.endswith("  close(k)\n")


def test_every_name_the_engine_binds_is_a_name_a_word_may_say() -> None:
  """A word runs in the module of its chain, which holds every name the engine binds, an import of its own among
  them, so the gate reads those as bound; a name the contract alone declares is nobody's."""
  assert said("x = re.compile('a')\nclose(CancelledError)") == []
  assert said("close(Counter())") == []
  assert said("close(Question)")[0].startswith("line 1: error[unresolved-reference]")


def test_the_reading_of_a_word_comes_before_ty() -> None:
  """A word refused for what it says is a word no tool has anything to add about."""
  assert said("def (")[0].startswith("line 1: ")


def test_a_word_the_interpreter_will_not_take_is_a_finding_like_any_other() -> None:
  """A pin: a body of a module takes no yield where the sheet, which lays the word inside a function of its own,
  takes one, so the gate compiles the word exactly as the run will and says what the compiler said."""
  assert "'yield' outside function" in said("x = yield 1")[0]
  assert "'return' outside function" in said("return 1")[0]
  assert said("x = 1") == []


def test_a_word_is_read_against_the_names_it_will_have() -> None:
  """The gate reads a word in the names a chain holds, and a name of nobody is a finding."""
  assert said("close(1)") == []
  assert said("close(nowhere())")[0].startswith("line 1: error[unresolved-reference]")
  assert said("close('a', 'rung://elsewhere')") == []
  assert said("k = 1") == []


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
  """A name of the engine is a binding of the chain, so a word may read it, subclass it and rebind it."""
  assert said("got = span(1, 2)\nx: list[int] = got(['a', 'b'])") == []
  assert said("x: str = span(1, 2)(['a'])")[0].startswith("line 1: error[invalid-assignment]")
  assert said("old = HEAD\nHEAD = old\nx = TIMEOUT + 1") == []
  assert said("class Mine(Act): ...\nx: int = 1") == []
  assert said('x = 3\ndebug(t"{x}")') == []


def test_a_warning_of_ty_refuses_no_word() -> None:
  """A warning is no finding: a name that may be unbound is a warning of ty, and a word that reads it runs."""
  assert said("if chance() > 0.5:\n  maybe = 1\nclose(maybe)") == []


def test_the_kernel_reads_a_sheet_through_the_gate_of_the_crate() -> None:
  """The Kernel gates through the crate's checker, so the two find the same on a sheet: one gate, one reading."""
  for word, ladder in [
    ("close(1)", ()),
    ("close(nowhere())", ()),
    ("a = 1\ny: int = kept", ("kept = 'text'",)),
    ("x: str = span(1, 2)(['a'])", ()),
    ("close((await bash('ls')).code)", ()),
  ]:
    assert sheet.gate(NAMES, list(ladder), word, furb_monty.gate) == sheet.gate(NAMES, list(ladder), word, checked)


def test_a_word_that_imports_what_the_sandbox_does_not_run_is_refused() -> None:
  """The gate reads a word against the typeshed of the sandbox, so a word that imports what monty does not run is
  refused before it runs, and a word that imports what it runs is not."""
  assert said("import subprocess\nclose(subprocess.run)") == [
    "line 1: error[unresolved-import] Cannot resolve imported module `subprocess`"
  ]
  assert said("import re\nclose(re.compile('a'))") == []


def test_a_return_inside_a_word_is_the_scope_it_stands_in() -> None:
  """A word answers by a close, so a return at its top level is no python; one inside a def is that def's own."""
  assert said("def f():\n  return 1\nclose(f())") == []
  assert "'return' outside function" in said("if True:\n  return 2")[0]


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
