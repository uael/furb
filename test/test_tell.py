"""tell, what a query tells of itself."""

from collections.abc import Generator

from conftest import STANDS, Sand, life, named, said, settle, verb
from furb import engine
from furb.engine import OPERATOR

COUNTER = (
  "def counting(id):\n"
  "  n = 0\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('bump', qid, _, _, by):\n"
  "        n += by\n"
  "        yield 'done', qid, n\n"
  "\n"
  "def bump(by=1, on=''):\n"
  "  _, got = ask('bump', on, by)\n"
  "  tell('bump', f'{by} gave {got}')\n"
  "  return got\n"
  "\n"
  "counter = act('counter', '', counting)\n"
)
"""A word of a rung that writes a query that changes a state, the count an act of its own holds, and tells it."""


async def test_what_a_query_that_shows_a_content_or_changes_a_state_tells_of_itself() -> None:
  """What a query that shows a content or changes a state tells of itself: a paragraph headed with its kind, its words and what it was answered, said on the run that asked it, and nothing at all outside a run; a query that only reads a value tells nothing, since the word that asked it holds the value, which it debugs to see."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung(COUNTER, on=root)
  word = "got = bump(2)\nseen = [peek(counter), turns(), clock(), chance(), gate('k = 1')]\n"
  sand.script[root] = [word + "debug(t'{seen[2]}')\nclose(got)"]
  assert await engine.prompt(int, "ask things", on=root) == 2
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == "prompt1"]
  kinds = ["bump", "peek", "turns", "clock", "chance", "program", "gate"]
  assert [a[0] for a in engine.asked.values() if a[2] == step] == kinds
  told = [a[3] for a in said(log, "tell") if a[1] == step and a[2] == step]
  assert told == [["#bump 2 gave 2"], [f"#{step} debugged seen[2] = 1001.0"]]
  assert "#bump 2 gave 2" in engine.turns(on=root)[-1][1]
  assert [one for one in named(engine.turns(on=root)) if one in kinds[1:]] == []
  seen = engine.modules[root]["seen"]
  assert isinstance(seen, list) and seen[2:] == [1001.0, 2 / 7, []]
  was = engine.turns(on=root)
  assert verb("bump", root)(3) == 5
  assert engine.clock(on=root) == 1003.0
  assert engine.turns(on=root) == was


def still(id: str) -> Generator[tuple | None, tuple]:
  """The ear of an act that answers the first reading of the clock it hears with a stopped clock, and returns."""
  while True:
    match (yield):
      case ("clock", qid, *_):
        yield "done", qid, 0.5
        return


async def test_a_query_is_put_to_the_living_generators_in_turn_the_acts_first_and_the_outside_last() -> None:
  """A query is put to the living generators in turn, the acts first and the outside last, and it stops at the first answer, so the World is asked for nothing that the engine knows."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert engine.ask("ladder", root, act)[1] == ""
  assert engine.turns(on=root) != []
  assert engine.peek(act, on=root) is None
  assert [a[0] for a in sand.calls] == ["stand", "start"]
  engine.act("still", root, still)
  assert engine.clock(on=root) == 0.5
  assert [a[0] for a in sand.calls] == ["stand", "start"]
  assert engine.clock(on=root) == 1001.0
  assert [a[0] for a in sand.calls] == ["stand", "start", "clock"]
