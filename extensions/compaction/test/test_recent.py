"""recent, the filter of the last replies."""

from pathlib import Path

from conftest import of, settle
from extensions.conftest import compacting
from furb import engine


async def test_recent_keeps_the_rungs_of_the_last_n_replies(tmp_path: Path) -> None:
  """recent(n) keeps the rungs of the last n replies, and all that they made."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["read('a.txt')\nclose(1)"]
  assert await engine.prompt(int, "read", on=root) == 1
  sand.script[root] = ["x = bash('echo hi')\nawait x", "close(chain('two', 'chain1', recent(2)))"]
  two = await engine.prompt(str, "run and compact", on=root)
  await settle()
  assert [len(of(engine.turns(on=one), "read")) for one in (root, two)] == [1, 0]
  assert len(of(engine.turns(on=two), "bash")) == 2
  said = [turn[1] for turn in engine.turns(on=two) if turn[0] == "assistant"]
  assert said == ["x = bash('echo hi')\nawait x", "close(chain('two', 'chain1', recent(2)))"]
