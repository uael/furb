"""cwd, the working directory that the paths of a chain resolve against."""

from conftest import STANDS, Sand, Where, life, plain, said, settle
from furb import engine
from furb.engine import OPERATOR, Text


async def test_the_working_directory_of_a_chain_is_the_closest_cd_back_in_its_transcript() -> None:
  """The working directory of a chain is the closest cd back in its transcript."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  engine.cd("/x", on=root)
  assert engine.cwd(on=root) == "/x"
  engine.cd("/y", on=root)
  assert engine.cwd(on=root) == "/y"
  await engine.rung("cd('/z')", on=root)
  assert engine.cwd(on=root) == "/z"


async def test_the_working_directory_of_a_chain_is_the_directory_of_the_standing_it_stands_on() -> None:
  """The working directory of a chain is the directory of the standing it stands on while no cd stands in its transcript, so a later standing moves no chain that a cd moved."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
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
  """The World resolves the path of a read, a write and a command against the working directory it asks the chain for."""
  sand = Where(files={"/x/a.txt": "two\n"}, stands=STANDS)
  _, root = life(sand)
  engine.cd("/x", on=root)
  assert engine.read("a.txt", on=root) == Text("/x/a.txt", "two\n")
  assert engine.write(Text("b.txt", "kept"), on=root) == Text("/x/b.txt", "kept")
  assert sand.files["/x/b.txt"] == "kept"
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/x"]


async def test_cwd_gives_the_working_directory_that_the_paths_of_the_chain_resolve_against() -> None:
  """cwd gives the working directory that the paths of the chain resolve against."""
  sand = Sand(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.cwd(on=root) == "/w" and engine.read("a.txt", on=root).path == "/w/a.txt"
  engine.cd("/x", on=root)
  assert engine.cwd(on=root) == "/x" and engine.read("a.txt", on=root).path == "/x/a.txt"


async def test_the_chain_answers_for_where_its_paths_resolve() -> None:
  """The chain answers for where its paths resolve, which is the closest cd back in what it heard."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert engine.cwd(on=root) == "/w"
  engine.cd("/deep", on=root)
  assert engine.cwd(on=root) == "/deep"
  asked = [a for a in engine.asked.values() if a[0] == "cwd"]
  assert asked == [("cwd", "cwd@operator.2", OPERATOR, root), ("cwd", "cwd@operator.4", OPERATOR, root)]
  answered = [a for a in said(log, "done") if a[1].startswith("cwd@")]
  assert answered == [("done", "cwd@operator.2", root, "/w"), ("done", "cwd@operator.4", root, "/deep")]
