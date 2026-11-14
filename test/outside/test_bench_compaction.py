"""The bench of compaction, which finds the choice and the cut of compacted on the chain itself."""

import importlib
from pathlib import Path

import pytest

from conftest import settle
from furb import engine
from skills.conftest import FULL, compacting


async def test_the_bench_finds_the_choice_and_the_cut_on_the_chain_and_no_chain_of_the_choice(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  """The bench finds the prompt of the choice that compacted made on the chain, and the cut in place of that chain."""
  monkeypatch.syspath_prepend(str(Path(__file__).parents[2] / "script" / "bench"))
  bench = importlib.import_module("compaction")
  sand, root, _ = compacting(tmp_path)
  await engine.rung("watch = compacted()", on=root)
  watch = str(engine.module(root)["watch"])
  sand.cost = FULL
  sand.script[root] = ["x = 1\ndebug(t'{x}')", "SHARE = 2\nclose(([], 'gist'))", "close(x)"]
  assert await engine.prompt(int, "go", on=root) == 1
  await settle()
  (choice,) = bench.chosen(root, watch)
  assert engine.peek(choice) == ([], "gist") and bench.cut(root, watch)
  assert bench.compacting(watch) and not bench.compacting(choice)
  assert [a for a in engine.transcript(root) if a[0] == "chain"] == []
