"""cwd, the working directory that the paths of a chain resolve against."""

from conftest import STANDS, Sand, Where, acts, born, life, plain, said, settle
from furb import engine
from furb.engine import Text


async def test_the_working_directory_of_a_chain_is_the_closest_cd_back_in_its_transcript() -> None:
  """The working directory of a chain is the closest cd back in its transcript."""
  _, _, root = born()
  engine.cd("/x", on=root)
  assert engine.cwd(on=root) == "/x"
  engine.cd("/y", on=root)
  assert engine.cwd(on=root) == "/y"
  await engine.rung("cd('/z')", on=root)
  assert engine.cwd(on=root) == "/z"


async def test_the_working_directory_of_a_chain_is_the_directory_of_the_standing_it_stands_on() -> None:
  """The working directory of a chain is the directory of the standing it stands on while no cd stands in its transcript, so a later standing moves no chain that a cd moved."""
  sand, _, root = born()
  assert STANDS[1] == "/w"
  assert engine.cwd(on=root) == "/w"
  two = engine.chain("two")
  await engine.rung("cd('/deep')", on=two)
  await settle()
  assert engine.cwd(on=two) == "/deep"
  life(Sand(stands=[STANDS[0], "/z", STANDS[2]]), plain(sand.record))
  await settle(300)
  assert engine.cwd(on=root) == "/z" and engine.cwd(on=two) == "/deep"


async def test_the_world_resolves_the_path_of_a_read_a_write_and_a_command_against_the_working_directory() -> None:
  """The World resolves the path of a read, a write and a command against the working directory of the chain, which it reads."""
  sand = Where(files={"/x/a.txt": "two\n"})
  _, root = life(sand)
  engine.cd("/x", on=root)
  assert engine.read("a.txt", on=root) == Text("/x/a.txt", "two\n")
  assert engine.write(Text("b.txt", "kept"), on=root) == Text("/x/b.txt", "kept")
  assert sand.files["/x/b.txt"] == "kept"
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/x"]


async def test_cwd_gives_the_working_directory_that_the_paths_of_the_chain_resolve_against() -> None:
  """cwd gives the working directory that the paths of the chain resolve against."""
  _, _, root = born(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"})
  assert engine.cwd(on=root) == "/w" and engine.read("a.txt", on=root).path == "/w/a.txt"
  engine.cd("/x", on=root)
  assert engine.cwd(on=root) == "/x" and engine.read("a.txt", on=root).path == "/x/a.txt"


async def test_cwd_reads_the_transcript_of_the_chain_and_asks_nothing() -> None:
  """cwd reads the transcript of the chain and asks nothing, so no act is made and the journal keeps nothing."""
  sand, log, root = born()
  made, kept = list(acts(log)), list(sand.record)
  assert engine.cwd(on=root) == "/w"
  assert list(acts(log)) == made and sand.record == kept and said(log, "cwd") == []
