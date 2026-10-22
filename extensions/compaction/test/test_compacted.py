"""compacted, a chain that compacts itself."""

from pathlib import Path

from conftest import Sand, settle
from extensions.conftest import FULL, compacting
from furb import engine
from furb.engine import OPERATOR, Act, Text

GIST = "close(([], 'gist'))"
"""GIST is the word of a model that chooses no act and sums the rest up as gist."""


async def wrapped(
  tmp_path: Path, *words: str, chooser: str = GIST, after: str = "close(x + y)"
) -> tuple[str, Act[int]]:
  """A life whose root compacted wraps, whose model answers each answer past SHARE with the words given, then x = 1
  and y = 2 when it is given none, whose model of the choice answers with the chooser, and whose model of the new
  chain answers with after: the root and the prompt go, whose answer it awaits."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  for word in words:
    await engine.rung(word, on=root)
  sand.cost = FULL
  sand.script.update({root: ["x = 1", "y = 2"], "chain2": [chooser], "chain3": [after]})
  act = engine.prompt(int, "go", on=root)
  await act
  await settle()
  return root, act


def first(chain: str) -> str:
  """The first word of a chain that compacted made, which compacted said."""
  return next(str(a[4]) for a in engine.transcript(chain) if a[0] == "rung" and a[2] == "compacted1")


async def stopped(tmp_path: Path) -> tuple[Sand, str, Act[int], str]:
  """A life whose root compacted wraps, with a prompt go whose rung hired a worker that waits on the operator and waits
  on the operator too, which compacted compacts at the reply of a poke: the World, the root, go and the word of its
  rung."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  sand.cost = FULL
  word = "w = prompt(int, 'work', on=chain('worker'))\nn = await prompt(int, 'n', to=OPERATOR)\nclose(n)"
  worker = "v = await prompt(int, 'w?', to=OPERATOR)\nclose(v)"
  sand.script.update({root: [word], "chain2": [GIST], "chain3": [worker]})
  act = engine.prompt(int, "go", on=root)
  await settle()
  engine.prompt(None, "poke", on=root)
  await settle()
  return sand, root, act, word


def asked(chain: str, message: str) -> str:
  """The prompt of a message on a chain."""
  return next(str(a[1]) for a in engine.transcript(chain) if a[0] == "prompt" and a[5] == message)


async def test_compacted_wraps_a_chain(tmp_path: Path) -> None:
  """compacted() wraps a chain: after an answer that reads more than SHARE of the window of its actor, it asks choice() of the chain, and at the next reply on the chain once that came, it compacts the chain and wraps the new chain in turn."""
  root, act = await wrapped(tmp_path)
  assert engine.peek(act) == 3
  assert [engine.get(one)[3:] for one in ("chain2", "chain3", "chain4")] == [
    (root, "chosen", ""),
    (root, "compacted", root),
    ("chain3", "chosen", ""),
  ]
  assert first("chain3") == "restore('chain1')\n<s:summary>\ngist</s:summary>\nreground()"


