"""cd, the verb that moves the working directory of a chain."""

from conftest import acts, born, paragraphs, said, settle
from furb import engine


async def test_a_cd_the_paths_of_its_chain_resolve_against_its_path_from_then_on() -> None:
  """A cd: the paths of its chain resolve against its path from then on, and it does nothing else."""
  _, log, root = born(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"})
  assert engine.read("a.txt", on=root).content == "one\n"
  made = list(acts(log))
  assert engine.cd("/x", on=root) == "/x"
  assert list(acts(log)) == [*made, "cd1"]
  assert engine.read("a.txt", on=root).content == "two\n"


async def test_cd_completes_at_once_and_gives_the_new_working_directory() -> None:
  """cd completes at once and gives the new working directory."""
  _, _, root = born()
  got = engine.cd("/x", on=root)
  assert got == "/x" and isinstance(got, str)
  assert engine.cwd(on=root) == "/x"


async def test_the_chain_answers_it_with_the_path_it_was_given_and_holds_it() -> None:
  """The chain answers it with the path it was given, and holds it, so what a chain heard is where its working directory stands."""
  _, log, root = born()
  assert [engine.cd("/x", on=root), engine.cd("/y", on=root)] == ["/x", "/y"]
  held = engine.transcript(root)
  assert [a[4] for a in held if a[0] == "cd"] == ["/x", "/y"]
  assert [a for a in said(log, "done") if a[1].startswith("cd")] == [
    ("done", "cd1", root, "/x"),
    ("done", "cd2", root, "/y"),
  ]
  assert engine.cwd(on=root) == "/y"


async def test_it_is_a_question_and_no_fact() -> None:
  """It is a question and no fact, since a fact a running word says is heard when the word yields, where a question is answered at once, so the paths of that word resolve against the new directory from then on."""
  _, _, root = born(
    "before = read('a.txt').content\ncd('/x')\nclose([before, cwd(), read('a.txt').content])",
    files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"},
  )
  assert await engine.prompt(list, "move and read", on=root) == ["one\n", "/x", "two\n"]
  await settle()
  assert engine.cwd(on=root) == "/x"


async def test_cd_tells_the_path_it_was_given() -> None:
  """cd tells the path it was given."""
  _, _, root = born("cd('/x')\nclose(1)")
  assert await engine.prompt(int, "move", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#cd /x\n\n#prompt1 closed 1"
  was = paragraphs(engine.turns(on=root))
  assert engine.cd("/y", on=root) == "/y" and paragraphs(engine.turns(on=root)) == was
