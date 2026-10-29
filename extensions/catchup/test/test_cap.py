"""CAP, the lines that differ past which a catch-up tells their count."""

from conftest import paragraphs
from extensions.conftest import CATCHUP, caught_up, constant
from furb import engine

CAP = constant(CATCHUP, "CAP")


async def test_cap_is_the_number_of_lines_that_differ_in_a_file_past_which_a_catch_up_tells_its_count() -> None:
  """CAP is the number of lines that differ in a file past which a catch-up tells the path of the file with that number alone."""
  assert CAP == 40
  lines = "".join(f"{i}\n" for i in range(CAP + 1))
  sand, _, root = await caught_up(files={"/w/a.txt": lines, "/w/b.txt": lines})
  engine.read("a.txt", on=root)
  engine.read("b.txt", on=root)
  sand.files["/w/a.txt"] = lines.replace("\n", "!\n", CAP)
  sand.files["/w/b.txt"] = lines.replace("\n", "!\n")
  engine.stand()
  shown = {i: f"{i - 1}!" for i in range(1, CAP + 1)}
  assert paragraphs(engine.turns(on=root))[-1] == (
    f"#chain1 caught up\nchain1_changed = {{'/w/b.txt': {CAP + 1}, '/w/a.txt': {shown!r}}}"
  )
