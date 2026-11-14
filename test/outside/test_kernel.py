"""The Kernel of this interpreter: how a word runs in the module of its chain.

The run is begun and carried by the facts of the engine, so it is driven through a life whose World answers a
standing, takes a wait, and takes nothing else.
"""

import re

import pytest

import furb
import furb.checkpoint
import furb.kernel
import furb.record
import furb_monty.engine
from conftest import Sand, born, journaled, kernel, plain, relived, swapped
from furb import engine
from furb.engine import OPERATOR, WINDOW, Drift, Refused
from outside.doubles import booted, worlds

STANDS = [[[OPERATOR, [], WINDOW], ["opus", ["low"], 1000]], "/w", "opus/low"]


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_reads_a_name_of_the_engine_as_the_program_bound_it_last(which: object) -> None:
  """A rung that bound a name of the engine again leaves that value to the word after it, on both engines, so a word
  that calls it reads that value when it runs."""
  swapped(which)
  try:
    root = engine.boot(journaled(), world=worlds(STANDS), **kernel())
    assert await engine.rung("def read(path: str) -> int:\n  return len(path)", on=root) is None
    assert await engine.rung("close(read('ab'))", on=root) == 2
  finally:
    swapped(furb.python)


@pytest.mark.parametrize("which", [furb.python, furb_monty.engine], ids=["python", "monty"])
async def test_a_word_compares_sets_as_python_does_on_both_engines(which: object) -> None:
  """A set and a frozenset compare by the subset order, by their members and by the methods of sets, on both engines,
  as python compares them."""
  swapped(which)
  try:
    root = engine.boot(journaled(), world=worlds(STANDS), **kernel())
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


async def test_a_word_awaits_an_act_and_nothing_else() -> None:
  """A rung awaits an act and nothing else, so anything else it awaits is refused where it waited, and a word that
  catches the refusal carries on."""
  root = booted(worlds(STANDS))
  with pytest.raises(Refused, match="a rung awaits an act"):
    await engine.rung("import asyncio\nawait asyncio.sleep(0)", on=root)
  word = "import asyncio\ntry:\n  await asyncio.sleep(0)\nexcept Refused as no:\n  why = str(no)\nclose(why)"
  assert "a rung awaits an act" in str(await engine.rung(word, on=root))


async def test_a_word_the_interpreter_cannot_compile_is_what_the_run_came_to() -> None:
  """A pin: the compile stands inside the run, so a word the interpreter will not take is what the run came to and
  no more, and never an error out of the life that began the frame."""
  root = booted(worlds(STANDS))
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
  word = "class A:\n  pass\ny = bash(f'echo {A()}')\nawait y\nclose(1)"
  sand, _, root = born(word)
  assert await engine.prompt(int, "go", on=root) == 1
  addressed = lambda one: re.sub(r" at 0x[0-9a-f]+", " at 0x1", one) if isinstance(one, str) else one  # noqa: E731
  record: list = [(tuple(map(addressed, e)),) for (e,) in plain(sand.record)]
  assert any(" at 0x1" in str(e) for e in record)
  with pytest.raises(Drift, match=r"^bash1 drifts"):
    engine.boot(furb.checkpoint.journal(vars(furb.python), record), **kernel(), world=Sand().hears())
  log, again = await relived(Sand(), record)
  assert again == root and [a[4][:6] for a in log if a[0] == "bash"] == ["echo <"]
  held = furb.record.Recorded("<f at 0x1>")
  assert furb.record.Recorded() == len and held == "<f at 0x2>" and held != "<g at 0x1>"
  assert (
    held == furb.record.Recorded("<f at 0x1>") and repr(furb.record.Recorded()) == "<a callable of an earlier life>"
  )
  wrote = (
    ("write", "write1", "rung1", "chain1", engine.Text("a", "<A at 0x1>"), ["<B at 0x1>"], {"c": "<C at 0xAB>"}),
  )
  (stood,) = furb.record.recorded(vars(furb.python), [wrote])
  assert stood[0][4:] == (engine.Text("a", "<A at 0x2>"), ["<B at 0x2>"], {"c": "<C at 0x2f>"})
