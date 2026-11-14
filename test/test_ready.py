"""Ready, the word a rung holds."""

from conftest import DOOR, Sand, born, fresh, plain, prompted, relived, said, settle
from furb import engine


async def test_a_ready_says_the_word_a_rung_holds() -> None:
  """A ready says the word a rung holds."""
  sand, log, root = born()
  word = "<s:hi>hi</s:hi>\nk = hi"
  laid = engine.rung(word, on=root)
  await laid
  sand.script[root] = ["<s:there>\nthere\n</s:there>\nclose(k + there)"]
  act = engine.prompt(str, "greet", on=root)
  assert await act == "hithere\n"
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  asked = "<s:there>\nthere\n</s:there>\nclose(k + there)"
  assert said(log, "ready") == [("ready", laid, laid, word), ("ready", step, step, asked)]


async def test_the_word_of_a_rung_enters_the_program_of_the_chain_at_its_ready() -> None:
  """The word of a rung enters the program of the chain at its ready, and runs in the globals of the chain."""
  _, log, root = born("a = 1\ndebug(t'go')", "close(a + 1)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  assert engine.program(root) == {
    f"{first}_told": fresh(root, prompted(act, "int", "count"), f"#{first} advance on {act}"),
    first: "a = 1\ndebug(t'go')",
    f"{second}_told": f"#{first} debugged\n{first}_debug = 'go'\n\n#{second} advance on {act}",
    second: "close(a + 1)",
  }
  assert [a[3] for a in said(log, "ready") if a[1] in (first, second)] == ["a = 1\ndebug(t'go')", "close(a + 1)"]
  assert engine.read(act, on=root).content == "a = 1\ndebug(t'go')\nclose(a + 1)"
  assert (engine.module(root)[root], engine.module(root)[act], engine.module(root)["a"]) == (root, act, 1)


async def test_the_word_of_a_rung_that_extends_the_engine_is_part_of_the_program() -> None:
  """The word of a rung that extends the engine is part of the program, so the extension returns in a later life."""
  sand, _, root = born(DOOR, "close(None)")
  assert await engine.prompt(int, "a door", on=root) == 1
  await settle()
  program = engine.program(root)
  assert DOOR in program.values()
  _, over = await relived(Sand(), plain(sand.record))
  assert engine.read("note://a", on=over).content == "kept"


async def test_the_old_words_stay_in_the_program_and_in_the_turns_after_a_rung_rebinds_a_name() -> None:
  """The old words stay in the program and in the turns after a rung rebinds a name."""
  sand, log, root = born()
  old, new = "def twice(x):\n  return x * 2\nclose(None)", "def twice(x):\n  return x * 3"
  sand.script[root] = [old]
  act = engine.prompt(None, "bind it", on=root)
  assert await act is None
  laid = engine.rung(new, on=root)
  await laid
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  program = engine.program(root)
  wrote = fresh(root, prompted(act, "None", "bind it"), f"#{step} advance on {act}")
  assert program == {f"{step}_told": wrote, step: old, laid: new}
  assert [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"] == [old]
  assert engine.turns(on=root)[-1][1] == new
