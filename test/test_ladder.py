"""Ladder, the question of the program of the ladder of a prompt."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_ladder_is_the_question_of_the_program_of_the_ladder_of_a_prompt() -> None:
  """A ladder is the question of the program of the ladder of a prompt, which the chain it is on answers."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  asked, got = engine.ask("ladder", root, act)
  assert asked == ("ladder", asked[1], OPERATOR, root, act)
  assert got == "a = 1\nclose(a + 1)"
  assert [a[2] for a in said(log, "done") if a[1] == asked[1]] == [root]
  assert [a[0] for a in sand.calls] == ["stand", "ask", "ask"]


async def test_the_chain_answers_a_ladder_of_a_name_that_is_one_of_the_prompts_it_has_heard_on_itself() -> None:
  """The chain answers a ladder of a name that is one of the prompts it has heard on itself, and of no other name."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  assert engine.ask("ladder", root, act)[1] == "a = 1\nclose(a + 1)"
  other = engine.chain("other")
  await settle()
  assert engine.ask("ladder", other, act)[1] is None
  for path in ("", "nowhere://x", root, *[a[1] for a in said(log, "rung")]):
    assert engine.ask("ladder", root, path)[1] is None


async def test_the_program_of_a_ladder_holds_the_word_of_every_rung_of_it_in_order() -> None:
  """The program of a ladder holds the word of every rung of it in order, the words the gate refused among them, and none at all for a prompt that ran no word."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["x = BAD", "x = wait(0)\nclose(1)", "close(None)"]
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  assert engine.ask("ladder", root, act)[1] == "x = BAD\nx = wait(0)\nclose(1)"
  bare = engine.prompt(int, "later", to=OPERATOR, on=root)
  assert engine.ask("ladder", root, bare)[1] == ""


async def test_a_ladder_leaves_out_the_word_of_the_rung_that_asks_it() -> None:
  """A ladder leaves out the word of the rung that asks it, since a rung is no part of the program it reads."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  word = "close(ask('ladder', '', get(acting())[2])[1])"
  sand.script[root] = ["a = 1", word, "close(None)"]
  act = engine.prompt(str, "read it", on=root)
  assert await act == "a = 1"
  await settle()
  assert engine.ask("ladder", root, act)[1] == f"a = 1\n{word}"


async def test_a_ladder_given_a_word_edits_the_program_of_that_ladder() -> None:
  """A ladder given a word edits the program of that ladder: the chain makes the rungs of the ladder again from the word, and answers with it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  asked, got = engine.ask("ladder", root, act, "k = 21")
  await settle()
  assert asked == ("ladder", asked[1], OPERATOR, root, act, "k = 21") and got == "k = 21"
  assert [(a[2], a[4], a[5]) for a in said(log, "rung")] == [(act, "k = 21", "")]
  assert engine.modules[root]["k"] == 21
  assert engine.ask("ladder", root, act, "k = 21\nk = 22")[1] == "k = 21\nk = 22"
  await settle()
  first = said(log, "rung")[0][1]
  assert [(a[2], a[4], a[5]) for a in said(log, "rung")] == [
    (act, "k = 21", ""),
    (root, "k = 21", first),
    (act, "k = 22", ""),
  ]
  assert engine.ask("ladder", root, act)[1] == "k = 21\nk = 22" and engine.modules[root]["k"] == 22
  assert [a[0] for a in sand.calls] == ["stand", "start"]


async def test_a_ladder_given_a_word_leaves_out_the_word_of_the_rung_that_asks_it() -> None:
  """A ladder given a word leaves out the word of the rung that asks it, and that rung is no rung of the chain after it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  editor = "mine = get(acting())[2]\nask('ladder', '', mine, ask('ladder', '', mine)[1].replace('BAD', '2'))"
  sand.script[root] = ["k = 1", "ok = BAD", editor, "close((k, ok, ask('ladder', '', get(acting())[2])[1]))"]
  act = engine.prompt(object, "fix it", on=root)
  assert await act == (1, 2, "k = 1\nok = 2")
  await settle()
  binding, first, writer = (a[1] for a in said(log, "run")[:3])
  assert [(a[4], a[5]) for a in said(log, "run")] == [
    (f"chain1: Act[object] = Act('chain1')\n{act}: Act[object] = Act({act!r})", ""),
    ("k = 1", ""),
    (editor, ""),
    (f"chain1: Act[object] = Act('chain1')\n{act}: Act[object] = Act({act!r})", binding),
    ("k = 1", first),
    ("ok = 2", ""),
    ("close((k, ok, ask('ladder', '', get(acting())[2])[1]))", ""),
  ]
  _, program = engine.ask("program", root)
  assert isinstance(program, dict) and writer not in program
  assert list(program.values()) == [a[4] for a in said(log, "run")[3:]]
