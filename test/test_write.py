"""write, the text put at its path."""

import pytest

from conftest import STANDS, Dead, Sand, World, life, of, paragraphs, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Refused, Text

KEPT = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('write', qid, _, _, Text(path=path)) if path.startswith('nums://'):\n"
  "        say('done', qid, 7)\n"
  "\n"
  "act('nums', '', kept)\n"
  "close(write(Text('nums://a', 'x')))\n"
)
"""A word of a rung that opens a door of its own, which answers a write with a number, and gives what it wrote."""


class Hoard(Sand):
  """A World that keeps what stands at a path and gives it back with what is written after it."""

  def hears(self) -> World:
    """The World that answers a standing, a read of what it holds, and a write with the whole of what it holds."""
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          engine.say("done", qid, self.stands or [[], "", ""])
        case ("read", qid, _, _, path):
          engine.say("done", qid, Text(path, self.files.get(path, "")))
        case ("write", qid, _, _, Text(path=path, content=content)):
          self.files[path] = self.files.get(path, "") + content
          engine.say("done", qid, Text(path, self.files[path]))


class Firm(Sand):
  """A World whose disk holds one line more than it was asked to write."""

  def hears(self) -> World:
    """The World that answers a standing, and a write it takes with a line of its own at the end."""
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          self.calls.append(a)
          engine.say("done", qid, self.stands or [[], "", ""])
        case ("write", qid, _, _, Text(path=path, content=content)):
          self.calls.append(a)
          self.files[path] = content + "END\n"
          engine.say("done", qid, Text(path, self.files[path]))


async def test_a_write_whoever_serves_the_path_of_the_text_takes_its_content() -> None:
  """A write: whoever serves the path of the text takes its content."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
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
  sand = Sand(stands=STANDS)
  _, root = life(sand)
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
  dead = Dead(stands=STANDS)
  _, root = life(dead)
  with pytest.raises(Refused, match="a dead World answers no write"):
    engine.write(Text("b.txt", "one\n"), on=root)
  word = "try:\n  write(Text('b.txt', 'one\\n'))\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "a dead World answers no write"


async def test_the_engine_tells_of_a_write_of_a_text_only_the_lines_that_differ() -> None:
  """The engine tells of a write of a text only the lines that differ from what the caller asked, and of a write a door answers with a value, that value."""
  sand = Firm(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("write(Text('b.txt', 'one\\ntwo\\n'))", on=root) is None
  assert sand.files == {"b.txt": "one\ntwo\nEND\n"}
  assert of(engine.turns(on=root), "write") == ["#write b.txt\n# b.txt, 0 known\n# 3 END"]
  other = sown()
  _, two = life(other)
  other.script[two] = [KEPT]
  assert await engine.prompt(int, "a door of my own", on=two) == 7
  assert of(engine.turns(on=two), "write") == ["#write nums://a\n# 7"]


async def test_a_write_takes_no_show() -> None:
  """A write takes no show, since what a write would show the word of the model already said: it tells the lines of what came back that differ from what it asked for, and of those, the lines the model has not seen, so a write that the disk took as it was asked tells nothing at all."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("write(Text('b.txt', 'one\\ntwo\\n'))", on=root) is None
  assert of(engine.turns(on=root), "write") == []
  held = engine.transcript(root)
  assert said(held, "write")[0][4:] == (Text("b.txt", "one\ntwo\n"),)


