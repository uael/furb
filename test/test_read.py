"""read, the text at a path."""

import shlex
from pathlib import Path

import pytest

from conftest import NUMS, Dead, born, life, machine, of, paragraphs, said, settle
from furb import engine
from furb.engine import HEAD, HIDDEN, LARGEST, OPERATOR, Refused, Text, span
from furb_monty import _monty

BIG = "".join(f"line {i}\n" for i in range(1, 2101))
"""A text of two thousand and one hundred lines, which is longer than HEAD."""


async def test_a_read_whoever_serves_the_path_answers_it_with_the_text_of_it() -> None:
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines its show picks."""
  _, _, root = born("t = read('a.txt')\nsame = read('a.txt')\nclose([len(t.lines), same.content])")
  assert await engine.prompt(list, "read them", on=root) == [2, "one\ntwo\n"]
  assert of(engine.turns(on=root), "read") == [
    "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>",
    "#read2\nread2_path = 'a.txt'\n<s:read2_text>\none\ntwo</s:read2_text>",
  ]


async def test_a_read_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A read that the World refuses raises Refused in the caller."""
  _, root = life(Dead())
  with pytest.raises(Refused, match="a dead World answers no read"):
    engine.read("a.txt", on=root)
  word = "try:\n  read('a.txt')\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=root) == "a dead World answers no read"


async def test_the_engine_judges_no_scheme() -> None:
  """The engine judges no scheme, so a path of an unknown scheme goes to the World too."""
  sand, _, root = born(files={"weird://x": "kept\n"})
  assert engine.read("weird://x", on=root) == Text("weird://x", "kept\n")
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x"]
  with pytest.raises(Refused, match="nothing takes read"):
    engine.read("weird://y", on=root)
  assert [a[4] for a in said(sand.calls, "read")] == ["weird://x", "weird://y"]


async def test_read_is_given_a_path_and_a_show() -> None:
  """read is given a path and a show."""
  _, _, root = born("read('a.txt', span(2, 2))\nread('a.txt', grep('^o'))\nclose(1)")
  assert await engine.prompt(int, "read them", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    "#read1\nread1_path = 'a.txt'\nread1_text = 'two'",
    "#read2\nread2_path = 'a.txt'\nread2_text = 'one'",
  ]


async def test_read_gives_a_text() -> None:
  """read gives a Text."""
  _, _, root = born()
  got = engine.read("a.txt", on=root)
  assert isinstance(got, Text) and got == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("a.txt", span(1, 1), on=root) == Text("/w/a.txt", "one\ntwo\n")


async def test_a_text_without_a_show_is_told_as_head() -> None:
  """A text without a show is told as HEAD, which is the span of its first 2000 lines."""
  assert HEAD(BIG.splitlines()) == span(1, 2000)(BIG.splitlines()) == list(range(1, 2001))
  _, _, root = born("read('big.txt')\nclose(1)", files={"/w/big.txt": BIG})
  assert await engine.prompt(int, "read it", on=root) == 1
  told = ["#read1", "read1_path = 'big.txt'", "<s:read1_text>", *[f"line {i}" for i in range(1, 2001)]]
  assert of(engine.turns(on=root), "read") == ["\n".join(told) + "</s:read1_text>"]


