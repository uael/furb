"""cd, the verb that moves the working directory of a chain."""

import pytest

from conftest import WORLD, acts, born, paragraphs, said, settle
from furb import engine
from furb.engine import Refused


async def test_a_cd_the_paths_of_its_chain_resolve_against_the_directory_it_came_to_from_then_on() -> None:
  """A cd: the paths of its chain resolve against the directory it came to from then on, and it does nothing else."""
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


async def test_the_world_answers_it_with_the_directory_that_its_path_names() -> None:
  """The World answers it with the directory that its path names, resolved against the working directory of the chain, and refuses a path that names no directory, so a refused cd moves nothing."""
  _, log, root = born(gone={"/x/none"})
  assert [engine.cd("/x", on=root), engine.cd("sub", on=root)] == ["/x", "/x/sub"]
  with pytest.raises(Refused):
    engine.cd("/x/none", on=root)
  held = engine.transcript(root)
  assert [a[4] for a in held if a[0] == "cd"] == ["/x", "sub", "/x/none"]
  assert [(a[1], a[2], a[3]) for a in said(log, "done") if a[1].startswith("cd")][:2] == [
    ("cd1", WORLD, "/x"),
    ("cd2", WORLD, "/x/sub"),
  ]
  assert isinstance(engine.peek("cd3"), Refused) and engine.cwd(on=root) == "/x/sub"


async def test_it_is_a_question_and_no_fact() -> None:
  """It is a question and no fact, since a fact a running word says is heard when the word yields, where a question is answered at once, so the paths of that word resolve against the new directory from then on."""
  _, _, root = born(
    "before = read('a.txt').content\ncd('/x')\nclose([before, cwd(), read('a.txt').content])",
    files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"},
  )
  assert await engine.prompt(list, "move and read", on=root) == ["one\n", "/x", "two\n"]
  await settle()
  assert engine.cwd(on=root) == "/x"


async def test_cd_tells_the_directory_it_came_to() -> None:
  """cd tells the directory it came to."""
  _, _, root = born("cd('x')\nclose(1)")
  assert await engine.prompt(int, "move", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#cd1\ncd1_path = '/w/x'\n\n#prompt1 closed\nprompt1_value = 1"
  was = paragraphs(engine.turns(on=root))
  assert engine.cd("/y", on=root) == "/y" and paragraphs(engine.turns(on=root)) == was
