"""Run, the act of running the word of a rung."""

from asyncio import CancelledError

from conftest import (
  Py,
  Sand,
  born,
  chained,
  counted,
  fresh,
  kept,
  plain,
  prompted,
  ran,
  relived,
  said,
  settle,
  sown,
  watched,
  written,
)
from furb import engine


async def test_a_run_is_the_act_of_running_the_word_of_a_rung() -> None:
  """A run is the act of running the word of a rung or the text of a told rung, which the chain makes on itself and the Kernel takes: it says started as the run, runs the word as the rung, and says the run done with what the word gave."""
  log, root, laid, act, step = await counted()
  first = fresh(root, written(laid, "k = 1"))
  second = f"{prompted(act, 'int', 'count')}\n\n#{step} advance on {act}"
  assert [a[2:] for a in said(log, "run")] == [
    (root, root, f"{laid}_told", first, ""),
    (root, root, laid, "k = 1", ""),
    (root, root, f"{step}_told", second, ""),
    (root, root, step, "close(k + 1)", ""),
  ]
  runs = [a[1] for a in said(log, "run")]
  owned = [(a[0], a[1]) for a in log if a[1] in runs and a[0] in ("started", "done") and a[2] == a[1]]
  assert sorted(owned) == sorted((kind, run) for run in runs for kind in ("started", "done"))
  assert [type(engine.peek(run)).__name__ for run in runs] == ["NoneType", "NoneType", "NoneType", "CancelledError"]
  assert ran(log) == [first, "k = 1", second, "close(k + 1)"] and engine.module(root)["k"] == 1


async def test_the_word_a_run_carries_is_python_which_unquoted_made_of_the_word_of_the_rung() -> None:
  """The word a run carries is python, which unquoted made of the word of the rung."""
  sand, log, root = born()
  laid = engine.rung("<s:hi>\nhi\n</s:hi>\nk = hi", on=root)
  await laid
  sand.script[root] = ["<s:there>it's</s:there>\nclose(k + there)"]
  act = engine.prompt(str, "greet", on=root)
  assert await act == "hi\nit's"
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  words = {a[1]: a[3] for a in said(log, "ready")}
  runs = [(a[4], a[5]) for a in said(log, "run") if a[4] in (laid, step)]
  assert runs == [(laid, "hi = 'hi\\n'\n\n\nk = hi"), (step, 'there = "it\'s"\nclose(k + there)')]
  assert [word for _, word in runs] == [engine.unquoted(words[laid]), engine.unquoted(words[step])]
  assert [compile(word, "<run>", "exec").co_names for _, word in runs] == [("hi", "k"), ("there", "close", "k")]


async def test_a_run_names_the_rung_that_the_word_retells() -> None:
  """A run names the rung that the word retells, so a Kernel may answer a retold run from what it kept of that one instead of running the word again."""
  sand = sown()
  log: list[tuple] = []
  root = engine.boot(kernel=kept(Py().kernel()), gate=Py().gating(), probe=watched(log), world=sand.hears())
  await engine.rung("t = read('a.txt')", on=root)
  twin = await chained("twin", root, 300)
  mine = [a for a in said(log, "run") if a[3] == twin]
  theirs = [a for a in said(log, "run") if a[3] == root]
  assert [a[6] for a in theirs] == ["", ""] and [a[6] for a in mine] == ["", theirs[0][4], theirs[1][4]]
  assert [a[0] for a in sand.calls].count("read") == 1
  assert engine.module(twin)["t"] == engine.module(root)["t"]


