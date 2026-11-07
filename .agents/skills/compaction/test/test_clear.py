"""clear, the chain cut to nothing that goes on under its name."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import compacting


async def test_clear_supersedes_the_chain_of_the_caller_with_a_filter_that_keeps_nothing(tmp_path: Path) -> None:
  """clear() supersedes the chain of the caller with a filter that keeps nothing, with no summary and no regrounding, and gives that chain."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["read('a.txt')", "close(clear())"]
  assert await engine.prompt(str, "clear", on=root) == root
  await settle()
  (first,) = [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  assert engine.get("chain2") is None
  assert [a for a in engine.transcript(root) if a[0] == "tell" and a[1] == first] == []
  assert [a for a in engine.transcript(root) if a[0] == "rung" and str(a[4]).endswith("reground()")] == []
  assert "summary" not in engine.module(root)


async def test_so_the_model_of_the_chain_reads_from_zero_and_the_chain_keeps_what_it_is(tmp_path: Path) -> None:
  """So the model of the chain reads from zero, and the chain keeps its name, its module, its open prompts and its threads."""
  sand, root, _ = compacting(tmp_path)
  word = "k = 7\nread('a.txt')\nw = prompt(int, 'n', to=OPERATOR)\nside = chain('worker')"
  sand.script[root] = [word, "clear()\nclose(k)"]
  the = engine.prompt(int, "the k", on=root)
  await settle()
  engine.prompt(None, "go on", on=the)
  assert await the == 7
  await settle()
  asked = next(str(a[1]) for a in engine.transcript(root) if a[0] == "prompt" and a[5] == "n")
  (turn,) = engine.turns(on=root)
  assert turn[0] == "user" and turn[1].startswith("#chain1\n") and "a.txt" not in turn[1] and "k = 7" not in turn[1]
  assert engine.module(root)["k"] == 7 and engine.module(root)["side"] == "chain2"
  assert not engine.over(asked) and engine.get("chain2")[3:] == (root, "worker", "")


async def test_the_record_keeps_each_act_of_the_chain(tmp_path: Path) -> None:
  """The record keeps each act of the chain."""
  sand, root, _ = compacting(tmp_path)
  await engine.rung("read('a.txt')\nbash('echo hi')", on=root)
  before = engine.transcript(root)
  await engine.rung("clear()", on=root)
  after = engine.transcript(root)
  record = [a for (a,) in sand.record]
  assert [a for a in before if a not in after] != []
  acts = [a[1] for a in before if a[0] in ("read", "bash")]
  assert len(acts) == 2 and [one for one in acts if one not in {x[1] for x in record}] == []
  names = [a[1] for a in before if engine.question(a)]
  assert [a[1] for a in after if engine.question(a)][: len(names)] == names
