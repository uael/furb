"""write, the text put at its path."""

import pytest

from conftest import FILES, STANDS, Dead, Sand, World, life, made, of, paragraphs, said, settle, sown, texted, verb
from furb import engine
from furb.engine import OPERATOR, Refused

KEPT = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('write', qid, _, _, path, _) if path.startswith('nums://'):\n"
  "        yield 'done', qid, 7\n"
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
          yield "done", qid, self.stands or [[], "", ""]
        case ("read", qid, _, _, path):
          yield "done", qid, {"path": path, "content": self.files.get(path, "")}
        case ("write", qid, _, _, path, content):
          self.files[path] = self.files.get(path, "") + content
          yield "done", qid, {"path": path, "content": self.files[path]}


class Firm(Sand):
  """A World whose disk holds one line more than it was asked to write."""

  def hears(self) -> World:
    """The World that answers a standing, and a write it takes with a line of its own at the end."""
    while True:
      a = yield
      match a:
        case ("stand", qid, *_):
          yield "done", qid, self.stands or [[], "", ""]
        case ("write", qid, _, _, path, content):
          self.files[path] = content + "END\n"
          yield "done", qid, {"path": path, "content": self.files[path]}


async def test_a_write_whoever_serves_the_path_of_the_text_takes_its_content() -> None:
  """A write: whoever serves the path of the text takes its content."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert texted(verb("write", root)(made(root, "Text", "b.txt", "one\n"))) == ("/w/b.txt", "one\n")
  assert sand.files == {"/w/b.txt": "one\n"}
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert texted(verb("write", root)(made(root, "Text", act, "k = 1"))) == (act, "k = 1")
  await settle()
  assert sand.files == {"/w/b.txt": "one\n"}
  assert engine.modules[root]["k"] == 1


async def test_write_is_given_a_text_and_gives_the_text_as_it_is_on_disk_after_the_write() -> None:
  """write is given a text, and gives the text as it is on disk after the write."""
  sand = Firm(stands=STANDS, words=FILES)
  _, root = life(sand)
  word = "t = Text('b.txt', 'one\\n')\ngot = write(t)\nclose([type(got) is Text, got.path, got.content, t.content])"
  assert await engine.rung(word, on=root) == [True, "b.txt", "one\nEND\n", "one\n"]
  assert sand.files["b.txt"] == "one\nEND\n"


async def test_a_write_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A write that the World refuses raises Refused in the caller."""
  dead = Dead(stands=STANDS, words=FILES)
  _, root = life(dead)
  with pytest.raises(Refused, match="a dead World answers no write"):
    verb("write", root)(made(root, "Text", "b.txt", "one\n"))
  word = "try:\n  write(Text('b.txt', 'one\\n'))\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "a dead World answers no write"


async def test_write_tells_of_a_write_of_a_text_only_the_lines_that_differ() -> None:
  """write tells of a write of a text only the lines that differ from what the caller asked, and of a write a door answers with a value, that value."""
  sand = Firm(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert await engine.rung("write(Text('b.txt', 'one\\ntwo\\n'))", on=root) is None
  assert sand.files == {"b.txt": "one\ntwo\nEND\n"}
  assert of(engine.turns(on=root), "write") == ["#write b.txt\n# b.txt, 0 known\n# 3 END"]
  other = sown(FILES)
  _, two = life(other)
  other.script[two] = [KEPT]
  assert await engine.prompt(int, "a door of my own", on=two) == 7
  assert of(engine.turns(on=two), "write") == ["#write nums://a\n# 7"]


async def test_a_write_takes_no_show() -> None:
  """A write takes no show, since what a write would show the word of the model already said: it tells the lines of what came back that differ from what it asked for, and of those, the lines the model has not seen, so a write that the disk took as it was asked tells nothing at all."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert await engine.rung("write(Text('b.txt', 'one\\ntwo\\n'))", on=root) is None
  assert of(engine.turns(on=root), "write") == []
  with pytest.raises(Refused):
    await engine.rung("write(Text('c.txt', 'x'), HEAD)", on=root)
  assert "c.txt" not in str(sand.files)


async def test_a_door_that_answers_a_write_with_more_than_it_was_asked_for() -> None:
  """A door that answers a write with more than it was asked for tells the lines it added and no line the model read before."""
  sand = Hoard(files={"b.txt": "one\ntwo\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  assert await engine.rung("read('b.txt')\nwrite(Text('b.txt', 'three\\n'))", on=root) is None
  await settle()
  told = [one for one in paragraphs(engine.turns(on=root)) if one.startswith(("#read ", "#write "))]
  assert told == ["#read b.txt\n# b.txt, 0 known\n# 1 one\n# 2 two", "#write b.txt\n# b.txt, 2 known\n# 3 three"]


async def test_a_write_to_a_door_of_a_ladder_gives_the_ladder_its_content_as_a_word() -> None:
  """A write to a door of a ladder gives the ladder its content as a word, so the chain makes the rungs of that ladder again from it, and the write gives the text it was given and tells nothing."""
  sand = Sand(stands=STANDS, words=FILES)
  log, root = life(sand)
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  word = f"t = Text({act!r}, 'k = 21')\nclose(write(t) is t)"
  assert await engine.rung(word, on=root) is True
  await settle()
  assert engine.modules[root]["k"] == 21 and engine.ask("ladder", root, act)[1] == "k = 21"
  assert [a[4] for a in said(log, "rung") if a[2] == act] == ["k = 21"]
  assert of(engine.turns(on=root), "write") == [] and said(sand.calls, "write") == []


async def test_a_write_asks_with_the_path_and_the_content_of_its_text_and_with_no_text() -> None:
  """A write asks with the path and the content of its text, and with no Text, so the record holds plain data."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  step = engine.rung("write(Text('b.txt', 'one\\n'))", on=root)
  assert await step is None
  await settle()
  (asked,) = [e[0] for e in sand.record if e[0][0] == "write"]
  assert asked[2:] == (step, root, "b.txt", "one\n")
  (entry,) = [e for e in sand.record if e[0][0] == "write"]
  assert entry[1:] == ({"path": "/w/b.txt", "content": "one\n"},)


async def test_a_new_file_is_a_write_of_a_text_made_of_its_path_and_its_content() -> None:
  """A new file is a write of a Text made of its path and its content."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert await engine.rung("close(write(Text('new.txt', 'fresh\\n')).path)", on=root) == "/w/new.txt"
  assert sand.files == {"/w/new.txt": "fresh\n"}
