"""turns, what a model reads of a chain."""

from conftest import STANDS, Sand, heads, life, paragraphs, said, settle
from furb import engine
from furb.engine import span

CARRY = ("tell", "pause", "wake", "cancel", "close")
"""The kinds of fact that carry notes, of which the turns are folded."""


def notes(held: list[tuple]) -> list[list[object]]:
  """The notes of every fact of a transcript that carries notes, in the order they were said."""
  return [a[4] if a[0] == "close" else a[3] for a in held if a[0] in CARRY]


async def test_the_turns_of_a_chain_folded_from_what_it_has_heard() -> None:
  """The turns of a chain, folded from what it has heard."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  got = engine.turns(on=root)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert got == engine.turns_of(held)
  assert got == [
    (
      "user",
      f"#{root} root\n{root}: Act[object] = Act({root!r})\n\n#{root} stands {STANDS!r}\n\n#rung1\nk = 1\n\n#rung1 closed",
      None,
      None,
    )
  ]


async def test_a_turns_asked_from_a_run_tells_how_many_turns_there_are() -> None:
  """A turns asked from a run tells how many turns there are, and never the turns."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(len(turns()))"]
  assert await engine.prompt(int, "count them", on=root) == 3
  assert engine.turns(on=root)[-1][1] == "#turns 3\n\n#prompt1 closed 3"


async def test_the_turns_of_what_a_chain_has_heard() -> None:
  """The turns of what a chain has heard: every fact that carries notes stands as a paragraph of them, and nothing else stands at all."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  carried = notes(held)
  assert all(isinstance(note, str) for one in carried for note in one)
  assert paragraphs(engine.turns(on=root)) == ["\n".join(str(note) for note in one) for one in carried]
  assert {a[0] for a in held if a[0] not in CARRY} == {
    "rung",
    "ready",
    "run",
    "ran",
    "done",
    "prompt",
    "holds",
    "ask",
    "answer",
  }


async def test_the_turn_a_model_was_answered_with_closes_the_turn_of_the_operator() -> None:
  """The turn a model was answered with closes the turn of the operator and stands as the turn it is, and a text stands by the lines it has not seen, which the one that tells it says the show of."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\n"}, stands=STANDS)
  log, root = life(sand)
  word = "read('n.txt', span(2, 2))\nclose(1)"
  sand.script[root] = [word]
  assert await engine.prompt(int, "read it", on=root) == 1
  got = engine.turns(on=root)
  assert [role for role, *_ in got] == ["user", "assistant", "user"]
  assert heads(got[:1])[-1] == f"#{said(log, 'rung')[0][1]} advance on prompt1"
  assert got[1] == said(log, "answer")[0][3] == ("assistant", word, (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
  assert got[2][1] == "#read n.txt\n# /w/n.txt, 0 known\n# 2 two\n\n#prompt1 closed 1"
  assert span(2, 2)(["one", "two"]) == [2]


async def test_the_chain_answers_for_its_turns() -> None:
  """The chain answers for its turns, folded from what it has heard."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("cd('/x')", on=root)
  got = engine.turns(on=root)
  assert [role for role, *_ in got] == ["user"]
  assert heads(got) == [f"#{root} root", f"#{root} stands {STANDS!r}", "#rung1", "#cd /x", "#rung1 closed"]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  asking = said(held, "turns")[-1]
  assert [a[2] for a in said(held, "done") if a[1] == asking[1]] == [root]
