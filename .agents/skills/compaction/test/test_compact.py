"""compact, the chain that jumps to a chain with less."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import compacting


async def test_compact_supersedes_the_chain_of_the_caller(tmp_path: Path) -> None:
  """compact() supersedes the chain of the caller with a filter that applies the filters in order, FILTERS when it is given none, so the chain goes on with less, and gives the jump."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = [
    "read('a.txt')\nbash('echo hi')",
    "await rung(Text('mine.py', 'FILTERS = [lambda acts: []]'))\nclose([compact(), compact([quiet()])])",
  ]
  went = await engine.prompt(list, "compact", on=root)
  await settle()
  (first,) = [a[1] for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  (said,) = [a[1] for a in engine.transcript(root) if a[0] == "bash"]
  new = engine.scope(root)
  assert [engine.get(one)[0] for one in went] == ["jump", "jump"] and {engine.peek(one) for one in went} == {new}
  assert [a for a in engine.transcript(new) if a[1] in (first, said)] == []


async def test_the_next_word_of_that_chain_binds_the_summary_and_regrounds_the_chain(tmp_path: Path) -> None:
  """The first word of the chain that it jumps to binds the summary that compact is given as the quote summary when there is one, and regrounds the chain."""
  sand, root, _ = compacting(tmp_path)
  word = "close([compact(), compact(summary='gist'), compact(summary='a </s:summary> b')])"
  sand.script[root] = [word]
  await engine.prompt(list, "compact", on=root)
  await settle()
  new = engine.scope(root)
  words = [str(a[4]) for a in engine.transcript(new) if a[0] == "rung" and str(a[4]).endswith("reground()")]
  assert words == [
    "reground()",
    "<s:summary>\ngist</s:summary>\nreground()",
    "<s:summary_>\na </s:summary> b</s:summary_>\nreground()",
  ]
  assert (engine.module(new)["summary"], engine.module(new)["summary_"]) == ("gist", "a </s:summary> b")
