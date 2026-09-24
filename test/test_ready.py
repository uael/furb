"""Ready, the word a rung holds."""

from conftest import DOOR, STANDS, Sand, life, plain, relived, said, settle, sown
from furb import engine


async def test_a_ready_says_the_word_a_rung_holds() -> None:
  """A ready says the word a rung holds."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  laid = engine.rung("<S1>hi</S1>\nk = S1", on=root)
  await laid
  sand.script[root] = ["<S2>\nthere\n</S2>\nclose(k + S2)"]
  act = engine.prompt(str, "greet", on=root)
  assert await act == "hithere\n"
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (binding,) = [a[1] for a in said(log, "rung") if a[2] == root]
  assert said(log, "ready") == [
    ("ready", laid, laid, "<S1>hi</S1>\nk = S1"),
    ("ready", binding, binding, f"{root}: Act[object] = Act({root!r})\n{act}: Act[str] = Act({act!r})"),
    ("ready", step, step, "<S2>\nthere\n</S2>\nclose(k + S2)"),
  ]


async def test_the_word_of_a_rung_enters_the_program_when_the_gate_accepts_it_or_the_chain_wrote_it() -> None:
  """The word of a rung enters the program of the chain when the gate accepts it or when the chain wrote it, and runs in the globals of the chain."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = BAD", "a = 1", "close(a + 1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  wrote = f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"
  (binding,) = [a[1] for a in said(log, "rung") if a[2] == root]
  refused, first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  assert engine.ask("program", root)[1] == {binding: wrote, first: "a = 1", second: "close(a + 1)"}
  assert [q[4] for q in engine.asked.values() if q[0] == "gate"] == ["a = BAD", "a = 1", "close(a + 1)"]
  assert engine.ask("ladder", root, act)[1] == "a = BAD\na = 1\nclose(a + 1)"
  assert (engine.modules[root][root], engine.modules[root][act], engine.modules[root]["a"]) == (root, act, 1)
  assert type(engine.outcomes[refused]).__name__ == "Refused"


async def test_the_word_of_a_rung_that_extends_the_engine_is_part_of_the_program() -> None:
  """The word of a rung that extends the engine is part of the program, so the extension returns in a later life."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [DOOR, "close(None)"]
  assert await engine.prompt(int, "a door", on=root) == 1
  await settle()
  _, program = engine.ask("program", root)
  assert isinstance(program, dict)
  assert DOOR in program.values()
  _, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert engine.ask("read", over, "note://a")[1] == "kept"


async def test_the_old_words_stay_in_the_program_and_in_the_turns_after_a_rung_rebinds_a_name() -> None:
  """The old words stay in the program and in the turns after a rung rebinds a name."""
  sand = sown()
  log, root = life(sand)
  old, new = "def twice(x):\n  return x * 2\nclose(None)", "def twice(x):\n  return x * 3"
  sand.script[root] = [old]
  act = engine.prompt(None, "bind it", on=root)
  assert await act is None
  laid = engine.rung(new, on=root)
  await laid
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (binding,) = [a[1] for a in said(log, "rung") if a[2] == root]
  _, program = engine.ask("program", root)
  assert program == {
    binding: f"{root}: Act[object] = Act({root!r})\n{act}: Act[None] = Act({act!r})",
    step: old,
    laid: new,
  }
  assert [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"] == [old]
  assert engine.turns(on=root)[-1][1].endswith(f"#{laid}\n{new}")
