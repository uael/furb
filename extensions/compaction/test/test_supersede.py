"""supersede, the chain cut to a prefix that goes on under its name."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_supersede_says_on_the_chain_a_prefix_of_what_the_filter_kept(tmp_path: Path) -> None:
  """supersede(keep) says on the chain of the caller a prefix of what the filter keep kept of its acts, everything that made what it kept, and every act of the chain, and gives the chain."""
  sand, root, _ = compacting(tmp_path)
  kept, dropped = await engine.rung("k = 1", on=root), engine.bash("echo hi", on=root)
  before = engine.transcript(root)
  sand.script[root] = [f"close(supersede(take({dropped!r}, inside=False)))"]
  assert kept is None and await engine.prompt(str, "cut", on=root) == root
  after = engine.transcript(root)
  questions = [a[1] for a in before if engine.question(a)]
  assert [a[1] for a in after if engine.question(a)][: len(questions)] == questions
  assert [a for a in before if a[1] == dropped and not engine.question(a)] != []
  assert [a for a in after if a[1] == dropped and not engine.question(a)] == []


async def test_so_the_chain_goes_on_under_its_name(tmp_path: Path) -> None:
  """So the chain goes on under its name with its module, its ladders and its acts, and only its turns are cut."""
  sand, root, _ = compacting(tmp_path)
  assert await engine.rung("k = 7\nread('a.txt')", on=root) is None
  turns = engine.turns(on=root)
  sand.script[root] = ["supersede(lambda _: [])\nclose(k)"]
  assert await engine.prompt(int, "the k", on=root) == 7
  assert engine.get("chain2") is None and len(str(engine.turns(on=root))) < len(str(turns)) + 400
  assert [a for a in engine.transcript(root) if a[0] == "tell" and "a.txt" in str(a)] == []