async def test_the_new_chain_keeps_the_acts_that_the_model_chose(tmp_path: Path) -> None:
  """The new chain keeps the acts that the model chose but each rung that is not over, each act said on the chain after it asked, and of each path only the last read or write, and the summary of the model."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("compacted()", on=root)
  await engine.rung("write(Text('c.txt', 'c'))\nwrite(Text('d.txt', 'd'))\nread('a.txt')", on=root)
  held = {
    str(a[4].path if isinstance(a[4], Text) else a[4]): str(a[1])
    for a in engine.transcript(root)
    if a[0] in ("read", "write")
  }
  waits = engine.rung("n = await prompt(int, 'n', to=OPERATOR)", on=root)
  await settle()
  sand.cost = FULL
  chooser = f"close(([{held['c.txt']!r}, {held['a.txt']!r}, {waits!r}], 'gist'))"
  sand.script.update({root: ["read('a.txt')\nx = 1", "y = 2"], "chain2": [chooser], "chain3": ["close(x + y)"]})
  assert await engine.prompt(int, "go", on=root) == 3
  again = [str(a[1]) for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"][-1]
  kept = {str(a[1]) for a in engine.transcript("chain3") if a[0] in ("read", "write") and engine.scope(a[1]) == root}
  assert kept == {held["c.txt"], again}
  assert [a for a in engine.transcript("chain3") if a[1] == waits] == []


async def test_when_the_model_gives_no_tuple_it_compacts_the_chain_with_filters(tmp_path: Path) -> None:
  """When the model gives no tuple of a list and a str, it compacts the chain with FILTERS and no summary of the model."""
  root, act = await wrapped(tmp_path, "FILTERS = [lambda acts: []]\nread('a.txt')", chooser="close((1, 2))")
  assert engine.peek(act) == 3
  assert first("chain3") == "restore('chain1')\nreground()"
  assert [a for a in engine.transcript("chain3") if a[0] == "read" and engine.scope(a[1]) == root] == []


async def test_it_moves_each_open_prompt_of_a_model_to_the_new_chain(tmp_path: Path) -> None:
  """It moves each open prompt of a model to the new chain, and closes the old prompt with what the new one comes to."""
  _, act = await wrapped(tmp_path, after="close(x + y + 4)")
  moved = [a[1:] for a in engine.transcript("chain3") if a[0] == "prompt" and a[3] == "chain3"]
  assert [one[3:] for one in moved] == [("int", "go", "")]
  assert engine.peek(act) == engine.peek(moved[0][0]) == 7


async def test_it_stops_each_open_rung_of_a_prompt_that_it_moves_with_a_close(tmp_path: Path) -> None:
  """It stops each open rung of a prompt that it moves with a close, which reaches that rung alone, and it pauses nothing, so what such a rung started goes on, as the workers that a conductor hired do."""
  _, root, _, word = await stopped(tmp_path)
  ran = next(str(a[4]) for a in engine.transcript(root) if a[0] == "run" and a[5] == word)
  assert engine.over(ran) and not engine.over(asked(root, "n"))
  assert [a for a in engine.transcript("chain4") if a[0] == "rung" and a[5] == ran] == []
  engine.close(7, asked("chain3", "w?"))
  await settle()
  assert engine.peek(asked("chain3", "work")) == 7
  assert [a for one in (root, "chain3") for a in engine.transcript(one) if a[0] == "pause"] == []


async def test_a_rung_of_its_own_keeps_each_prompt_that_it_moves_from_asking_again(tmp_path: Path) -> None:
  """A rung of its own on each prompt that it moves keeps that prompt from asking its model again."""
  root, act = await wrapped(tmp_path)
  held = [(a[2], a[4]) for a in engine.transcript(root) if a[0] == "rung" and a[3] == act]
  assert held == [("compacted1", f"await Act({act!r})")]
  assert len([a for a in engine.transcript(root) if a[0] == "reply"]) == 3


async def test_the_summary_names_each_rung_that_it_stopped(tmp_path: Path) -> None:
  """The summary names each rung that it stopped, the act that it awaited and its word, and says to run no line before that await again, so a word that loops runs again in the new chain only when a word runs it again, on what restore bound."""
  _, root, act, word = await stopped(tmp_path)
  ran = next(str(a[4]) for a in engine.transcript(root) if a[0] == "run" and a[5] == word)
  summary = (
    f"gist\n\nThe compaction stopped {ran}, a step of {act}, as it awaited {asked(root, 'n')}. Each line of its word"
    " before that await ran, and the module holds what it bound, so run none of them again: go on from that await, as"
    f" by running again the loop that holds it. Its word was:\n{word}"
  )
  assert first("chain4") == f"restore('chain1')\n<s:summary>\n{summary}</s:summary>\nreground()"
  assert engine.module("chain4")["w"] == engine.module(root)["w"] == asked("chain3", "work")


async def test_a_prompt_that_asks_on_a_chain_that_it_compacted_moves_to_the_newest_chain(tmp_path: Path) -> None:
  """A prompt of a model that asks its model on a chain that it compacted moves to the newest chain at that reply."""
  root, _ = await wrapped(tmp_path)
  late = engine.prompt(int, "late", on=root)
  await settle()
  moved = [a[1] for a in engine.transcript("chain3") if a[0] == "prompt" and a[5] == "late"]
  assert len(moved) == 1 and engine.peek(late) is None
  assert [a[4] for a in engine.transcript(root) if a[0] == "rung" and a[3] == late] == [f"await Act({late!r})"]


async def test_it_moves_no_prompt_to_the_operator_and_no_prompt_that_made_it(tmp_path: Path) -> None:
  """It moves no prompt to the operator and no prompt that made it, and it never completes."""
  sand, root, _ = compacting(tmp_path)
  sand.cost = FULL
  sand.script.update({root: ["w = compacted()", "x = 1", "y = 2", "close(1)"], "chain2": [GIST]})
  operator = engine.prompt(int, "ask me", to=OPERATOR, on=root)
  assert await engine.prompt(int, "go", on=root) == 1
  await settle()
  watcher = engine.module(root)["w"]
  assert engine.get("chain2")[4] == "chosen" and engine.get("chain3") is None
  assert [engine.peek(watcher, "open"), engine.peek(operator, "open")] == ["open", "open"]
