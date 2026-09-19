"""tell, what a query tells of itself."""

from conftest import life, said, settle, sown, tags
from furb import engine
from furb.engine import Text


async def test_what_a_query_tells_of_itself() -> None:
  """What a query tells of itself: a tag of its own name, with its words and what it was answered, said on the run that asked it, and nothing at all outside a run."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["read('a.txt')\ncwd()\nclose(1)"]
  assert await engine.prompt(int, "ask things", on=root) == 1
  await settle()
  _, step, *_ = said(log, "rung")[0]
  told = [tag for a in said(log, "tell") if a[1] == step and a[2] == step for tag in a[3]]
  assert [(tag[0], tag[1]) for tag in told] == [("read", [("path", "a.txt")]), ("cwd", [("path", "/w")])]
  was = tags(engine.turns(on=root))
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.cwd(on=root) == "/w"
  assert tags(engine.turns(on=root)) == was


async def test_a_query_is_put_to_the_living_generators_in_turn_the_acts_first_and_the_outside_last() -> None:
  """A query is put to the living generators in turn, the acts first and the outside last, and it stops at the first answer, so the World is asked for nothing that the engine knows."""
  sand = sown()
  _, root = life(sand)
  assert engine.cwd(on=root) == "/w"
  assert engine.turns(on=root) != []
  assert engine.read(root, on=root) == Text(root, "")
  assert [a[0] for a in sand.calls] == ["stand"]
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "read"]
