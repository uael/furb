"""cd, the verb that moves the working directory of a chain."""

from conftest import STANDS, Sand, life, paragraphs, settle
from furb import engine


async def test_a_cd_the_paths_of_its_chain_resolve_against_its_path_from_then_on() -> None:
  """A cd: the paths of its chain resolve against its path from then on, and it does nothing else."""
  sand = Sand(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.read("a.txt", on=root).content == "one\n"
  made = list(engine.acts)
  assert engine.cd("/x", on=root) == "/x"
  assert engine.read("a.txt", on=root).content == "two\n"
  assert list(engine.acts) == made


async def test_cd_completes_at_once_and_gives_the_new_working_directory() -> None:
  """cd completes at once and gives the new working directory."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  got = engine.cd("/x", on=root)
  assert got == "/x" and isinstance(got, str)
  assert engine.cwd(on=root) == "/x"


async def test_it_answers_with_the_path_it_was_given_and_the_chain_that_hears_it_holds_it() -> None:
  """It answers with the path it was given, and the chain that hears it holds it, so what a chain heard is where its working directory stands."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert [engine.cd("/x", on=root), engine.cd("/y", on=root)] == ["/x", "/y"]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a[4] for a in held if a[0] == "cd"] == ["/x", "/y"]
  assert engine.cwd(on=root) == "/y"


async def test_it_is_a_question_and_no_fact() -> None:
  """It is a question and no fact, since a fact a running word says is heard when the word yields, where a question is answered at once, so the paths of that word resolve against the new directory from then on."""
  sand = Sand(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"}, stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["before = read('a.txt').content\ncd('/x')\nclose([before, cwd(), read('a.txt').content])"]
  assert await engine.prompt(list, "move and read", on=root) == ["one\n", "/x", "two\n"]
  await settle()
  assert engine.cwd(on=root) == "/x"


async def test_cd_tells_the_path_it_was_given() -> None:
  """cd tells the path it was given."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["cd('/x')\nclose(1)"]
  assert await engine.prompt(int, "move", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#cd /x\n\n#prompt1 closed 1"
  was = paragraphs(engine.turns(on=root))
  assert engine.cd("/y", on=root) == "/y" and paragraphs(engine.turns(on=root)) == was
