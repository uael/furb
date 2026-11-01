"""write, the text put at its path."""

from pathlib import Path

import pytest

from conftest import Dead, Sand, World, born, fresh, life, machine, of, paragraphs, prompted, said, settle
from furb import engine
from furb.engine import OPERATOR, Refused, Text
from furb_monty import _monty

KEPT = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('write', qid, _, _, Text(path=path)) if path.startswith('nums://'):\n"
  "        yield 'done', qid, 7\n"
  "\n"
  "act('nums', '', kept)\n"
  "close(write(Text('nums://a', 'x')))\n"
)
"""A word of a rung that opens a door of its own, which answers a write with a number, and gives what it wrote."""


class Hoard(Sand):
  """A World that keeps what stands at a path and gives it back with what is written after it."""

  def hear(self, a: tuple) -> World:
    """The World of the suite, but for a read of what it holds, and a write it answers with the whole of what it
    holds."""
    match a:
      case ("read", qid, _, _, path):
        yield "done", qid, Text(path, self.files.get(path, ""))
      case ("write", qid, _, _, Text(path=path, content=content)):
        self.files[path] = self.files.get(path, "") + content
        yield "done", qid, Text(path, self.files[path])
      case _:
        yield from super().hear(a)


class Firm(Sand):
  """A World whose disk holds one line more than it was asked to write."""

  def hear(self, a: tuple) -> World:
    """The World of the suite, but for a write, which it takes with a line of its own at the end."""
    match a:
      case ("write", qid, _, _, Text(path=path, content=content)):
        self.files[path] = content + "END\n"
        yield "done", qid, Text(path, self.files[path])
      case _:
        yield from super().hear(a)


async def test_a_write_whoever_serves_the_path_of_the_text_takes_its_content() -> None:
  """A write: whoever serves the path of the text takes its content."""
  sand, _, root = born(files={})
  assert engine.write(Text("b.txt", "one\n"), on=root) == Text("/w/b.txt", "one\n")
  assert sand.files == {"/w/b.txt": "one\n"}
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert engine.write(Text(act, "k = 1"), on=root) == Text(act, "k = 1")
  await settle()
  assert sand.files == {"/w/b.txt": "one\n"}
  assert engine.module(root)["k"] == 1
  assert engine.read(act, on=root) == Text(act, "k = 1")


async def test_write_is_given_a_text_and_gives_the_text_as_it_is_on_disk_after_the_write() -> None:
  """write is given a text, and gives the text as it is on disk after the write."""
  sand, _, root = born()
  assert engine.write(Text("b.txt", "one\n"), on=root) == Text("/w/b.txt", "one\n")
  assert engine.write(Text("b.txt", "two\n"), on=root) == Text("/w/b.txt", "two\n")
  assert sand.files["/w/b.txt"] == "two\n"
  assert engine.read("b.txt", on=root) == Text("/w/b.txt", "two\n")
  held = engine.transcript(root)
  asked = said(held, "write")[0]
  assert asked[4] == Text("b.txt", "one\n")
  assert [a[3] for a in said(held, "done") if a[1] == asked[1]] == [Text("/w/b.txt", "one\n")]


