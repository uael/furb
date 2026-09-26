"""Fact, what everything said in a life is."""

from conftest import WORLD, acts, born, dones, said, settle, world_says
from furb import engine
from furb.engine import OPERATOR


async def test_a_fact_is_a_tuple_its_kind_the_act_it_is_about_who_said_it_and_its_words() -> None:
  """A fact is a tuple: its kind, the act it is about, who said it, and its words, deconstructed only by match."""
  _, log, root = born()
  act = engine.bash("echo hi", fed=True, on=root)
  word = said(log, "bash")[0]
  match word:
    case ("bash", about, by, on, command, fed, timeout):
      got = (about, by, on, command, fed, timeout)
    case _:
      got = None
  assert got == (act, OPERATOR, root, "echo hi", True, 600.0)


async def test_everything_that_the_engine_the_world_the_kernel_and_the_operator_say_is_a_fact() -> None:
  """Everything that the engine, the World, the Kernel and the operator say is a fact, and the kind of a fact is its first slot."""
  _, log, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  assert all(isinstance(one, tuple) and isinstance(one[0], str) for one in log)
  assert {OPERATOR, WORLD, "gate", root, "journal"} <= {one[2] for one in log}
  kinds = {"chain", "module", "stand", "prompt", "rung", "reply", "gate", "started", "done", "ready", "run", "keep"}
  assert kinds <= {one[0] for one in log}
  assert {name: one[0] for name, one in acts(log).items()} == {
    "chain1": "chain",
    "stand1": "stand",
    "prompt1": "prompt",
    "rung1": "rung",
    "rung2": "rung",
    "reply1": "reply",
    "run1": "run",
    "gate1": "gate",
    "run2": "run",
  }


async def test_a_fact_says_who_said_it() -> None:
  """A fact says who said it: the rung that made it, the operator outside a rung, or the World or the Kernel."""
  _, log, root = born("close(read('a.txt').content)", files={"/w/a.txt": "one\n"})
  assert await engine.prompt(str, "read it", on=root) == "one\n"
  assert said(log, "rung")[0][1:3] == ("rung1", "prompt1")
  assert said(log, "read")[0][2] == "rung1"
  assert [one[2] for one in said(log, "done") if one[1].startswith("reply")] == [WORLD]
  assert [one[2] for one in said(log, "done") if one[1].startswith("gate")] == ["gate"]
  assert [(one[1], one[2]) for one in dones(log, "run")] == [("run1", "run1"), ("run2", "run2")]
  engine.read("a.txt", on=root)
  assert said(log, "read")[-1][2] == OPERATOR


async def test_a_fact_is_on_the_scope_of_the_act_it_is_about() -> None:
  """A fact is on the scope of the act it is about, so the chain it is on is no slot of it."""
  _, log, _ = born()
  two = engine.chain("two")
  await settle()
  act = engine.bash("echo hi", on=two)
  await act
  theirs = [one for one in log if one[1] == act]
  assert [one[0] for one in theirs if one[0] != "bash" and two in one] == []
  assert (said(log, "bash")[0][3], engine.scope(act)) == (two, two)


async def test_a_control_is_about_the_act_it_is_over() -> None:
  """A control is about the act it is over, a done, a tell and the facts of the World about the act they settle, tell of or come from, and a question about itself."""
  _, log, root = born(auto=False)
  act = engine.bash("slow", on=root)
  world_says("out", act, "half\n", "stdout")
  engine.cancel(act)
  await settle()
  assert [one[1] for one in said(log, "cancel")] == [act]
  assert {one[0] for one in log if one[1] == act} == {"bash", "started", "out", "tell", "done", "cancel"}
  told = [one for one in said(log, "tell") if one[1] == act]
  assert [one[3][0] for one in told] == ["#bash1 slow"]


async def test_a_verb_takes_a_chain_and_the_act_it_makes_is_on_that_chain() -> None:
  """A verb takes a chain, and the act it makes is on that chain."""
  _, log, root = born()
  two = engine.chain("two")
  await settle()
  engine.bash("echo hi", on=two)
  engine.bash("echo there", on=root)
  assert [one[3] for one in said(log, "bash")] == [two, root]


async def test_a_model_calls_a_verb_from_a_rung_and_the_operator_calls_the_same_verb_outside_a_rung() -> None:
  """A model calls a verb from a rung, and the operator calls the same verb outside a rung."""
  _, log, root = born("x = bash('from a rung')\nclose(1)")
  assert await engine.prompt(int, "run it", on=root) == 1
  engine.bash("from the operator", on=root)
  step = said(log, "rung")[0][1]
  assert [(one[4], one[2]) for one in said(log, "bash")] == [("from a rung", step), ("from the operator", OPERATOR)]


async def test_an_act_is_on_the_chain_that_the_verb_names() -> None:
  """An act is on the chain that the verb names."""
  sand, log, root = born()
  two = engine.chain("two")
  await settle()
  sand.script[root] = [f"x = bash('elsewhere', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  assert [one[3] for one in said(log, "bash")] == [two]


async def test_an_act_is_on_a_chain_and_its_facts_are_on_its_scope() -> None:
  """An act is on a chain, and its facts are on its scope."""
  _, log, _ = born()
  two = engine.chain("two")
  await settle()
  act = engine.bash("echo hi", on=two)
  await settle()
  assert (said(log, "bash")[0][3], engine.scope(act)) == (two, two)
  assert {engine.scope(one[1]) for one in log if one[1] == act} == {two}


async def test_a_chain_is_on_no_chain_and_its_scope_is_itself() -> None:
  """A chain is on no chain, and its scope is itself."""
  _, log, root = born()
  two = engine.chain("two")
  word = said(log, "chain")[-1]
  assert (word[1], word[3], engine.scope(two)) == (two, "", two)
  assert engine.scope(root) == root


async def test_the_world_is_given_the_id_of_the_act_and_the_facts_about_the_act_carry_the_same_id() -> None:
  """The World is given the id of the act, and the facts about the act carry the same id."""
  sand, log, root = born()
  act = engine.bash("echo hi", on=root)
  await settle()
  assert [one[1] for one in sand.calls if one[0] == "bash"] == [act]
  assert {
    one[1] for one in log[log.index(engine.get(act)) :] if one[0] in ("out", "done", "started") and one[2] == WORLD
  } == {act}
  assert (await act).code == 0


async def test_a_prompt_that_a_rung_makes_on_another_chain_is_an_act_of_that_chain() -> None:
  """A prompt that a rung makes on another chain is an act of that chain."""
  sand, log, root = born()
  two = engine.chain("two")
  await settle()
  sand.script[root] = [f"p = prompt(int, 'hi', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  theirs = said(log, "prompt")[-1]
  assert theirs[3] == two
  held = engine.transcript(two)
  assert isinstance(held, list) and theirs in held
