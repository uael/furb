"""cwd, the working directory that the paths of a chain resolve against."""

from pathlib import Path

from conftest import STANDS, Sand, Where, acts, born, life, machine, plain, said, settle
from furb import engine
from furb.engine import Text


async def test_the_working_directory_of_a_chain_is_the_directory_that_the_closest_cd_came_to() -> None:
  """The working directory of a chain is the directory that the closest cd back in its transcript came to."""
  _, _, root = born()
  engine.cd("/x", on=root)
  assert engine.cwd(on=root) == "/x"
  engine.cd("/y", on=root)
  assert engine.cwd(on=root) == "/y"
  await engine.rung("cd('/z')", on=root)
  assert engine.cwd(on=root) == "/z"


async def test_a_chain_whose_transcript_holds_no_cd_has_no_working_directory() -> None:
  """A chain whose transcript holds no cd has no working directory, so cwd gives an empty path, and the World resolves the paths of that chain against the directory of the standing."""
  sand, _, root = born(files={"/w/a.txt": "one\n"})
  assert engine.cwd(on=root) == "" and engine.read("a.txt", on=root) == Text("/w/a.txt", "one\n")
  two = engine.chain("two")
  await engine.rung("cd('/deep')", on=two)
  await settle()
  assert engine.cwd(on=two) == "/deep"
  life(Sand(files={"/z/a.txt": "two\n"}, stands=[STANDS[0], "/z", STANDS[2]]), plain(sand.record))
  await settle(300)
  assert engine.cwd(on=root) == "" and engine.cwd(on=two) == "/deep"
  assert engine.read("a.txt", on=root) == Text("/z/a.txt", "two\n")


async def test_the_world_resolves_the_path_of_a_read_a_write_and_a_command_against_the_working_directory(
  tmp_path: Path,
) -> None:
  """The World resolves the path of a read, a write and a command against the working directory of the chain, which it reads, and the path of a cd, a read or a write that is ~ or starts with ~/, or with ~\\ on Windows, against the home of the user, and one of scratch:// against the scratch directory."""
  sand = Where(files={"/x/a.txt": "two\n"})
  _, root = life(sand)
  engine.cd("/x", on=root)
  assert engine.read("a.txt", on=root) == Text("/x/a.txt", "two\n")
  assert engine.write(Text("b.txt", "kept"), on=root) == Text("/x/b.txt", "kept")
  assert sand.files["/x/b.txt"] == "kept"
  await engine.bash("echo hi", on=root)
  assert sand.where == ["/x"]
  sand, _, root = born(files={"/h/a.txt": "home\n", "/w/~a.txt": "here\n"})
  assert engine.read("~/a.txt", on=root) == Text("~/a.txt", "home\n") and sand.calls[-1][4] == "~/a.txt"
  assert engine.read("~a.txt", on=root) == Text("/w/~a.txt", "here\n")
  assert engine.write(Text("~/b.txt", "kept"), on=root) == Text("~/b.txt", "kept") and sand.files["/h/b.txt"] == "kept"
  (tmp_path / "sub").mkdir()
  live, root = machine(tmp_path)
  engine.write(Text("scratch://b.txt", "kept"), on=root)
  engine.cd("sub", on=root)
  moved = engine.read("scratch://b.txt", on=root)
  live.end()
  assert moved == Text("scratch://b.txt", "kept")


async def test_the_world_writes_each_path_under_the_home_of_the_user_with_a_tilde_in_place_of_the_home() -> None:
  """The World writes each path under the home of the user with ~ in place of the home, as ~/a.txt, in each text, each cd and each standing that it gives, and every other path as it stands, so a path that a model reads names no home."""
  sand, _, root = born(files={"/h/a.txt": "home\n", "/w/a.txt": "here\n"}, stands=[STANDS[0], "/h/w", STANDS[2]])
  assert engine.standing()[1] == "~/w" and engine.cwd(on=root) == ""
  assert engine.read("/h/a.txt", on=root) == Text("~/a.txt", "home\n")
  assert engine.read("/w/a.txt", on=root) == Text("/w/a.txt", "here\n")
  assert (
    engine.write(Text("c.txt", "kept"), on=root) == Text("~/w/c.txt", "kept") and sand.files["/h/w/c.txt"] == "kept"
  )
  assert engine.cd("/h", on=root) == "~" and engine.cd("x", on=root) == "~/x"
  assert engine.cd("/w", on=root) == "/w"


async def test_cwd_gives_the_working_directory_that_the_paths_of_the_chain_resolve_against() -> None:
  """cwd gives the working directory that the paths of the chain resolve against."""
  _, _, root = born(files={"/w/a.txt": "one\n", "/x/a.txt": "two\n"})
  assert engine.cwd(on=root) == "" and engine.read("a.txt", on=root).path == "/w/a.txt"
  engine.cd("/x", on=root)
  assert engine.cwd(on=root) == "/x" and engine.read("a.txt", on=root).path == "/x/a.txt"


async def test_cwd_reads_the_transcript_of_the_chain_and_asks_nothing() -> None:
  """cwd reads the transcript of the chain and asks nothing, so no act is made and the journal keeps nothing."""
  sand, log, root = born()
  made, kept = list(acts(log)), list(sand.record)
  engine.cd("/x", on=root)
  made, kept = list(acts(log)), list(sand.record)
  assert engine.cwd(on=root) == "/x"
  assert list(acts(log)) == made and sand.record == kept and said(log, "cwd") == []
