"""Run, what tells the Kernel to run the word of a rung."""

from itertools import pairwise

from conftest import STANDS, Py, Sand, life, plain, relived, said, settle, sown
from furb import engine
from furb.engine import WORLD


async def test_a_run_tells_the_kernel_to_run_the_word_of_a_rung() -> None:
  """A run tells the Kernel to run the word of a rung, and says the chain whose module it runs in."""
  sand, py = Sand(stands=STANDS), Py()
  log, root = life(sand, kernel=py)
  laid = engine.rung("k = 1", on=root)
  await laid
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  _, asking, *_ = said(log, "rung")[-1]
  assert said(log, "run") == [("run", laid, root, root, "k = 1"), ("run", asking, root, root, "close(k + 1)")]
  assert py.ran == ["k = 1", "close(k + 1)"]


async def test_every_rung_of_a_chain_runs_in_the_globals_of_the_chain() -> None:
  """Every rung of a chain runs in the globals of the chain, the word of a model and a word its caller wrote alike."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("mine = 1", on=root)
  sand.script[root] = ["theirs = mine + 1\nclose(theirs)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  assert (engine.modules[root]["mine"], engine.modules[root]["theirs"]) == (1, 2)


async def test_what_a_rung_binds_stays_bound_for_every_later_rung_of_the_chain() -> None:
  """What a rung binds stays bound for every later rung of the chain."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["a = 1", "b = a + 1", "close(b + 1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 3
  assert (engine.modules[root]["a"], engine.modules[root]["b"]) == (1, 2)


async def test_the_last_rung_to_bind_a_name_wins() -> None:
  """The last rung to bind a name wins."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("def twice(x):\n  return x * 2", on=root)
  await engine.rung("def twice(x):\n  return x * 3", on=root)
  sand.script[root] = ["close(twice(21))", "close(None)"]
  assert await engine.prompt(int, "use it", on=root) == 63


async def test_one_rung_runs_at_a_time_on_a_chain_and_rungs_interleave_at_their_awaits() -> None:
  """One rung runs at a time on a chain, and rungs interleave at their awaits, whether they are rungs of one chain or of many."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS, auto=False)
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = ["x = bash('slow here')\nclose((await x).code)"]
  sand.script[two] = ["y = bash('slow there')\nclose((await y).code)"]
  here = engine.prompt(int, "go", on=root)
  there = engine.prompt(int, "go", on=two)
  await settle()
  assert [engine.scope(a[1]) for a in said(log, "wants")] == [root, two]
  assert here not in engine.outcomes and there not in engine.outcomes
  for a in said(log, "bash"):
    engine.send("exited", a[1], 0, by=WORLD)
  await settle()
  assert ((await here), (await there)) == (0, 0)


async def test_the_word_of_a_rung_runs_again_in_every_chain_made_from_its_chain() -> None:
  """The word of a rung runs again in every chain made from its chain and in every later life, and what it does outside its acts it does again."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["marks = []\nmarks.append(1)\nclose(len(marks))", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert engine.modules[twin]["marks"] == [1] and engine.modules[twin]["marks"] is not engine.modules[root]["marks"]
  _, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert engine.modules[over]["marks"] == [1]


async def test_two_prompts_on_one_chain_see_the_bindings_of_each_other_as_they_run() -> None:
  """Two prompts on one chain see the bindings of each other as they run."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["mine = 1\nclose(mine)", "close(mine + 1)", "close(None)"]
  first = engine.prompt(int, "bind", on=root)
  second = engine.prompt(int, "read it", on=root)
  await settle()
  assert ((await first), (await second)) == (1, 2)


async def test_a_rung_whose_word_rebinds_a_broken_name_repairs_the_chain() -> None:
  """A rung whose word rebinds a broken name repairs the chain, since the last rung to bind wins."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["def twice(x):\n  raise ValueError('broken')", "close(None)"]
  assert await engine.prompt(None, "bind it", on=root) is None
  await engine.rung("def twice(x):\n  return x * 2", on=root)
  sand.script[root] = ["close(twice(21))", "close(None)"]
  assert await engine.prompt(int, "use it", on=root) == 42


async def test_a_prompt_after_such_a_rung_finds_what_it_bound() -> None:
  """A prompt after such a rung finds what it bound."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("def twice(x):\n  return x * 2", on=root)
  sand.script[root] = ["close(twice(21))", "close(None)"]
  assert await engine.prompt(int, "use it", on=root) == 42


async def test_the_chain_runs_one_word_at_a_time() -> None:
  """The chain runs one word at a time."""
  sand, py = Sand(stands=STANDS, auto=False), Py()
  log, root = life(sand, kernel=py)
  first = engine.rung("x = bash('echo hi')\nclose(await x)", on=root)
  second = engine.rung("k = 2\nclose(k)", on=root)
  await settle()
  runs = [i for i, a in enumerate(log) if a[0] == "run"]
  assert len(runs) == 2 and py.ran == ["x = bash('echo hi')\nclose(await x)", "k = 2\nclose(k)"]
  for one, two in pairwise(runs):
    assert [a for a in log[one:two] if a[0] in ("wants", "ran") and a[1] == log[one][1]]
  engine.send("exited", said(log, "bash")[0][1], 0, by=WORLD)
  await settle()
  assert first in engine.outcomes and await second == 2


async def test_a_word_that_waits_for_an_act_gives_the_chain_to_the_next_word() -> None:
  """A word that waits for an act gives the chain to the next word, which runs while it waits, and the waiting word runs on at the done of what it awaits."""
  sand = sown()
  log, root = life(sand)
  sand.script["chain://operator.2.1"] = ["close(21)", "close(None)", "close(None)"]
  sand.script[root] = ["close(None)", "close(None)"]
  got = engine.rung("f = chain('x', source=__name__)\nn = await prompt(int, 'add', on=f)\nclose(n)", on=root)
  assert await got == 21
  await settle()
  fork = said(log, "chain")[1]
  assert (fork[1], fork[5]) == ("chain://operator.2.1", root)
