"""Question, a fact that takes a name of its own and is answered."""

from conftest import Sand, acts, born, life, said, settle
from furb import engine
from furb.engine import OPERATOR, Act, Text


async def test_a_fact_that_takes_a_name_of_its_own_when_it_is_said_and_is_answered() -> None:
  """A fact that takes a name of its own when it is said and is answered, which is an act: answered now with a done, or later, with a started now and a done after it."""
  _, log, root = born()
  act = engine.bash("echo hi", on=root)
  assert engine.get(act) == said(log, "bash")[0] and [a[1] for a in said(log, "started")][-1] == act
  assert (await act).code == 0
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  asked = said(log, "read")[0]
  assert engine.get(asked[1]) == asked and [a for a in said(log, "started") if a[1] == asked[1]] == []
  assert engine.peek(asked[1]) == Text("/w/a.txt", "one\ntwo\n")


async def test_a_question_is_a_fact_whose_about_is_its_own_name_and_whose_first_word_is_the_chain() -> None:
  """A question is a fact whose about is its own name and whose first word is the chain it is on."""
  _, log, root = born()
  act = engine.bash("echo hi", on=root)
  engine.read("a.txt", on=root)
  made, asked = said(log, "bash")[0], said(log, "read")[0]
  assert (made[1], made[3]) == (act, root)
  assert (asked[1], asked[3]) == (asked[1], root)
  assert engine.scope(act) == root and engine.scope(asked[1]) == root


async def test_every_question_takes_a_name() -> None:
  """Every question takes a name, what is answered now as well as what is answered later."""
  _, log, root = born()
  act = engine.bash("echo hi", on=root)
  engine.read("a.txt", on=root)
  assert act == "bash1"
  assert [a[1] for a in said(log, "read")] == ["read1"]


async def test_a_question_is_named_by_its_kind() -> None:
  """A question is named by its kind, so a fact is a question when the act it is about is named under its kind, which is what question says."""
  _, log, root = born()
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  made = said(log, "bash")[0]
  assert engine.question(made) and made[1] == made[0] + "1"
  assert not engine.question(said(log, "tell")[0])
  assert not engine.question(said(log, "out")[0])


async def test_the_name_of_an_act() -> None:
  """The name of an act is its kind and how many acts of that kind the life has made with it, so the root is chain1, the first command is bash1 and the first prompt is prompt1, and python binds each name as it is."""
  _, log, root = born("x = bash('echo hi')\ny = bash('echo ho')\nclose(1)", "close(bash1 + bash2 == 'bash1bash2')")
  asking = engine.prompt(int, "run it", on=root)
  assert await asking == 1
  await settle()
  assert (root, asking) == ("chain1", "prompt1")
  assert said(log, "rung")[0][1] == "rung1"
  assert [a[1] for a in said(log, "bash")] == ["bash1", "bash2"]
  assert await Act("prompt2") is True
  await settle()
  assert [a[1] for a in said(log, "prompt")] == ["prompt1", "prompt2", "prompt3"]
  assert all(name.isidentifier() for name in acts(log))
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert twin == "chain2"
  assert [a[1] for a in said(log, "bash")] == ["bash1", "bash2"]


async def test_a_read_takes_its_number_as_a_command_does() -> None:
  """A read takes its number as a command does, so the first read is read1, whoever made it."""
  sand, log, root = born()
  engine.read("a.txt", on=root)
  sand.script[root] = ["x = bash('echo hi')\nt = read('a.txt')\nclose(len(t.lines))"]
  asking = engine.prompt(int, "read it", on=root)
  assert await asking == 2
  await settle()
  assert [(a[1], a[2]) for a in said(log, "read")] == [("read1", OPERATOR), ("read2", "rung1")]
  later = Sand()
  again, _ = life(later, list(sand.record))
  await settle(300)
  assert [a[1] for a in said(again, "read")] == ["read1", "read2"] and engine.peek("prompt1") == 2
  assert [a for a in later.calls if a[0] in ("read", "reply")] == []


async def test_the_generator_that_settles_an_await_is_named_after_that_act() -> None:
  """The generator that settles an await of an act from outside a run is named after that act and the task that awaits it, which is the name of no question."""
  _, log, root = born("x = bash('echo hi')\nclose(1)")
  asking = engine.prompt(int, "run it", on=root)
  assert await asking == 1
  assert [a for a in log if "waits" in a[1]] == []
  assert [name for name in acts(log) if "waits" in name] == []
  assert not engine.question(("prompt", f"{asking} waits 1", OPERATOR))