async def test_a_door_that_answers_a_write_with_more_than_it_was_asked_for() -> None:
  """A door that answers a write with more than it was asked for tells the lines it added and no line the model read before."""
  sand = Hoard(files={"b.txt": "one\ntwo\n"}, stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("read('b.txt')\nwrite(Text('b.txt', 'three\\n'))", on=root) is None
  await settle()
  told = [one for one in paragraphs(engine.turns(on=root)) if one.startswith(("#read ", "#write "))]
  assert told == ["#read b.txt\n# b.txt, 0 known\n# 1 one\n# 2 two", "#write b.txt\n# b.txt, 2 known\n# 3 three"]


async def test_a_write_to_the_door_of_a_prompt_edits_the_program_of_its_ladder() -> None:
  """A write to the door of a prompt edits the program of its ladder, so the door and the verb are one act."""
  sand = sown()
  log, root = life(sand)
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  got = engine.write(Text(act, "k = 21"), on=root)
  await settle()
  assert got == Text(act, "k = 21")
  held = engine.transcript(root)
  assert said(held, "write") == [("write", "write1", OPERATOR, root, Text(act, "k = 21"))]
  assert [(a[2], a[4], a[5]) for a in said(log, "rung")] == [(act, "k = 21", "")]
  engine.write(Text(act, "k = 21\nk = 22"), on=root)
  await settle()
  first = said(log, "rung")[0][1]
  assert [(a[2], a[4], a[5]) for a in said(log, "rung")] == [
    (act, "k = 21", ""),
    (root, "k = 21", first),
    (act, "k = 22", ""),
  ]
  assert engine.read(act, on=root) == Text(act, "k = 21\nk = 22") and engine.module(root)["k"] == 22


async def test_the_chain_answers_a_write_of_the_door_of_one_of_its_prompts_with_the_text_it_took() -> None:
  """The chain answers a write of the door of one of its prompts with the text it took, and makes its rungs again from it."""
  sand = sown()
  log, root = life(sand)
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  got = engine.write(Text(act, "k = 21"), on=root)
  await settle()
  assert engine.module(root)["k"] == 21
  assert [a[4] for a in said(log, "rung")] == ["k = 21"]
  assert got == Text(act, "k = 21") and [a[0] for a in sand.calls] == ["stand", "prompt"]
  again = engine.write(Text(act, "k = 22"), on=root)
  await settle()
  assert again == Text(act, "k = 22") and engine.read(act, on=root) == Text(act, "k = 22")
  assert [a[4] for a in said(log, "rung")] == ["k = 21", "k = 22"] and engine.module(root)["k"] == 22
  assert [a[0] for a in sand.calls] == ["stand", "prompt"]


async def test_a_write_of_a_door_that_a_rung_of_that_ladder_says_leaves_the_word_of_that_rung_out() -> None:
  """A write of a door that a rung of that ladder says leaves the word of that rung out, and that rung is no rung of the chain after it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = [
    "k = 1",
    "ok = BAD",
    "mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))",
    "close((k, ok, read(get(acting())[2]).content))",
  ]
  act = engine.prompt(object, "fix it", on=root)
  assert await act == (1, 2, "k = 1\nok = 2")
  await settle()
  binding, first, writer = (a[4] for a in said(log, "run")[:3])
  assert [(a[5], a[6]) for a in said(log, "run")] == [
    (f"chain1: Act[object] = Act('chain1')\n{act}: Act[object] = Act({act!r})", ""),
    ("k = 1", ""),
    ("mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))", ""),
    (f"chain1: Act[object] = Act('chain1')\n{act}: Act[object] = Act({act!r})", binding),
    ("k = 1", first),
    ("ok = 2", ""),
    ("close((k, ok, read(get(acting())[2]).content))", ""),
  ]
  program = engine.program(root)
  assert isinstance(program, dict) and writer not in program
  assert list(program.values()) == [a[5] for a in said(log, "run")[3:]]
  assert engine.read(act, on=root) == Text(act, "k = 1\nok = 2\nclose((k, ok, read(get(acting())[2]).content))")


async def test_a_new_file_is_a_write_of_a_text_made_of_its_path_and_its_content() -> None:
  """A new file is a write of a Text made of its path and its content."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert "/w/new.txt" not in sand.files
  assert engine.write(Text("new.txt", "one\n"), on=root) == Text("/w/new.txt", "one\n")
  assert sand.files["/w/new.txt"] == "one\n"
  assert engine.read("new.txt", on=root) == Text("/w/new.txt", "one\n")
