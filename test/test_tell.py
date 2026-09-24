"""tell, what a query tells of itself."""

from conftest import life, named, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Text


async def test_what_a_query_that_shows_a_text_or_changes_a_state_tells_of_itself() -> None:
  """What a query that shows a text or changes a state tells of itself: a paragraph headed with its kind, its words and what it was answered, said on the run that asked it, and nothing at all outside a run; a query that only reads a value tells nothing, since the word that asked it holds the value, which it debugs to see."""
  sand = sown()
  log, root = life(sand)
  word = (
    "read('a.txt')\ncd('/x')\nx = bash('echo hi')\nseen = [peek(x), turns(), clock(), chance(), gate('k = 1'), cwd()]\n"
  )
  sand.script[root] = [word + "debug(t'{seen[2]}')\nclose(1)"]
  assert await engine.prompt(int, "ask things", on=root) == 1
  await settle()
  _, step, *_ = said(log, "rung")[0]
  kinds = ["read", "cd", "peek", "turns", "clock", "chance", "program", "gate", "cwd"]
  assert [a[0] for a in engine.asked.values() if a[2] == step] == kinds
  told = [a[3] for a in said(log, "tell") if a[1] == step and a[2] == step]
  assert [notes[0] for notes in told] == ["#read a.txt", "#cd /x", f"#{step} debugged seen[2] = 1001.0"]
  assert [len(notes) for notes in told] == [2, 1, 1] and told[0][1][0] == Text("/w/a.txt", "one\ntwo\n")
  assert (
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two\n\n#cd /x\n\n#bash1 echo hi" in engine.turns(on=root)[-1][1]
  )
  assert [one for one in named(engine.turns(on=root)) if one in kinds[2:]] == []
  seen = engine.modules[root]["seen"]
  assert isinstance(seen, list) and seen[2:] == [1001.0, 2 / 7, [], "/x"]
  was = engine.turns(on=root)
  assert engine.cd("/w", on=root) == "/w"
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.clock(on=root) == 1003.0
  assert engine.turns(on=root) == was


async def test_a_query_is_put_to_the_living_generators_in_turn_the_acts_first_and_the_outside_last() -> None:
  """A query is put to the living generators in turn, the acts first and the outside last, and it stops at the first answer, so the World is asked for nothing that the engine knows."""
  sand = sown()
  _, root = life(sand)
  assert engine.cwd(on=root) == "/w"
  assert engine.turns(on=root) != []
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert engine.read(act, on=root) == Text(act, "")
  assert [a[0] for a in sand.calls] == ["stand", "start"]
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "start", "read"]
