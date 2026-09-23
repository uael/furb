"""Question, a fact that takes a name of its own and is answered."""

from conftest import STANDS, Sand, life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Act, Text


async def test_a_fact_that_takes_a_name_of_its_own_when_it_is_said_and_is_answered() -> None:
  """A fact that takes a name of its own when it is said and is answered: an act if it lives, a query if it does not."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert engine.acts[act] == said(log, "bash")[0]
  assert (await act).code == 0
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  asked = said(log, "read")[0]
  assert engine.asked[asked[1]] == asked and asked[1] not in engine.acts
  assert engine.outcomes[asked[1]] == Text("/w/a.txt", "one\ntwo\n")


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


async def test_an_act_or_a_query_takes_a_name() -> None:
  """An act or a query takes a name."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  engine.read("a.txt", on=root)
  assert act == "bash1"
  assert [a[1] for a in said(log, "read")] == ["read@operator.3"]


async def test_a_question_is_named_by_its_kind() -> None:
  """A question is named by its kind, so a fact is a question when the act it is about is named under its kind, which is what question says."""
  sand = sown()
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  made = said(log, "bash")[0]
  assert engine.question(made) and made[1] == made[0] + "1"
  assert not engine.question(said(log, "tell")[0])
  assert not engine.question(said(log, "exited")[0])


async def test_the_name_of_an_act() -> None:
  """The name of an act is its kind and how many acts of that kind the life has made with it, so the root is chain1, the first command is bash1 and the first prompt is prompt1, and python binds each name as it is."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\ny = bash('echo ho')\nclose(1)", "close(bash1 + bash2 == 'bash1bash2')"]
  asking = engine.prompt(int, "run it", on=root)
  assert await asking == 1
  await settle()
  assert (root, asking) == ("chain1", "prompt1")
  assert said(log, "rung")[0][1] == "rung1"
  assert [a[1] for a in said(log, "bash")] == ["bash1", "bash2"]
  assert await Act("prompt2") is True
  await settle()
  assert [a[1] for a in said(log, "prompt")] == ["prompt1", "prompt2", "prompt3"]
  assert all(name.isidentifier() for name in engine.acts)
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert twin == "chain2"
  assert [a[1] for a in said(log, "bash")] == ["bash1", "bash2"]


async def test_the_name_of_a_query() -> None:
  """The name of a query is its kind, @, the one that made it, a dot, and how many questions that one has made with it, as read@rung1.2, so a query takes no number from the acts, and a query of the operator that a later life does not ask again moves no name."""
  sand = sown()
  log, root = life(sand)
  assert engine.cwd(on=root) == "/w"
  sand.script[root] = ["x = bash('echo hi')\nt = read('a.txt')\nclose(len(t.lines))"]
  asking = engine.prompt(int, "read it", on=root)
  assert await asking == 2
  await settle()
  assert [name for name, a in engine.asked.items() if a[0] == "cwd" and a[2] == OPERATOR] == ["cwd@operator.2"]
  assert (asking, said(log, "bash")[0][1]) == ("prompt1", "bash1")
  assert [name for name in engine.asked if name.startswith("read@")] == ["read@rung1.2"]
  later = Sand(stands=STANDS)
  again, _ = life(later, list(sand.record))
  await settle(300)
  assert [a[1] for a in said(again, "prompt")] == [a[1] for a in said(log, "prompt")] == ["prompt1", "prompt2"]
  assert [name for name in engine.asked if name.startswith("read@")] == ["read@rung1.2"]
  assert engine.outcomes["prompt1"] == 2
  assert [a for a in later.calls if a[0] == "read"] == []
  assert [a[1] for a in later.calls if a[0] == "ask"] == [a[1] for a in sand.calls if a[0] == "ask"][1:]


async def test_the_generator_that_settles_an_await_is_named_after_that_act() -> None:
  """The generator that settles an await of an act from outside a run is named after that act and the task that awaits it, which is the name of no question."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  asking = engine.prompt(int, "run it", on=root)
  assert await asking == 1
  assert [a for a in log if "waits" in a[1]] == []
  assert [name for name in (*engine.acts, *engine.asked) if "waits" in name] == []
  assert not engine.question(("prompt", f"{asking} waits 1", OPERATOR))
