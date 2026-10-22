"""compact, the chain that goes on with less."""

from pathlib import Path

from conftest import settle
from extensions.conftest import compacting
from furb import engine


async def test_compact_makes_a_chain_whose_source_is_the_chain_of_the_caller(tmp_path: Path) -> None:
  """compact() makes a chain whose source is the chain of the caller, whose filter applies the filters in order, FILTERS when it is given none, and gives that chain, which it does not prompt."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["made = compact()\nFILTERS = [lambda acts: []]\nclose([made, compact(), compact([quiet()])])"]
  act = engine.prompt(list, "compact", on=root)
  first, bare, loud = await act
  await settle()
  assert [engine.get(one)[3:] for one in (first, bare, loud)] == [(root, "compacted", root)] * 3
  for one in (first, bare, loud):
    assert [a for a in engine.transcript(one) if a[0] == "prompt" and a[3] == one] == []
  kept = [[a[1] for a in engine.transcript(one) if a[0] == "prompt"] for one in (first, bare, loud)]
  assert kept == [[act], [], [act]]


async def test_the_first_word_of_that_chain_restores_and_regrounds_the_chain_of_the_caller(tmp_path: Path) -> None:
  """The first word of that chain restores the chain of the caller, binds the summary that compact is given as the quote summary when there is one, and regrounds the new chain."""
  sand, root, _ = compacting(tmp_path)
  word = "close([compact(), compact(summary='gist'), compact(summary='a </s:summary> b')])"
  sand.script[root] = [word]
  made = await engine.prompt(list, "compact", on=root)
  await settle()
  first = [
    next(str(a[4]) for a in engine.transcript(one) if a[0] == "rung" and str(a[4]).startswith("restore("))
    for one in made
  ]
  assert first == [
    "restore('chain1')\nreground()",
    "restore('chain1')\n<s:summary>\ngist</s:summary>\nreground()",
    "restore('chain1')\n<s:summary_>\na </s:summary> b</s:summary_>\nreground()",
  ]
  assert (engine.module(made[1])["summary"], engine.module(made[2])["summary_"]) == ("gist", "a </s:summary> b")
