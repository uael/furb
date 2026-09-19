"""Fact, what everything said in a life is."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_fact_is_a_tuple_its_kind_the_act_it_is_about_who_said_it_and_its_words() -> None:
  """A fact is a tuple: its kind, the act it is about, who said it, and its words, deconstructed only by match."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", fed=True, on=root)
  word = said(log, "bash")[0]
  match word:
    case ("bash", about, by, on, command, fed, timeout):
      got = (about, by, on, command, fed, timeout)
    case _:
      got = None
  assert got == (act, OPERATOR, root, "echo hi", True, 600.0)


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_everything_that_the_engine_the_world_the_kernel_and_the_operator_say_is_a_fact() -> None:
  """Everything that the engine, the World, the Kernel and the operator say is a fact, and the kind of a fact is its first slot."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert all(isinstance(one, tuple) and isinstance(one[0], str) for one in log)
  assert {OPERATOR, WORLD, "kernel", root, "journal"} <= {one[2] for one in log}
  assert {"chain", "stand", "prompt", "rung", "ask", "answer", "ready", "run", "ran", "keep"} <= {one[0] for one in log}
  assert all(one[1].startswith(one[0] + "://") for one in log if engine.question(one))


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_fact_says_who_said_it() -> None:
  """A fact says who said it: the rung that made it, the operator outside a rung, or the World or the Kernel."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(read('a.txt').content)"]
  assert await engine.prompt(str, "read it", on=root) == "one\n"
  step = said(log, "rung")[0][1]
  assert said(log, "read")[0][2] == step
  assert said(log, "answer")[0][2] == WORLD
  assert [one[2] for one in said(log, "done") if one[1].startswith("gate://")] == ["kernel"]
  assert said(log, "ran")[0][2] == step
  engine.read("a.txt", on=root)
  assert said(log, "read")[-1][2] == OPERATOR


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_fact_is_on_the_scope_of_the_act_it_is_about() -> None:
  """A fact is on the scope of the act it is about, so the chain it is on is no slot of it."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  two = engine.chain("two")
  await settle()
  act = engine.bash("echo hi", on=two)
  await act
  theirs = [one for one in log if one[1] == act]
  assert [one[0] for one in theirs if one[0] != "bash" and two in one] == []
  assert (said(log, "bash")[0][3], engine.scope(act)) == (two, two)


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_control_is_about_the_act_it_is_over() -> None:
  """A control is about the act it is over, a done, a tell and the facts of the World about the act they settle, tell of or come from, and a question about itself."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  await act
  engine.cancel(act)
  assert [one[1] for one in said(log, "cancel")] == [act]
  assert {one[0] for one in log if one[1] == act} == {"bash", "start", "out", "exited", "tell", "done", "cancel"}
  told = [one for one in said(log, "tell") if one[1] == act]
  assert [tag[0] for one in told for tag in one[3]] == ["opened", "closed"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_verb_takes_a_chain_and_the_act_it_makes_is_on_that_chain() -> None:
  """A verb takes a chain, and the act it makes is on that chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  engine.bash("echo hi", on=two)
  engine.bash("echo there", on=root)
  assert [one[3] for one in said(log, "bash")] == [two, root]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_model_calls_a_verb_from_a_rung_and_the_operator_calls_the_same_verb_outside_a_rung() -> None:
  """A model calls a verb from a rung, and the operator calls the same verb outside a rung."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('from a rung')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  engine.bash("from the operator", on=root)
  step = said(log, "rung")[0][1]
  assert [(one[4], one[2]) for one in said(log, "bash")] == [("from a rung", step), ("from the operator", OPERATOR)]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_an_act_is_on_the_chain_that_the_verb_names() -> None:
  """An act is on the chain that the verb names."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  sand.script[root] = [f"x = bash('elsewhere', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  assert [one[3] for one in said(log, "bash")] == [two]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_an_act_is_on_a_chain_and_its_facts_are_on_its_scope() -> None:
  """An act is on a chain, and its facts are on its scope."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  two = engine.chain("two")
  await settle()
  act = engine.bash("echo hi", on=two)
  await settle()
  assert (said(log, "bash")[0][3], engine.scope(act)) == (two, two)
  assert {engine.scope(one[1]) for one in log if one[1] == act} == {two}


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_chain_is_on_no_chain_and_its_scope_is_itself() -> None:
  """A chain is on no chain, and its scope is itself."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  word = said(log, "chain")[-1]
  assert (word[1], word[3], engine.scope(two)) == (two, "", two)
  assert engine.scope(root) == root


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_world_is_given_the_id_of_the_act_and_the_facts_about_the_act_carry_the_same_id() -> None:
  """The World is given the id of the act, and the facts about the act carry the same id."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  await settle()
  assert [one[1] for one in sand.calls if one[0] == "start"] == [act]
  assert {one[1] for one in log if one[0] in ("out", "exited", "start")} == {act}
  assert (await act).code == 0


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_prompt_that_a_rung_makes_on_another_chain_is_an_act_of_that_chain() -> None:
  """A prompt that a rung makes on another chain is an act of that chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  sand.script[root] = [f"p = prompt(int, 'hi', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  theirs = said(log, "prompt")[-1]
  assert theirs[3] == two
  _, held = engine.ask("transcript", two, two)
  assert isinstance(held, list) and theirs in held
