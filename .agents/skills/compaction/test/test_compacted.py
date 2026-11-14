"""compacted, a chain that compacts itself and goes on under its name."""

from collections.abc import Callable
from pathlib import Path

from conftest import until
from furb import engine
from furb.engine import Act, Text
from skills.conftest import FULL, compacting

GIST = "close(([], 'gist'))"
"""GIST is the word of a model that chooses no act and sums the rest up as gist."""
ONCE = "SHARE = 2\ny = 2\ndebug(t'{y}')"
"""ONCE is a word that binds y and rebinds SHARE past any share, so the chain compacts once."""


async def wrapped(
  tmp_path: Path, *words: str, chooser: str = GIST, ends: Callable[[str], bool] | None = None
) -> tuple[str, Act[int]]:
  """A life whose root compacted wraps, after the words given, whose model answers x = 1, then ONCE, then close(x + y),
  and whose model of the choice answers with the chooser: the root and the prompt go, which it awaited."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  for word in words:
    await engine.rung(word, on=root)
  sand.cost = FULL
  sand.script[root] = ["x = 1\ndebug(t'{x}')", "SHARE = 2\n" + chooser, ONCE, "close(x + y)"]
  act = engine.prompt(int, "go", on=root)
  await act
  await until(lambda: (ends or compacted_on)(root))
  return root, act


def made(root: str, by: str = "compacted1") -> list[str]:
  """The words of the rungs that an act made on a chain."""
  return [str(a[4]) for a in engine.transcript(root) if a[0] == "rung" and a[2] == by]


def compacted_on(root: str, by: str = "compacted1") -> bool:
  """Whether a rung that an act made on a chain ran to its end: the fact that ends a step of compaction."""
  return any(engine.over(str(a[1])) for a in engine.transcript(root) if a[0] == "rung" and a[2] == by)


def told_kept(root: str) -> bool:
  """Whether the chain was told that it was kept as it is: the fact that ends a refused choice."""
  return any(a[0] == "tell" and "kept the chain as it is" in str(a[3]) for a in engine.transcript(root))


def opened(chain: str, message: str) -> bool:
  """Whether a prompt of a message is open on a chain."""
  return any(a[0] == "prompt" and a[5] == message and not engine.over(str(a[1])) for a in engine.transcript(chain))


def asked(chain: str, message: str) -> str:
  """The prompt of a message on a chain."""
  return next(str(a[1]) for a in engine.transcript(chain) if a[0] == "prompt" and a[5] == message)


def choices(root: str) -> list[tuple]:
  """The prompts of choice on a chain."""
  return [a for a in engine.transcript(root) if a[0] == "prompt" and a[4] == "tuple"]


async def test_compacted_wraps_a_chain(tmp_path: Path) -> None:
  """compacted() wraps a chain: after an answer that reads more than SHARE of the window of its actor, it asks choice() of the chain, and at the next reply on the chain once that came, it compacts the chain, which it wraps still."""
  root, act = await wrapped(tmp_path)
  assert engine.peek(act) == 3
  assert engine.get("chain2") is None and len(choices(root)) == 1
  assert made(root) == ["<s:summary>\ngist</s:summary>\nreground()"]


async def test_the_chain_keeps_the_acts_that_the_model_chose(tmp_path: Path) -> None:
  """The chain keeps the acts that the model chose, each act said on the chain after it asked, and of each path only the last read or write, and the summary of the model."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  await engine.rung("write(Text('c.txt', 'c'))\nwrite(Text('d.txt', 'd'))\nread('a.txt')", on=root)
  held = {
    str(a[4].path if isinstance(a[4], Text) else a[4]): str(a[1])
    for a in engine.transcript(root)
    if a[0] in ("read", "write")
  }
  sand.cost = FULL
  chooser = f"close(([{held['c.txt']!r}, {held['a.txt']!r}], 'gist'))"
  sand.script[root] = ["read('a.txt')\nx = 1", "SHARE = 2\n" + chooser, ONCE, "close(x + y)"]
  assert await engine.prompt(int, "go", on=root) == 3
  await until(lambda: compacted_on(root))
  told = {a[1] for a in engine.reading(root) if a[0] == "tell"}
  again = [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt" and a[1] != held["a.txt"]]
  assert held["c.txt"] in told and held["d.txt"] not in told and held["a.txt"] not in told
  assert again[0] in told and engine.module(root)["summary"] == "gist"


async def test_when_the_model_gives_no_tuple_it_compacts_the_chain_with_filters(tmp_path: Path) -> None:
  """When the model gives no tuple of a list and a str, it compacts the chain with FILTERS and no summary of the model."""
  root, act = await wrapped(tmp_path, "FILTERS = [lambda acts: []]\nread('a.txt')", chooser="close((1, 2))")
  assert engine.peek(act) == 3 and made(root) == ["reground()"]
  (one,) = [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  assert [a for a in engine.reading(root) if a[0] == "tell" and a[1] == one] == []


async def test_it_moves_stops_and_pauses_nothing(tmp_path: Path) -> None:
  """It moves, stops and pauses nothing, so each open prompt, each rung that waits and each word that runs goes on in the chain."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  sand.cost = FULL
  word = "w = prompt(int, 'work', on=chain('worker'))\nn = await prompt(int, 'n', to=OPERATOR)\nclose(n + await w)"
  worker = "v = await prompt(int, 'w?', to=OPERATOR)\nclose(v)"
  sand.script.update({root: [word, GIST, "close(None)"], "chain2": [worker]})
  act = engine.prompt(int, "go", on=root)
  await until(lambda: opened(root, "n") and opened("chain2", "w?"))
  engine.prompt(None, "poke", on=root)
  await until(lambda: compacted_on(root))
  assert made(root) == ["<s:summary>\ngist</s:summary>\nreground()"]
  ran = next(str(a[4]) for a in engine.transcript(root) if a[0] == "run" and a[5] == word)
  assert not engine.over(ran) and not engine.over(act)
  assert [a for one in (root, "chain2") for a in engine.transcript(one) if a[0] == "pause"] == []
  engine.close(5, asked("chain2", "w?"))
  engine.close(2, asked(root, "n"))
  assert await act == 7


async def test_it_never_completes(tmp_path: Path) -> None:
  """It never completes."""
  sand, root, _ = compacting(tmp_path)
  sand.cost = FULL
  sand.script[root] = ["w = compacted()\ndebug(t'{w}')", "x = 1\ndebug(t'{x}')", "SHARE = 2\n" + GIST, "close(1)"]
  assert await engine.prompt(int, "go", on=root) == 1
  watcher = engine.module(root)["w"]
  await until(lambda: compacted_on(root, watcher))
  assert made(root, watcher) == ["<s:summary>\ngist</s:summary>\nreground()"]
  assert engine.peek(watcher, "open") == "open"


async def test_when_the_choice_ends_refused_it_keeps_the_chain_as_it_is(tmp_path: Path) -> None:
  """When the choice ends refused, it keeps the chain as it is, tells why, and asks no choice again."""
  root, act = await wrapped(tmp_path, chooser="close(Refused('no choice'))", ends=told_kept)
  assert engine.peek(act) == 3
  assert made(root) == []
  assert len(choices(root)) == 1
