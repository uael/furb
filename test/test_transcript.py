"""transcript, the facts on a chain."""

from conftest import born, heads, lasting, paragraphs, said, settle
from furb import engine


async def test_transcript_gives_the_facts_on_a_chain() -> None:
  """transcript gives the facts on a chain, each of which the life adds when it is said, so a chain reads at once what it said itself."""
  sand, log, root = born("close(1)")
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  (asked,) = said(log, "reply")
  assert heads(sand.turns[asked[1]])[-1] == f"#{asked[2]} advance on {one}"
  held, log = lasting(engine.transcript(root)), lasting(log)
  assert [a for a in held if a not in log] == []
  assert [a for a in log if a in held and not engine.question(a)] == [a for a in held if not engine.question(a)]


async def test_transcript_gives_a_new_list_at_each_call() -> None:
  """transcript gives a new list at each call, so a word that changes the list it was given changes no transcript."""
  _, _, root = born()
  before = engine.transcript(root)
  word = "n = 0\nfor x in transcript():\n  n += 1\n  debug(t'{n}')\ntranscript().clear()\nclose(n)"
  counted = await engine.rung(word, on=root)
  held = engine.transcript(root)
  assert isinstance(counted, int) and lasting(held[: len(before)]) == lasting(before)
  assert len(before) < counted < len(held)
  assert engine.transcript(root) == held and engine.transcript(root) is not held


async def test_the_transcript_is_the_whole_state_of_a_chain() -> None:
  """The transcript is the whole state of a chain: its module, its program, its working directory and its turns are read off it, and the standing is read off the transcript of the root."""
  _, _, root = born()
  await engine.rung("k = 1\ncd('/x')", on=root)
  held = engine.transcript(root)
  assert engine.module(root) == [a[3] for a in held if a[0] == "module"][-1]
  assert engine.program(root) == {a[4]: a[5] for a in held if a[0] == "run"} == {"rung1": "k = 1\ncd('/x')"}
  assert engine.cwd(on=root) == [a[4] for a in held if a[0] == "cd"][-1] == "/x"
  assert paragraphs(engine.turns(on=root)) == ["\n".join(a[3]) for a in said(held, "tell")]
  assert engine.standing() == next(a[3] for a in engine.transcript(engine.ROOT) if a[:2] == ("done", "stand1"))


async def test_a_name_of_no_chain_gives_no_fact() -> None:
  """A name of no chain gives no fact."""
  _, _, root = born()
  one = engine.bash("echo hi", on=root)
  assert engine.transcript("chain9") == [] and engine.transcript(one) == []
