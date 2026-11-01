"""compact, the chain that goes on with less, under its name."""

from pathlib import Path

from conftest import settle
from extensions.conftest import compacting
from furb import engine


async def test_compact_supersedes_the_chain_of_the_caller(tmp_path: Path) -> None:
  """compact() supersedes the chain of the caller with a filter that applies the filters in order, FILTERS when it is given none, so the chain goes on under its name with less, and gives that chain."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["read('a.txt')", "FILTERS = [lambda acts: []]\nclose([compact(), compact([quiet()])])"]
  act = engine.prompt(list, "compact", on=root)
  assert await act == [root, root]
  await settle()
  (first,) = [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  assert engine.get("chain2") is None
  assert [a for a in engine.transcript(root) if a[0] == "tell" and a[1] == first] == []


async def test_the_next_word_of_that_chain_binds_the_summary_and_regrounds_the_chain(tmp_path: Path) -> None:
  """The next word of that chain binds the summary that compact is given as the quote summary when there is one, and regrounds the chain."""
  sand, root, _ = compacting(tmp_path)
  word = "close([compact(), compact(summary='gist'), compact(summary='a </s:summary> b')])"
  sand.script[root] = [word]
  await engine.prompt(list, "compact", on=root)
  await settle()
  words = [str(a[4]) for a in engine.transcript(root) if a[0] == "rung" and str(a[4]).endswith("reground()")]
  assert words == [
    "reground()",
    "<s:summary>\ngist</s:summary>\nreground()",
    "<s:summary_>\na </s:summary> b</s:summary_>\nreground()",
  ]
  assert (engine.module(root)["summary"], engine.module(root)["summary_"]) == ("gist", "a </s:summary> b")
