"""cwd, the working directory that the paths of a chain resolve against."""

from conftest import FILES, STANDS, Sand, Where, life, plain, said, settle, texted, verb
from furb import engine


async def test_the_working_directory_of_a_chain_is_the_closest_cd_back_in_its_transcript() -> None:
  """The working directory of a chain is the closest cd back in its transcript."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  verb("cd", root)("/x")
  assert verb("cwd", root)() == "/x"
  verb("cd", root)("/y")
  assert verb("cwd", root)() == "/y"
  await engine.rung("cd('/z')", on=root)
  assert verb("cwd", root)() == "/z"


async def test_the_working_directory_of_a_chain_is_the_directory_of_the_standing_it_stands_on() -> None:
  """The working directory of a chain is the directory of the standing it stands on while no cd stands in its transcript, so a later standing moves no chain that a cd moved."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert STANDS[1] == "/w"
  assert verb("cwd", root)() == "/w"
  two = engine.chain("two")
  await engine.rung("cd('/deep')", on=two)
  await settle()
  assert verb("cwd", two)() == "/deep"
  life(Sand(stands=[STANDS[0], "/z", STANDS[2]], words=FILES), plain(sand.record))
  await settle(300)
  assert verb("cwd", root)() == "/z" and verb("cwd", two)() == "/deep"


async def test_the_world_resolves_the_path_of_a_read_and_a_write_against_the_working_directory() -> None:
  """The World resolves the path of a read and a write against the working directory that cwd gives on the chain of the question."""
  sand = Where(files={"/x/a.txt": "two\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  verb("cd", root)("/x")
  assert texted(verb("read", root)("a.txt")) == ("/x/a.txt", "two\n")
  word = "close(write(Text('b.txt', 'kept'), on=__name__).path)"
  assert await engine.rung(word, on=root) == "/x/b.txt"
  assert sand.files["/x/b.txt"] == "kept"


async def test_cwd_gives_the_working_directory_that_the_paths_of_the_chain_resolve_against() -> None:
  """cwd gives the working directory that the paths of the chain resolve against."""
  sand = Sand(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  assert verb("cwd", root)() == "/w" and texted(verb("read", root)("a.txt"))[0] == "/w/a.txt"
  verb("cd", root)("/x")
  assert verb("cwd", root)() == "/x" and texted(verb("read", root)("a.txt"))[0] == "/x/a.txt"


async def test_cwd_reads_where_the_paths_of_a_chain_resolve_off_the_transcript_that_the_chain_answers() -> None:
  """cwd reads where the paths of a chain resolve off the transcript that the chain answers, which is the closest cd back in what it heard."""
  sand = Sand(stands=STANDS, words=FILES)
  log, root = life(sand)
  verb("cd", root)("/x")
  verb("cd", root)("/deep")
  before = set(engine.asked)
  assert verb("cwd", root)() == "/deep"
  mine = [engine.asked[one] for one in engine.asked if one not in before]
  assert [(a[0], a[3:]) for a in mine] == [("transcript", (root, root)), ("stand", (root,))]
  assert [a[2] for a in said(log, "done") if a[1] == mine[0][1]] == [root]
  heard = engine.outcomes[mine[0][1]]
  assert isinstance(heard, list) and [a[4] for a in heard if a[0] == "cd"] == ["/x", "/deep"]
  assert [a[0] for a in sand.calls] == ["stand"]


async def test_a_chain_has_a_working_directory_of_its_own() -> None:
  """A chain has a working directory of its own."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  two = engine.chain("two")
  await settle()
  verb("cd", two)("/deep")
  assert (verb("cwd", root)(), verb("cwd", two)()) == ("/w", "/deep")
