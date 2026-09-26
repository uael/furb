"""turns, what a model reads of a chain."""

from conftest import STANDS, Sand, acts, heads, life, paragraphs, rows, said, settle, takes
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
  assert got == [
    ("user", f"#{root} root\n{root}: Act[object] = Act({root!r})\n\n{takes(root)}\n\n#rung1\nk = 1", None, None)
  ]


async def test_the_turns_of_what_a_chain_has_heard() -> None:
  """The turns of what a chain has heard: every fact that carries notes stands as a paragraph of them, and nothing else stands at all."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  held = engine.transcript(root)
  carried = notes(held)
  assert all(isinstance(note, str) for one in carried for note in one)
  assert paragraphs(engine.turns(on=root)) == ["\n".join(str(note) for note in one) for one in carried]
  assert {a[0] for a in held if a[0] not in CARRY} == {
    "stand",
    "rung",
    "gate",
    "ready",
    "run",
    "module",
    "started",
    "done",
    "prompt",
    "reply",
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
  assert (
    got[1] == engine.peek(said(log, "reply")[0][1]) == ("assistant", word, (0, 0, 0, 0, 0.0), [f"signed {len(word)}"])
  )
  assert got[2][1] == "#read n.txt\n# /w/n.txt, 0 known\n# 2 two\n\n#prompt1 closed 1"
  assert span(2, 2)(["one", "two"]) == [2]


async def test_turns_reads_the_transcript_of_the_chain_and_asks_nothing() -> None:
  """turns reads the transcript of the chain and asks nothing, so no act is made and the journal keeps nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("cd('/x')", on=root)
  made, kept = dict(acts(log)), len(sand.record)
  got = engine.turns(on=root)
  assert [role for role, *_ in got] == ["user"]
  assert heads(got) == [f"#{root} root", rows(root)[0], "#rung1", "#cd /x"]
  assert acts(log) == made and len(sand.record) == kept


async def test_a_user_turn_packs_one_paragraph_for_each_thing_told_since_the_last_reply() -> None:
  """A user turn packs one paragraph for each thing told since the last reply, in order."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  got = engine.turns(on=root)
  assert len(got) == 1
  assert heads(got) == ["#chain1 root", rows("chain1")[0], "#rung1"]
  assert got[0][1] == "\n\n".join(paragraphs(got))


async def test_the_turns_of_a_chain_only_grow() -> None:
  """The turns of a chain only grow."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  was = engine.turns(on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  now = engine.turns(on=root)
  assert paragraphs(now)[: len(paragraphs(was))] == paragraphs(was)
  assert len(paragraphs(now)) > len(paragraphs(was))


async def test_a_turn_once_phrased_is_phrased_the_same_on_every_later_reply() -> None:
  """A turn once phrased is phrased the same on every later reply."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  engine.grant(usd=10.0, on=root)
  sand.script[root] = ["a = 1", "b = 2", "close(3)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  asks = list(sand.turns.values())
  assert [len(one) for one in asks] == [1, 3, 5]
  assert [one[: len(asks[0])] for one in asks] == [asks[0]] * 3
  assert [one[: len(asks[1])] for one in asks[1:]] == [asks[1]] * 2


async def test_the_turns_are_folded_whole_at_each_reply() -> None:
  """The turns are folded whole at each reply, and a user turn holds every paragraph told before its reply and since the reply before it, so a paragraph told while a reply is in flight goes to the turn after the answer."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')", "y = 2", "close(3)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  asks = list(sand.turns.values())
  assert [[role for role, *_ in one] for one in asks] == [
    ["user"],
    ["user", "assistant", "user"],
    ["user", "assistant", "user", "assistant", "user"],
  ]
  assert [one[: len(asks[1])] for one in asks[1:]] == [asks[1]] * 2
  assert heads(asks[1][-1:]) == ["#bash1 slow", "#rung3 advance on prompt1"]
  assert heads(asks[2][-1:]) == ["#bash1 exited 0", "#rung5 advance on prompt1"]
  assert said(log, "bash")[0][1] == "bash1"


async def test_a_reply_appends_the_response_as_an_assistant_turn() -> None:
  """A reply appends the response as an assistant turn."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  got = engine.turns(on=root)
  assert got[-2] == engine.peek(said(log, "reply")[0][1])
  assert got[-2][0] == "assistant" and got[-1][0] == "user"


async def test_the_next_reply_tells_everything_that_the_step_told() -> None:
  """The next reply tells everything that the step told."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["raise ValueError('boom')", "close(1)"]
  assert await engine.prompt(int, "try", on=root) == 1
  await settle()
  second = sand.turns["reply2"]
  assert heads(second[-1:]) == ["#rung1 raised ValueError('boom')", "#rung3 advance on prompt1"]


async def test_the_turns_hold_every_answer_of_the_model_as_the_turn_it_is() -> None:
  """The turns hold every answer of the model as the turn it is."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(2)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  got = engine.turns(on=root)
  assert [turn for turn in got if turn[0] == "assistant"] == [engine.peek(a[1]) for a in said(log, "reply")]


async def test_the_turns_of_a_chain_show_every_act_the_model_made() -> None:
  """The turns of a chain show every act the model made, with its result, and turns is how they are read."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  got = engine.turns(on=root)
  assert [one for one in paragraphs(got) if one.startswith(f"#{command} ")] == [
    f"#{command} echo hi\n{command}: Act[Exit] = Act('{command}')",
    f"#{command} exited 0\n# {command}/stdout, 0 known\n# 1 ran echo hi",
  ]


async def test_the_turns_end_with_a_user_turn() -> None:
  """The turns end with a user turn, empty when nothing was told since the last answer."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.turns(on=root)[-1][0] == "user"
  sand.script[root] = ["await wait(9)\nclose(1)"]
  act = engine.prompt(int, "wait", on=root)
  await settle()
  got = engine.turns(on=root)
  assert got[-1] == ("user", "", None, None)
  assert [role for role, *_ in got] == ["user", "assistant", "user"]
  engine.cancel(act)
