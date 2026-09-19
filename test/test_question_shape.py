"""Question, a fact that takes a name of its own and is answered."""

import pytest

from conftest import STANDS, Sand, life, said, settle, sown
from furb import engine
from furb.engine import Text


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_fact_that_takes_a_name_of_its_own_when_it_is_said_and_is_answered() -> None:
  """A fact that takes a name of its own when it is said and is answered: an act if it lives, a query if it does not."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert engine.acts[act] == said(log, "bash")[0]
  assert (await act).code == 0
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  asked = said(log, "read")[0]
  assert engine.asked[asked[1]] is asked and asked[1] not in engine.acts
  assert engine.outcomes[asked[1]] == Text("/w/a.txt", "one\ntwo\n")


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_question_is_a_fact_whose_about_is_its_own_name_and_whose_first_word_is_the_chain() -> None:
  """A question is a fact whose about is its own name and whose first word is the chain it is on."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  engine.read("a.txt", on=root)
  made, asked = said(log, "bash")[0], said(log, "read")[0]
  assert (made[1], made[3]) == (act, root)
  assert (asked[1], asked[3]) == (asked[1], root)
  assert engine.scope(act) == root and engine.scope(asked[1]) == root


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_an_act_or_a_query_takes_a_name() -> None:
  """An act or a query takes a name."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  engine.read("a.txt", on=root)
  assert act == "bash://operator.2"
  assert [a[1] for a in said(log, "read")] == ["read://operator.3"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_question_is_named_by_its_kind() -> None:
  """A question is named by its kind, so a fact is a question when the act it is about is named under its kind, which is what question says."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  made = said(log, "bash")[0]
  assert engine.question(made) and made[1].startswith(made[0] + "://")
  assert not engine.question(said(log, "tell")[0])
  assert not engine.question(said(log, "exited")[0])


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_name_of_a_question() -> None:
  """The name of a question: its kind, the lineage of the one that made it, which is the lineage of that act when the maker is one and of its name otherwise, and how many that one has made, which the life counts for each maker."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  asking = engine.prompt(int, "run it", on=root)
  assert await asking == 1
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  assert (root, asking) == ("chain://operator.1", "prompt://operator.2")
  assert (step, command) == ("rung://operator.2.1", "bash://operator.2.1.1")
  twin = engine.chain("twin", source=root)
  await settle(300)
  retold = next(a for a in said(log, "rung") if a[3] == twin)
  assert retold[5] == step and engine.lineage(retold[1]).startswith(engine.lineage(twin))
  assert [a[1] for a in said(log, "bash")] == [command]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_question_is_named_by_its_kind_so_a_rung_is_rung() -> None:
  """A question is named by its kind, so a rung is rung://, a prompt prompt://, a read read://, and the generator that settles an await of an act from outside a run is named waits:// under that act, which is the name of no question."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  asking = engine.prompt(int, "run it", on=root)
  assert await asking == 1
  engine.read("a.txt", on=root)
  assert asking.startswith("prompt://")
  assert said(log, "rung")[0][1].startswith("rung://")
  assert said(log, "read")[0][1].startswith("read://")
  assert [a for a in log if a[1].startswith("waits://")] == []
  assert [name for name in (*engine.acts, *engine.asked) if name.startswith("waits://")] == []
