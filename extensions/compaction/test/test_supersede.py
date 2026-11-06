"""supersede, the chain cut to a prefix that goes on under its name."""

from pathlib import Path

from conftest import settle
from extensions.conftest import compacting, extended
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


async def test_supersede_keeps_whole_the_setup_that_the_model_reads_first(tmp_path: Path) -> None:
  """supersede(keep) keeps whole the setup of the chain, each fact from its paragraph chain() up to its open, so a cut, even one that keeps nothing, leaves the doctrine and the words of the setup that its model reads first."""
  sand, root = extended("compaction", tmp_path, doctrine="Answer in python alone.\n")
  sand.files[f"{tmp_path / 'work'}/a.txt"] = "one\n"
  await settle()
  before = engine.turns(on=root)
  await engine.rung("read('a.txt')", on=root)
  await engine.rung("clear()", on=root)
  await settle()
  shown = "\n".join(str(turn[1]) for turn in engine.turns(on=root))
  assert "Answer in python alone." in shown
  assert [turn for turn in before if turn not in engine.turns(on=root)] == []