async def test_every_rung_of_a_chain_runs_in_the_globals_of_the_chain() -> None:
  """Every rung of a chain runs in the globals of the chain, the word of a model and a word its caller wrote alike."""
  sand, _, root = born()
  await engine.rung("mine = 1", on=root)
  sand.script[root] = ["theirs = mine + 1\nclose(theirs)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  assert (engine.module(root)["mine"], engine.module(root)["theirs"]) == (1, 2)


async def test_what_a_rung_binds_stays_bound_for_every_later_rung_of_the_chain() -> None:
  """What a rung binds stays bound for every later rung of the chain."""
  _, _, root = born("a = 1", "b = a + 1", "close(b + 1)", "close(None)")
  assert await engine.prompt(int, "count", on=root) == 3
  assert (engine.module(root)["a"], engine.module(root)["b"]) == (1, 2)


async def test_the_last_rung_to_bind_a_name_wins() -> None:
  """The last rung to bind a name wins."""
  sand, _, root = born()
  await engine.rung("def twice(x):\n  return x * 2", on=root)
  await engine.rung("def twice(x):\n  return x * 3", on=root)
  sand.script[root] = ["close(twice(21))", "close(None)"]
  assert await engine.prompt(int, "use it", on=root) == 63


async def test_one_rung_runs_at_a_time_on_a_chain_and_rungs_interleave_at_their_awaits() -> None:
  """One rung runs at a time on a chain, and rungs interleave at their awaits, whether they are rungs of one chain or of many."""
  sand, log, root = born(auto=False)
  two = engine.chain("two")
  sand.script[root] = ["x = bash('slow here')\nclose((await x).code)"]
  sand.script[two] = ["y = bash('slow there')\nclose((await y).code)"]
  here = engine.prompt(int, "go", on=root)
  there = engine.prompt(int, "go", on=two)
  await settle()
  assert [engine.scope(a[1]) for a in said(log, "wants")] == [root, two]
  assert engine.peek(here, ...) is ... and engine.peek(there, ...) is ...
  quick = engine.rung("k = 1", on=root)
  await settle()
  for a in said(log, "bash"):
    sand.exits(a[1], 0)
  await settle()
  assert ((await here), (await there)) == (0, 0)
  (step,) = [a[1] for a in said(log, "rung") if a[2] == here]
  runs = {a[4]: a[1] for a in said(log, "run")}
  slow, fast = runs[step], runs[quick]
  (waits,) = [a[1] for a in said(log, "wants") if a[2] == slow]
  moves = [(a[0], a[1]) for a in log if a[1] in (slow, fast, waits) and a[0] in ("wants", "started", "done")]
  assert moves == [
    ("started", slow),
    ("wants", waits),
    ("started", waits),
    ("started", fast),
    ("done", fast),
    ("done", waits),
    ("done", slow),
  ]


async def test_the_word_of_a_rung_runs_again_in_every_chain_made_from_its_chain() -> None:
  """The word of a rung runs again in every chain made from its chain and in every later life, and what it does outside its acts it does again."""
  sand, _, root = born("marks = []\nmarks.append(1)\nclose(len(marks))", "close(None)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  twin = await chained("twin", root, 300)
  assert engine.module(twin)["marks"] == [1] and engine.module(twin)["marks"] is not engine.module(root)["marks"]
  _, over = await relived(Sand(), plain(sand.record))
  assert engine.module(over)["marks"] == [1]


async def test_two_prompts_on_one_chain_see_the_bindings_of_each_other_as_they_run() -> None:
  """Two prompts on one chain see the bindings of each other as they run."""
  sand, log, root = born(
    "mine, theirs = 1, 0\nx = bash('slow')\nawait x\nclose(mine + theirs)", "theirs = mine + 10\nclose(theirs)"
  )
  sand.auto = False
  first = engine.prompt(int, "bind", on=root)
  second = engine.prompt(int, "read it", on=root)
  await settle()
  assert engine.peek(first, ...) is ... and engine.peek(second) == 11
  sand.exits(said(log, "bash")[0][1], 0)
  await settle()
  assert ((await first), (await second)) == (12, 11)


async def test_a_rung_whose_word_rebinds_a_broken_name_repairs_the_chain() -> None:
  """A rung whose word rebinds a broken name repairs the chain, since the last rung to bind wins."""
  sand, _, root = born("def twice(x):\n  raise ValueError('broken')", "close(None)")
  assert await engine.prompt(None, "bind it", on=root) is None
  await engine.rung("def twice(x):\n  return x * 2", on=root)
  sand.script[root] = ["close(twice(21))", "close(None)"]
  assert await engine.prompt(int, "use it", on=root) == 42


async def test_a_prompt_after_such_a_rung_finds_what_it_bound() -> None:
  """A prompt after such a rung finds what it bound."""
  sand, _, root = born()
  await engine.rung("def twice(x):\n  return x * 2", on=root)
  sand.script[root] = ["close(twice(21))", "close(None)"]
  assert await engine.prompt(int, "use it", on=root) == 42


async def test_the_chain_runs_one_word_at_a_time() -> None:
  """The chain runs one word at a time."""
  sand, log, root = born(auto=False)
  first = engine.rung("x = bash('echo hi')\nclose(await x)", on=root)
  second = engine.rung("k = 2\nclose(k)", on=root)
  await settle()
  one, two = "x = bash('echo hi')\nclose(await x)", "k = 2\nclose(k)"
  opens = f"#bash1\nbash1_command = 'echo hi'\n{engine.bound('bash1', 'Exit')}"
  told = [engine.unquoted(fresh(root, written(first, one))), engine.unquoted(f"{opens}\n\n{written(second, two)}")]
  assert ran(log) == [told[0], one, told[1], two]
  (_, run, _, _) = [a[1] for a in said(log, "run")]
  spoken = [(a[0], a[2]) for a in log if a[0] in ("bash", "wants", "close")]
  assert spoken == [("bash", first), ("wants", run), ("close", second)]
  sand.exits(said(log, "bash")[0][1], 0)
  await settle()
  assert engine.peek(first, ...) is not ... and await second == 2
  assert [(a[0], a[2]) for a in log if a[0] in ("bash", "wants", "close")] == [*spoken, ("close", first)]


async def test_a_word_that_waits_for_an_act_gives_the_chain_to_the_next_word() -> None:
  """A word that waits for an act gives the chain to the next word, which runs while it waits, and the waiting word runs on at the done of what it awaits."""
  sand, log, root = born(auto=False)
  waiting = engine.rung("x = bash('slow')\nout = await x\nk = out.code", on=root)
  after = engine.rung("k = 2", on=root)
  await settle()
  assert engine.peek(after) is None and engine.peek(waiting, ...) is ... and engine.module(root)["k"] == 2
  sand.exits("bash1", 0)
  await settle()
  assert engine.peek(waiting) is None and engine.module(root)["k"] == 0
  (slow,) = [a[1] for a in said(log, "run") if a[4] == waiting]
  (quick,) = [a[1] for a in said(log, "run") if a[4] == after]
  (wants,) = said(log, "wants")
  moves = [(a[0], a[1]) for a in log if a is wants or (a[0] == "done" and a[1] in (slow, quick, "bash1", wants[1]))]
  assert wants[2] == slow and moves == [
    ("wants", wants[1]),
    ("done", quick),
    ("done", "bash1"),
    ("done", wants[1]),
    ("done", slow),
  ]


async def test_a_run_whose_rung_is_done_when_the_kernel_begins_it_never_begins_its_word() -> None:
  """A run whose rung is done when the Kernel begins it never begins its word, and is done with CancelledError."""
  _, _, root = born()
  assert await engine.rung("r = rung('ran = 1')\ncancel(r)", on=root) is None
  await settle()
  inner = engine.module(root)["r"]
  (run,) = [a[1] for a in engine.transcript(root) if a[0] == "run" and a[4] == inner]
  assert "ran" not in engine.module(root)
  assert isinstance(engine.peek(inner), CancelledError) and isinstance(engine.peek(run), CancelledError)