async def test_a_write_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A write that the World refuses raises Refused in the caller."""
  _, root = life(Dead())
  with pytest.raises(Refused, match="a dead World answers no write"):
    engine.write(Text("b.txt", "one\n"), on=root)
  word = "try:\n  write(Text('b.txt', 'one\\n'))\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "a dead World answers no write"


async def test_the_engine_tells_of_a_write_of_a_text_only_the_lines_that_differ() -> None:
  """The engine tells a write of a text by its header and its path, with the lines that differ from what the caller asked when some do, and a write that a door answers with a value by that value."""
  sand = Firm()
  _, root = life(sand)
  assert await engine.rung("write(Text('b.txt', 'one\\ntwo\\n'))", on=root) is None
  assert sand.files == {"b.txt": "one\ntwo\nEND\n"}
  assert of(engine.turns(on=root), "write") == ["#write1\nwrite1_path = 'b.txt'\nwrite1_text = 'END'"]
  _, _, two = born(KEPT)
  assert await engine.prompt(int, "a door of my own", on=two) == 7
  assert of(engine.turns(on=two), "write") == ["#write1\nwrite1_path = 'nums://a'\nwrite1_value = 7"]


async def test_a_write_takes_no_show() -> None:
  """A write takes no show, since what a write would show the word of the model already said: it tells its header and its path, so the turns show each write that landed, and the lines of what came back that differ from what it asked for, so a write that the disk took as it was asked tells no line."""
  _, _, root = born()
  assert await engine.rung("write(Text('b.txt', 'one\\ntwo\\n'))", on=root) is None
  assert of(engine.turns(on=root), "write") == ["#write1\nwrite1_path = 'b.txt'"]
  held = engine.transcript(root)
  assert said(held, "write")[0][4:] == (Text("b.txt", "one\ntwo\n"),)


async def test_a_door_that_answers_a_write_with_more_than_it_was_asked_for() -> None:
  """A door that answers a write with more than it was asked for tells each line of what came back that differs from the line it was asked for at the same place."""
  sand = Hoard(files={"b.txt": "one\ntwo\n"})
  _, root = life(sand)
  assert await engine.rung("read('b.txt')\nwrite(Text('b.txt', 'three\\n'))", on=root) is None
  await settle()
  told = [one for one in paragraphs(engine.turns(on=root)) if one.startswith(("#read", "#write"))]
  assert told == [
    "#read1\nread1_path = 'b.txt'\n<s:read1_text>\none\ntwo</s:read1_text>",
    "#write1\nwrite1_path = 'b.txt'\n<s:write1_text>\none\ntwo\nthree</s:write1_text>",
  ]


async def test_a_write_to_the_door_of_a_prompt_edits_the_program_of_its_ladder() -> None:
  """A write to the door of a prompt edits the program of its ladder, so the door and the verb are one act."""
  _, log, root = born()
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  got = engine.write(Text(act, "k = 21"), on=root)
  await settle()
  assert got == Text(act, "k = 21")
  held = engine.transcript(root)
  assert said(held, "write") == [("write", "write1", OPERATOR, root, Text(act, "k = 21"))]
  assert [(a[2], a[4], a[5]) for a in said(log, "rung")] == [(act, "k = 21", "")]
  engine.write(Text(act, "k = 21\nk = 22"), on=root)
  await settle()
  assert [(a[2], a[4], a[5]) for a in said(log, "rung")] == [(act, "k = 21", ""), (act, "k = 22", "")]
  assert engine.read(act, on=root) == Text(act, "k = 21\nk = 22") and engine.module(root)["k"] == 22


async def test_the_chain_answers_a_write_of_the_door_of_one_of_its_prompts_with_the_text_it_took() -> None:
  """The chain answers a write of the door of one of its prompts with the text it took, and makes its ladder from it."""
  sand, log, root = born()
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  got = engine.write(Text(act, "k = 21"), on=root)
  await settle()
  assert engine.module(root)["k"] == 21
  assert [a[4] for a in said(log, "rung")] == ["k = 21"]
  assert got == Text(act, "k = 21") and [a[0] for a in sand.calls] == ["stand", "prompt"]
  again = engine.write(Text(act, "k = 22"), on=root)
  await settle()
  assert again == Text(act, "k = 22") and engine.read(act, on=root) == Text(act, "k = 22")
  bound = fresh(root, prompted(act, "int", "count"))
  assert [a[4] for a in said(log, "rung")] == ["k = 21", bound, "k = 22"] and engine.module(root)["k"] == 22
  assert [a[0] for a in sand.calls] == ["stand", "prompt"]


async def test_a_write_of_a_door_that_a_rung_of_that_ladder_says_leaves_the_word_of_that_rung_out() -> None:
  """A write of a door that a rung of that ladder says leaves the word of that rung out of the ladder, so the door shows it no more."""
  _, log, root = born(
    "k = 1",
    "ok = BAD",
    "mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))\nwrite(read(mine))",
    "close((k, ok, read(get(acting())[2]).content))",
  )
  act = engine.prompt(object, "fix it", on=root)
  assert await act == (1, 2, "k = 1\nok = 2")
  await settle()
  one, _, writer, made, last = [a[1] for a in said(log, "rung") if a[2] == act]
  read = f"#read1\nread1_path = {act!r}\n<s:read1_text>\nk = 1\nok = BAD</s:read1_text>"
  assert said(log, "run")[4][4:] == (
    writer,
    "mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))\nwrite(read(mine))",
    "",
  )
  assert said(log, "run")[5][4:] == (f"{made}_told", engine.unquoted(read), "")
  assert engine.read(act, on=root) == Text(act, "k = 1\nok = 2\nclose((k, ok, read(get(acting())[2]).content))")
  assert engine.read(act, on=root) == Text(act, "\n".join(engine.program(root)[x] for x in (one, made, last)))


async def test_a_new_file_is_a_write_of_a_text_made_of_its_path_and_its_content() -> None:
  """A new file is a write of a Text made of its path and its content."""
  sand, _, root = born()
  assert "/w/new.txt" not in sand.files
  assert engine.write(Text("new.txt", "one\n"), on=root) == Text("/w/new.txt", "one\n")
  assert sand.files["/w/new.txt"] == "one\n"
  assert engine.read("new.txt", on=root) == Text("/w/new.txt", "one\n")


async def test_the_chain_makes_the_rungs_of_a_write_of_a_door_from_its_text_less_the_line_breaks_at_its_end() -> None:
  """The chain makes the rungs of a write of a door from its text less the line breaks at its end, since a read of the door joins the words of its rungs with one, so an append to the door makes a rung of what it adds alone."""
  _, log, root = born()
  act = engine.prompt(None, "work", to=OPERATOR, on=root)
  engine.write(Text(act, "bash('a')\n"), on=root)
  await settle()
  for more in ("bash('b')", "bash('c')"):
    engine.write(engine.read(act, on=root).append(more), on=root)
    await settle()
  assert [a[4] for a in said(log, "bash")] == ["a", "b", "c"]
  assert engine.read(act, on=root) == Text(act, "bash('a')\nbash('b')\nbash('c')")


async def test_a_write_of_scratch_puts_its_text_in_that_file_of_the_scratch_directory(tmp_path: Path) -> None:
  """A write of scratch://plan.md puts its text in that file of the scratch directory, and makes the directory and the folders of the file first when they are not there."""
  record = tmp_path / "record.jsonl"
  live, root = machine(tmp_path, record)
  place = Path(_monty.scratch(str(record)) or tmp_path / "none")
  assert not place.exists()
  wrote = engine.write(Text("scratch://deep/er/plan.md", "plan\n"), on=root)
  live.end()
  assert wrote == Text("scratch://deep/er/plan.md", "plan\n")
  assert (place / "deep" / "er" / "plan.md").read_text() == "plan\n"


