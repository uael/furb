"""tell, what a question answered now tells of itself."""

from conftest import life, named, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Text


async def test_what_a_question_answered_now_that_shows_a_text_or_changes_a_state_tells_of_itself() -> None:
  """What a question answered now that shows a text or changes a state tells of itself: a paragraph headed with its kind, its words and what it was answered, said on the run that asked it, and nothing at all outside a run; one that only reads a value tells nothing, since the word that asked it holds the value, which it debugs to see."""
  sand = sown()
  log, root = life(sand)
  word = (
    "read('a.txt')\ncd('/x')\nx = bash('echo hi')\nseen = [peek(x), turns(), clock(), chance(), gate('k = 1'), cwd()]\n"
  )
  sand.script[root] = [word + "debug(t'{seen[2]}')\nclose(1)"]
  assert await engine.prompt(int, "ask things", on=root) == 1
  await settle()
  _, step, *_ = said(log, "rung")[0]
  kinds = ["reply", "read", "cd", "bash", "clock", "chance", "gate"]
  assert [a[0] for a in engine.transcript(root) if engine.question(a) and a[2] == step] == kinds
  told = [a[3] for a in said(log, "tell") if a[1] == step and a[2] == step]
  assert [notes[0] for notes in told] == ["#read a.txt", "#cd /x", f"#{step} debugged seen[2] = 1001.0"]
  assert [len(notes) for notes in told] == [2, 1, 1] and told[0][1][0] == Text("/w/a.txt", "one\ntwo\n")
  assert (
    "#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n# 2 two\n\n#cd /x\n\n#bash1 echo hi" in engine.turns(on=root)[-1][1]
  )
  assert [one for one in named(engine.turns(on=root)) if one in ("clock", "chance", "gate")] == []
  seen = engine.module(root)["seen"]
  assert isinstance(seen, list) and seen[2:] == [1001.0, 2 / 7, [], "/x"]
  was = engine.turns(on=root)
  assert engine.cd("/w", on=root) == "/w"
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.clock(on=root) == 1003.0
  assert engine.turns(on=root) == was


async def test_a_question_is_put_to_the_ears_of_the_engine_before_those_of_the_outside() -> None:
  """A question is offered to the ears of acts before the ears of the outside, so the World is asked for nothing that the engine knows."""
  sand = sown()
  _, root = life(sand)
  assert engine.cwd(on=root) == "/w"
  assert engine.turns(on=root) != []
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert engine.read(act, on=root) == Text(act, "")
  assert [a[0] for a in sand.calls] == ["stand", "prompt"]
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "prompt", "read"]
