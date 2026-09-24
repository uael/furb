"""turns_of, the fold of what a chain has heard."""

from conftest import STANDS, Sand, heads, life, paragraphs, said, settle
from furb import engine


async def test_a_user_turn_packs_one_paragraph_for_each_thing_told_since_the_last_ask() -> None:
  """A user turn packs one paragraph for each thing told since the last ask, in order."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  got = engine.turns(on=root)
  assert len(got) == 1
  assert heads(got) == ["#chain1 root", f"#chain1 stands {STANDS!r}", "#rung1"]
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


async def test_a_turn_once_phrased_is_phrased_the_same_on_every_later_ask() -> None:
  """A turn once phrased is phrased the same on every later ask."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  engine.grant(usd=10.0, on=root)
  sand.script[root] = ["a = 1", "b = 2", "close(3)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  asks = [a[5] for a in said(log, "ask")]
  assert [len(one) for one in asks] == [1, 3, 5]
  assert [one[: len(asks[0])] for one in asks] == [asks[0]] * 3
  assert [one[: len(asks[1])] for one in asks[1:]] == [asks[1]] * 2


async def test_the_turns_are_folded_whole_at_each_ask() -> None:
  """The turns are folded whole at each ask, and a user turn holds every paragraph told before its ask and since the ask before it, so a paragraph told while an ask is in flight goes to the turn after the answer."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')", "y = 2", "close(3)"]
  assert await engine.prompt(int, "count", on=root) == 3
  await settle()
  asks = [a[5] for a in said(log, "ask")]
  assert [[role for role, *_ in one] for one in asks] == [
    ["user"],
    ["user", "assistant", "user"],
    ["user", "assistant", "user", "assistant", "user"],
  ]
  assert [one[: len(asks[1])] for one in asks[1:]] == [asks[1]] * 2
  assert heads(asks[1][-1:]) == ["#bash1 slow", "#rung3 advance on prompt1"]
  assert heads(asks[2][-1:]) == ["#bash1 exited 0", "#rung5 advance on prompt1"]
  assert said(log, "bash")[0][1] == "bash1"


async def test_an_ask_appends_the_response_as_an_assistant_turn() -> None:
  """An ask appends the response as an assistant turn."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  got = engine.turns(on=root)
  assert got[-2] == said(log, "answer")[0][3]
  assert got[-2][0] == "assistant" and got[-1][0] == "user"


async def test_the_next_ask_tells_everything_that_the_step_told() -> None:
  """The next ask tells everything that the step told."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["raise ValueError('boom')", "close(1)"]
  assert await engine.prompt(int, "try", on=root) == 1
  await settle()
  second = said(log, "ask")[1][5]
  assert heads(second[-1:]) == ["#rung1 raised ValueError('boom')", "#rung3 advance on prompt1"]


async def test_the_turns_hold_every_answer_of_the_model_as_the_turn_it_is() -> None:
  """The turns hold every answer of the model as the turn it is."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(2)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  got = engine.turns(on=root)
  assert [turn for turn in got if turn[0] == "assistant"] == [a[3] for a in said(log, "answer")]


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
