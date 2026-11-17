"""supersede, the chain that jumps to a chain that keeps what a filter keeps."""

from pathlib import Path

from conftest import settle
from furb import engine
from skills.conftest import compacting, extended


async def test_supersede_jumps_the_chain_to_a_chain_that_keeps_what_the_filter_kept(tmp_path: Path) -> None:
  """supersede(keep) jumps the chain of the caller to a chain that keeps what the filter keep kept of its acts, and runs word there first, and gives the jump."""
  sand, root, _ = compacting(tmp_path)
  # Two turns that the chain carried from another record, each an act that a filter keeps or drops by its name.
  turn = ("assistant", "x = 1", (1, 2, 3, 4, 0.5), None)
  carried = [
    (kind, f"reply{n}", f"carry{n}", *words) for n in (0, 7) for kind, *words in [("reply", root, "m"), ("done", turn)]
  ]
  engine.say("prefix", root, carried)
  kept, dropped = engine.bash("echo kept", on=root), engine.bash("echo hi", on=root)
  await settle()
  sand.script[root] = [f"close(supersede(take({dropped!r}, 'reply0', inside=False), word='w = 1'))"]
  went = await engine.prompt(str, "cut", on=root)
  await settle()
  new = str(engine.peek(went))
  assert (engine.get(went)[0], engine.get(new)[5], engine.scope(root)) == ("jump", root, new)
  held = {a[1] for a in engine.transcript(new)}
  assert (kept in held, dropped in held, engine.module(new)["w"]) == (True, False, 1)
  assert [a for a in engine.transcript(new) if a in carried] == carried[2:]


async def test_so_each_name_of_the_chain_names_a_chain_with_fewer_turns(tmp_path: Path) -> None:
  """So each name of the chain then names a chain with fewer turns, with its module, its open prompts and its threads, and the turns of the chain stay as its model read them."""
  sand, root, _ = compacting(tmp_path)
  assert await engine.rung("k = 7\nread('a.txt')", on=root) is None
  asked = engine.prompt(int, "n", to=engine.OPERATOR, on=root)
  sand.script[root] = ["supersede(lambda _: [])", "close(k)"]
  the = engine.prompt(int, "the k", on=root)
  assert await the == 7
  turns, new = engine.turns(on=root), engine.scope(root)
  assert new != root and len(str(engine.turns(on=new))) < len(str(turns))
  assert [a for a in engine.transcript(new) if a[0] == "tell" and "a.txt" in str(a)] == []
  assert "a.txt" in str(turns) and not engine.over(asked) and engine.scope(asked) == new


async def test_supersede_keeps_whole_the_setup_that_the_model_reads_first(tmp_path: Path) -> None:
  """supersede(keep) keeps whole the setup of the chain, so a jump, even one that keeps nothing, leaves the doctrine and the words of the setup that its model reads first."""
  sand, root = extended("compaction", tmp_path, doctrine="Answer in python alone.\n")
  sand.files[f"{tmp_path / 'work'}/a.txt"] = "one\n"
  await settle()
  before = engine.turns(on=root)
  await engine.rung("read('a.txt')", on=root)
  await engine.rung("clear()", on=root)
  await settle()
  after = engine.turns(on=engine.scope(root))
  assert "Answer in python alone." in "\n".join(str(turn[1]) for turn in after)
  assert [turn for turn in before[:-1] if turn not in after] == []


async def test_supersede_reads_no_name_that_the_module_of_the_chain_binds(tmp_path: Path) -> None:
  """With no filter, supersede() keeps every act of the chain, and it reads no builtin from the module of the chain, which a word may bind to a str."""
  _, root, _ = compacting(tmp_path)
  assert await engine.rung("read('a.txt')", on=root) is None
  acts = {a[1] for a in engine.transcript(root) if engine.question(a)}
  assert await engine.rung("globals()['list'] = 'ls -la'\ncut = supersede()", on=root) is None
  await settle()
  new = str(engine.peek(engine.module(root)["cut"]))
  assert acts <= {a[1] for a in engine.transcript(new)}
  assert engine.module(new)["list"] == "ls -la"
