"""cd, the verb that moves the working directory of a chain."""

from conftest import FILES, STANDS, Sand, life, paragraphs, settle, texted, verb
from furb import engine


async def test_a_cd_the_paths_of_its_chain_resolve_against_its_path_from_then_on() -> None:
  """A cd: the paths of its chain resolve against its path from then on, and it does nothing else."""
  sand = Sand(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  assert texted(verb("read", root)("a.txt")) == ("/w/a.txt", "one\n")
  made = list(engine.acts)
  assert verb("cd", root)("/x") == "/x"
  assert texted(verb("read", root)("a.txt")) == ("/x/a.txt", "two\n")
  assert list(engine.acts) == made


async def test_cd_completes_at_once_and_gives_the_new_working_directory() -> None:
  """cd completes at once and gives the new working directory."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  got = verb("cd", root)("/x")
  assert got == "/x" and isinstance(got, str)
  assert verb("cwd", root)() == "/x"


async def test_it_gives_the_path_it_was_given_and_the_chain_holds_the_query_it_asks() -> None:
  """It gives the path it was given, and the chain holds the query it asks, so what a chain heard is where its working directory stands."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert [verb("cd", root)("/x"), verb("cd", root)("/y")] == ["/x", "/y"]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a[4] for a in held if a[0] == "cd"] == ["/x", "/y"]
  assert verb("cwd", root)() == "/y"


async def test_it_is_a_query_and_no_fact() -> None:
  """It is a query and no fact, since a fact a running word says is heard when the word yields, where a query is heard at once, so the paths of that word resolve against the new directory from then on."""
  sand = Sand(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  sand.script[root] = ["before = read('a.txt').content\ncd('/x')\nclose([before, cwd(), read('a.txt').content])"]
  assert await engine.prompt(list, "move and read", on=root) == ["one\n", "/x", "two\n"]
  await settle()
  (asked,) = [a for a in engine.asked.values() if a[0] == "cd"]
  assert asked[1] not in engine.acts and asked[2] in engine.acts and asked[3:] == (root, "/x")
  assert verb("cwd", root)() == "/x"


async def test_cd_tells_the_path_it_was_given() -> None:
  """cd tells the path it was given."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  sand.script[root] = ["cd('/x')\nclose(1)"]
  assert await engine.prompt(int, "move", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#cd /x\n\n#prompt1 closed 1"
  was = paragraphs(engine.turns(on=root))
  assert verb("cd", root)("/y") == "/y" and paragraphs(engine.turns(on=root)) == was
