"""Ready, the word a rung holds."""

from conftest import DOOR, STANDS, Sand, life, plain, relived, said, settle, sown, text_of
from furb import engine


async def test_a_ready_says_the_word_a_rung_holds() -> None:
  """A ready says the word a rung holds."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  laid = engine.rung("k = 1", on=root)
  await laid
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  _, asking, *_ = said(log, "rung")[-1]
  assert [a[3:] for a in said(log, "ready")] == [("k = 1",), ("close(k + 1)",)]
  assert [a[1] for a in said(log, "ready")] == [laid, asking]


async def test_the_word_of_a_rung_enters_the_program_of_the_chain() -> None:
  """The word of a rung enters the program of the chain, and it runs in the globals of the chain when the gate accepts it."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = BAD", "a = 1", "close(a + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  _, program = engine.ask("program", root, root)
  assert isinstance(program, dict)
  assert list(program.values()) == ["a = BAD", "a = 1", "close(a + 1)"]
  assert engine.modules[root]["a"] == 1


async def test_the_word_of_a_rung_that_extends_the_engine_is_part_of_the_program() -> None:
  """The word of a rung that extends the engine is part of the program, so the extension returns in a later life."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [DOOR, "close(None)"]
  assert await engine.prompt(int, "a door", on=root) == 1
  await settle()
  _, program = engine.ask("program", root, root)
  assert isinstance(program, dict)
  assert DOOR in "\n".join(program.values())
  _, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert engine.read("note://a", on=over).content == "kept"


async def test_the_old_words_stay_in_the_program_and_in_the_turns_after_a_rung_rebinds_a_name() -> None:
  """The old words stay in the program and in the turns after a rung rebinds a name."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["def twice(x):\n  return x * 2\nclose(None)"]
  assert await engine.prompt(None, "bind it", on=root) is None
  await engine.rung("def twice(x):\n  return x * 3", on=root)
  _, program = engine.ask("program", root, root)
  assert isinstance(program, dict)
  assert list(program.values()) == ["def twice(x):\n  return x * 2\nclose(None)", "def twice(x):\n  return x * 3"]
  assert "def twice(x):\n  return x * 2\nclose(None)" in [text_of(turn) for turn in engine.turns(on=root)]