async def test_a_write_to_a_file_whose_every_line_ends_with_crlf_ends_each_line_of_the_text_with_crlf(
  tmp_path: Path,
) -> None:
  """A write to a file whose every line ends with CRLF ends each line of the text with CRLF, so the file keeps its line ends, and gives the text with LF, as a read does."""
  (at := tmp_path / "crlf.txt").write_bytes(b"one\r\n")
  live, root = machine(tmp_path)
  wrote = engine.write(Text(str(at), "one\ntwo\n"), on=root)
  fresh_one = engine.write(Text(str(tmp_path / "new.txt"), "one\n"), on=root)
  live.end()
  assert (wrote.content, at.read_bytes(), fresh_one.content, (tmp_path / "new.txt").read_bytes()) == (
    "one\ntwo\n",
    b"one\r\ntwo\r\n",
    "one\n",
    b"one\n",
  )


async def test_the_world_refuses_a_write_of_a_path_whose_first_part_is_the_name_of_an_act(tmp_path: Path) -> None:
  """The World refuses a write of a path whose first part is the name of an act, and writes no file, since that path is a door that no ear of the engine served, and the refusal names the chain of that act."""
  live, root = machine(tmp_path)
  away = engine.chain("away")
  asked = engine.wait(60.0, on=away)
  with pytest.raises(Refused) as one:
    engine.write(Text(asked, "x\n"), on=root)
  live.end()
  assert str(one.value) == f"{asked} is no file, since {asked} is an act of {away}: write it on {away}."
  assert not (tmp_path / asked).exists()