async def test_a_read_of_the_name_of_a_prompt_gives_the_program_of_that_ladder() -> None:
  """A read of the name of a prompt gives the program of that ladder, the words of its rungs in order."""
  _, _, root = born("k = BAD", "a = 1", "close(a + 1)", "close(None)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  assert engine.read(act, on=root) == Text(act, "k = BAD\na = 1\nclose(a + 1)")


async def test_whether_a_name_is_one_of_the_prompts_a_chain_has_heard_on_itself() -> None:
  """Whether a name is one of the prompts a chain has heard on itself, which is what its doors serve and no other path, a path of no name being none of them."""
  _, log, root = born("a = 1", "close(a + 1)", "close(None)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  assert engine.read(act, on=root) == Text(act, "a = 1\nclose(a + 1)")
  other = engine.chain("other")
  await settle()
  for path, on in [
    (act, other),
    *[(one, root) for one in ("", "nowhere://x", root, *[a[1] for a in said(log, "rung")])],
  ]:
    with pytest.raises(Refused, match="nothing takes read"):
      engine.read(path, on=on)


async def test_a_prompt_is_the_door_of_the_program_of_its_ladder() -> None:
  """A prompt is the door of the program of its ladder, so a read of its name gives the word of every rung of it in order, the words the gate refused among them, which are none at all for a prompt that ran no word."""
  _, _, root = born("x = BAD", "x = bash('echo hi')\nclose(1)", "close(None)")
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  assert engine.read(act, on=root) == Text(act, "x = BAD\nx = bash('echo hi')\nclose(1)")
  bare = engine.prompt(int, "later", to=OPERATOR, on=root)
  assert engine.read(bare, on=root) == Text(bare, "")


async def test_a_read_of_a_door_that_a_rung_of_that_ladder_says_leaves_the_word_of_that_rung_out() -> None:
  """A read of a door that a rung of that ladder says leaves the word of that rung out, since a rung is no part of the program it reads."""
  _, _, root = born("a = 1", "close(read(get(acting())[2]).content)", "close(None)")
  act = engine.prompt(str, "read it", on=root)
  assert await act == "a = 1"
  await settle()
  assert of(engine.turns(on=root), "read") == [f"#read1\nread1_path = {act!r}\nread1_text = 'a = 1'"]
  assert engine.read(act, on=root) == Text(act, "a = 1\nclose(read(get(acting())[2]).content)")


async def test_one_made_from_inside_an_act_tells_itself() -> None:
  """One made from inside an act tells itself under its own name, with its path and what it was answered, on the scope of that act; one made from outside an act tells nothing, and neither does one whose show is hidden."""
  _, log, root = born("read('a.txt')\nread('a.txt', HIDDEN)\nclose(1)")
  assert await engine.prompt(int, "read it", on=root) == 1
  step = said(log, "rung")[0][1]
  told = [a for a in said(log, "tell") if a[3][0].startswith("#read")]
  paragraph = "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>"
  assert [(a[1], a[2], a[3]) for a in told] == [(step, step, [paragraph])]
  held = engine.transcript(root)
  assert engine.scope(step) == root and told[0] in held
  was = engine.turns(on=root)
  assert of(was, "read") == [paragraph]
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("a.txt", HIDDEN, on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert paragraphs(engine.turns(on=root)) == paragraphs(was)


async def test_a_read_answered_with_what_is_no_text_gives_that_value() -> None:
  """A read answered with what is no text gives that value, and tells it as python shows it."""
  _, _, root = born(NUMS + "close(read('nums://a'))\n")
  assert await engine.prompt(list, "a door of my own", on=root) == [1, 2]
  assert of(engine.turns(on=root), "read") == ["#read1\nread1_path = 'nums://a'\nread1_value = [1, 2]"]


async def test_a_read_from_inside_an_act_whose_show_picks_more_than_largest_bytes_of_its_text_raises_refused() -> None:
  """A read from inside an act whose show picks more than LARGEST bytes of its text raises Refused, which names those bytes and LARGEST, and tells nothing, and a read from outside an act, which tells nothing, is never refused for its size."""
  long = "".join(f"{i:07}" + "x" * 1000 + "\n" for i in range(600))
  word = "try:\n  read('long.txt')\nexcept Refused as no:\n  why = str(no)\nclose([why, read('long.txt', span(2, 2)).lines[1][:7]])"
  _, _, root = born(word, files={"/w/long.txt": long})
  size = len("\n".join(long.splitlines()).encode())
  assert size > LARGEST
  assert await engine.prompt(list, "read it", on=root) == [f"long.txt shows {size} bytes, over {LARGEST}", "0000001"]
  assert [one.split("\n")[0] for one in of(engine.turns(on=root), "read")] == ["#read2"]
  assert engine.read("long.txt", HIDDEN, on=root).content == long
  assert engine.read("long.txt", on=root).content == long
  assert [one.split("\n")[0] for one in of(engine.turns(on=root), "read")] == ["#read2"]


async def test_the_world_serves_the_scratch_directory_as_scratch(tmp_path: Path) -> None:
  """The World serves the scratch directory as scratch://: a read of scratch://plan.md gives that file, and a read of scratch:// or of a folder of it gives the name of each entry of that folder, one on each line, each folder with / at its end, as ls does."""
  live, root = machine(tmp_path)
  engine.write(Text("scratch://notes/a.md", "a\n"), on=root)
  engine.write(Text("scratch://plan.md", "plan\n"), on=root)
  got = [engine.read(path, on=root) for path in ("scratch://plan.md", "scratch://", "scratch://notes")]
  live.end()
  assert got == [
    Text("scratch://plan.md", "plan\n"),
    Text("scratch://", "notes/\nplan.md"),
    Text("scratch://notes", "a.md"),
  ]


async def test_each_text_and_each_refusal_of_scratch_names_its_path_under_scratch(tmp_path: Path) -> None:
  """Each text and each refusal of scratch:// names its path under scratch:// and never the place of the scratch directory on the machine."""
  record = tmp_path / "record.jsonl"
  live, root = machine(tmp_path, record)
  place = _monty.scratch(str(record))
  assert place is not None
  wrote = engine.write(Text("scratch://notes/../plan.md", "plan\n"), on=root)
  with pytest.raises(Refused) as none:
    engine.read("scratch://none.md", on=root)
  with pytest.raises(Refused) as out:
    engine.write(Text("scratch://../out.md", "out"), on=root)
  live.end()
  assert wrote == Text("scratch://plan.md", "plan\n")
  assert [str(none.value), str(out.value)] == [
    "There is no file at scratch://none.md.",
    "scratch://../out.md is outside the scratch directory.",
  ]
  assert Path(place, "plan.md").read_text() == "plan\n"


async def test_a_path_of_scratch_that_leaves_the_scratch_directory_is_refused(tmp_path: Path) -> None:
  """A path of scratch:// that leaves the scratch directory, by .. or by a link, is refused, and so is a path that starts with scratch: and not scratch://, with a refusal that names scratch://."""
  (tmp_path / "away.md").write_text("away\n")
  live, root = machine(tmp_path)
  await engine.bash(f'ln -s {shlex.quote(str(tmp_path))} "$TMPDIR/away"', on=root)
  refused = []
  for path in ("scratch://../out.md", "scratch://away/away.md", "scratch:/typo.md", "scratch:plan.md"):
    with pytest.raises(Refused) as one:
      engine.read(path, on=root)
    refused.append(str(one.value))
  with pytest.raises(Refused):
    engine.write(Text("scratch://away/new.md", "x"), on=root)
  live.end()
  assert refused == [
    "scratch://../out.md is outside the scratch directory.",
    "scratch://away/away.md is outside the scratch directory.",
    "scratch:/typo.md is no path of scratch://, which starts with two slashes.",
    "scratch:plan.md is no path of scratch://, which starts with two slashes.",
  ]
  assert not (tmp_path / "new.md").exists()
