"""Filter, what says which acts of a transcript the turns of a chain with a source keep."""

from conftest import STANDS, Sand, life, lived, named, paragraphs, plain, relived, said, seen, settle, sown
from furb import engine


async def test_a_filter_is_given_the_acts_of_the_transcript_up_to_the_source() -> None:
  """A filter is given the acts of the transcript up to the source of a chain that has one."""
  sand = sown()
  log, root = await lived(sand)
  held: list[tuple] = []
  engine.chain("twin", source=root, filter=seen(held))
  await settle()
  theirs = engine.transcript(root)
  facts = [one[1] for one in theirs if engine.question(one)]
  assert [one[1] for one in held] == facts[: len(held)]
  made = {said(log, "prompt")[0][1], said(log, "rung")[0][1], said(log, "bash")[0][1]}
  assert made <= {one[1] for one in held}


async def test_a_filter_says_which_acts_the_turns_of_that_chain_keep_each_with_its_entries() -> None:
  """A filter says which acts the turns of that chain keep, each with its entries."""
  sand = sown()
  log, root = await lived(sand)
  (one, ack), command = [a[1] for a in said(log, "prompt")], said(log, "bash")[0][1]
  step, answered = [a[1] for a in said(log, "rung") if a[2] in (one, ack)]
  narrow = engine.chain("narrow", source=root, filter=engine.take(command, inside=False))
  await settle()
  assert named(engine.turns(on=root)) == [root, root, one, step, "read", command, command, one, ack, answered, ack]
  assert named(engine.turns(on=narrow)) == [root, root, one, step, "read", one, ack, answered, ack, narrow]
  kept = [part for part in paragraphs(engine.turns(on=narrow)) if part.startswith(f"#{step} ")]
  assert kept == [f"#{step} advance on {one}"]


async def test_a_filter_is_any_callable_of_that_shape() -> None:
  """A filter is any callable of that shape, so a word adds a filter by writing one, and take makes the filter of the file."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = [
    "x = bash('echo one')\nn = (await x).code",
    "def bare(facts):\n  return [a for a in facts if not under(a[1], x)]\n\nclose(chain('side', source=__name__, filter=bare))",
  ]
  side = await engine.prompt(str, "fork", on=root)
  await settle(200)
  command = said(log, "bash")[0][1]
  assert command in named(engine.turns(on=root)) and command not in named(engine.turns(on=side))
  assert engine.module(side)["n"] == 0
  assert engine.take(command, inside=False)([engine.get(command)]) == []


async def test_a_later_life_runs_the_filter_again_and_keeps_the_same_acts() -> None:
  """A later life runs the filter again and keeps the same acts, since the word that opened the chain opens it again with it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = [
    "x = bash('echo hi')\nn = (await x).code",
    "close(chain('side', source=__name__, filter=take(x, inside=False)))",
  ]
  side = await engine.prompt(str, "fork", on=root)
  await settle(200)
  command = said(log, "bash")[0][1]
  was = [(role, py) for role, py, *_ in engine.turns(on=side)]
  assert command in named(engine.turns(on=root)) and command not in named(engine.turns(on=side))
  _, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert over == root and [(role, py) for role, py, *_ in engine.turns(on=side)] == was


async def test_a_filter_is_no_word_of_a_chain() -> None:
  """A filter is no word of a chain: the verb keeps it for what the chain holds, so no record holds one."""
  sand = sown()
  log, root = await lived(sand)
  held: list[tuple] = []
  side = engine.chain("side", source=root, filter=seen(held))
  await settle()
  kept = [fact for fact, *_ in sand.record if fact[0] == "chain"]
  assert [(one[1], one[4], one[5]) for one in kept] == [(root, "root", ""), (side, "side", root)]
  assert [len(one) for one in said(log, "chain")] == [6, 6] and held
