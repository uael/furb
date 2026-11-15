"""jump, a chain that goes on as another chain."""

from asyncio import CancelledError

from conftest import born, paragraphs, said, settle
from furb import engine


async def test_a_jump_keeps_each_fact_that_the_chain_held_so_its_turns_never_change() -> None:
  """A jump: the chain goes on as another chain, and keeps each fact that it held, so the turns of a chain never change."""
  _, _, root = born()
  assert await engine.rung("k = 7", on=root) is None
  before, turns = [x[:3] for x in engine.transcript(root)], engine.turns(on=root)
  await engine.jump(on=root)
  assert await engine.rung("k = 8", on=root) is None
  assert ([x[:3] for x in engine.transcript(root)][: len(before)], engine.turns(on=root)) == (before, turns)
  assert [x for x in engine.transcript(root) if x[0] == "rung" and x[4] == "k = 8"] == []


async def test_jump_makes_at_rest_a_chain_with_the_chain_as_its_source() -> None:
  """jump makes, once no word of the chain runs or awaits and no reply of it is on the way, a chain with the chain as its source that keeps each act of its transcript but those that drop names, runs word there first as a rung of the jump, and is done with that chain."""
  _, _, root = born()
  assert await engine.rung("k = 7", on=root) is None
  engine.read("a.txt", on=root)
  kept = engine.bash("echo hi", on=root)
  await settle()
  read = next(x[1] for x in engine.transcript(root) if x[0] == "read")
  went = engine.jump([read], "w = k + 1", on=root)
  await settle()
  new = str(engine.peek(went))
  assert (engine.get(new)[0], engine.get(new)[5], engine.module(new)["w"]) == ("chain", root, 8)
  held = {x[1] for x in engine.transcript(new)}
  assert (kept in held, read in held) == (True, False)
  assert [x[2] for x in engine.transcript(new) if x[0] == "rung" and x[4] == "w = k + 1"] == [went]


async def test_each_jump_before_the_chain_rests_makes_that_one_chain() -> None:
  """Each jump of a chain that comes before the chain rests makes that one chain, which keeps no act that one of them drops, and runs their words in the order of the jumps."""
  _, _, root = born()
  one, two = engine.bash("echo 1", on=root), engine.bash("echo 2", on=root)
  await settle()
  word = f"a = jump([{one!r}], 'w = [1]')\nb = jump([{two!r}], 'w.append(2)')"
  assert await engine.rung(word, on=root) is None
  await settle()
  a, b = engine.module(root)["a"], engine.module(root)["b"]
  new = str(engine.peek(a))
  assert (engine.peek(b), engine.scope(root), engine.module(new)["w"]) == (new, new, [1, 2])
  assert {one, two} & {x[1] for x in engine.transcript(new)} == set()


async def test_the_chain_that_a_jump_makes_keeps_each_stand_and_each_cd() -> None:
  """The chain that a jump makes keeps each stand and each cd of the chain whatever drop names, so it stands where the chain stood and works in its directory."""
  _, _, root = born()
  assert await engine.rung("cd('sub')", on=root) is None
  went = [x[1] for x in engine.transcript(root) if x[0] in ("stand", "cd")]
  new = await engine.jump(went, on=root)
  assert (engine.cwd(on=new), engine.standing()) == ("/w/sub", engine.standing())
  assert {x[1] for x in engine.transcript(new) if x[0] in ("stand", "cd")} >= set(went)


async def test_after_a_jump_each_act_that_names_the_chain_is_on_the_chain_that_it_jumped_to() -> None:
  """After the jump, each act and each fact that names the chain is on the chain that it jumped to, as each later jump says, so its threads, its messages and the next ask of its model are there."""
  _, _, root = born()
  new = await engine.jump(on=root)
  later = engine.rung("x = 1", on=root)
  assert await later is None and engine.scope(later) == new
  newer = await engine.jump(on=root)
  assert engine.scope(root) == newer == engine.scope(new)


async def test_each_open_thread_of_the_chain_is_asked_again_on_the_chain_that_it_jumped_to() -> None:
  """Each open thread of the chain that a model answers is asked again on the chain that it jumped to, and each rung that waited for its reply on the chain is cancelled."""
  sand, log, root = born("jump()")
  sand.script["chain2"] = ["close(7)"]
  asked = engine.prompt(int, "the k", on=root)
  assert await asked == 7
  rungs = [a for a in said(log, "rung") if a[2] == asked]
  assert [engine.get(a[1])[3] for a in rungs] == [root, "chain2"]
  assert engine.over(rungs[0][1]) and not isinstance(engine.peek(rungs[0][1]), CancelledError)
  sand, log, root = born("jump()")
  sand.script["chain2"] = ["close(1)", "close(2)"]
  one = engine.prompt(int, "one", on=root)
  two = engine.prompt(int, "two", on=root)
  assert (await one, await two) == (1, 2)
  waited = [a[1] for a in said(log, "rung") if a[2] == two and engine.get(a[1])[3] == root]
  assert [type(engine.peek(x)) for x in waited] == [CancelledError]


async def test_the_reading_and_the_transcript_of_a_chain_that_jumped_stay_their_own() -> None:
  """The transcript of a chain that jumped stays its own, so a model reads by its name the turns that its model read and the facts that were on it."""
  _, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  turns, facts = engine.turns(on=root), [x[:3] for x in engine.transcript(root)]
  new = await engine.jump(on=root)
  assert await engine.rung("k = 2", on=new) is None
  assert engine.turns(on=root) == turns and paragraphs(engine.turns(on=new)) != paragraphs(turns)
  assert [x[:3] for x in engine.transcript(root)][: len(facts)] == facts
  assert [x[:3] for x in engine.transcript(new)] != [x[:3] for x in engine.transcript(root)]


async def test_a_word_that_jumps_its_own_chain_ends_first() -> None:
  """A word that jumps its own chain ends first, so no word runs across a jump."""
  _, _, root = born()
  assert await engine.rung("went = jump()\nz = 3", on=root) is None
  await settle()
  new = engine.scope(root)
  assert new != root and engine.module(new)["z"] == 3
