"""tell, what a question answered now tells of itself."""

from conftest import born, named, paragraphs, said, settle
from furb import engine
from furb.engine import OPERATOR, Text


async def test_what_a_question_answered_now_that_shows_a_text_or_changes_a_state_tells_of_itself() -> None:
  """What a question answered now that shows a text or changes a state tells of itself: a paragraph headed with the name of the question, which binds its path and what it was answered, said on the run that asked it, and nothing at all outside a run; one that only reads a value tells nothing, since the word that asked it holds the value, which it debugs to see."""
  sand, log, root = born()
  word = (
    "read('a.txt')\ncd('/x')\nx = bash('echo hi')\nseen = [peek(x), turns(), clock(), chance(), gate('k = 1'), cwd()]\n"
  )
  sand.script[root] = [word + "debug(t'{seen[2]}')\nclose(1)"]
  assert await engine.prompt(int, "ask things", on=root) == 1
  await settle()
  _, step, *_ = said(log, "rung")[0]
  kinds = ["reply", "read", "cd", "bash", "clock", "chance", "gate"]
  assert [a[0] for a in engine.transcript(root) if engine.question(a) and a[2] == step] == kinds
  told = [a[3] for a in said(log, "tell") if a[1] == step and a[2] == step]
  assert told == [
    ["#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>"],
    ["#cd1\ncd1_path = '/x'"],
    [f"#{step} debugged seen[2]\n{step}_debug = ['1001.0']"],
  ]
  assert "#cd1\ncd1_path = '/x'\n\n#bash1\nbash1_command = 'echo hi'" in engine.turns(on=root)[-1][1]
  assert [one for one in named(engine.turns(on=root)) if one in ("clock", "chance", "gate")] == []
  seen = engine.module(root)["seen"]
  assert isinstance(seen, list) and seen[2:] == [1001.0, 2 / 7, [], "/x"]
  was = engine.turns(on=root)
  assert engine.cd("/w", on=root) == "/w"
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.clock(on=root) == 1003.0
  assert engine.turns(on=root) == was


async def test_a_question_is_put_to_the_ears_of_the_engine_before_those_of_the_outside() -> None:
  """A question is put to the ears of the engine before those of the outside, so the World is asked for nothing that the engine knows."""
  sand, _, root = born()
  assert engine.cwd(on=root) == "/w"
  assert engine.turns(on=root) != []
  act = engine.prompt(int, "count", to=OPERATOR, on=root)
  assert engine.read(act, on=root) == Text(act, "")
  assert [a[0] for a in sand.calls] == ["stand", "prompt"]
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "prompt", "read"]


async def test_the_question_that_tell_names_is_the_last_one_of_its_kind_on_its_chain_that_the_run_made() -> None:
  """The question that tell names is the last one of its kind on its chain that the run made."""
  _, log, root = born("read('a.txt', HIDDEN)\nread('a.txt', span(1, 1))\nclose(1)")
  engine.read("a.txt", on=root)
  assert await engine.prompt(int, "read twice", on=root) == 1
  step = said(log, "reply")[0][2]
  assert [(a[1], a[2]) for a in said(log, "read")] == [("read1", OPERATOR), ("read2", step), ("read3", step)]
  assert [a[3] for a in said(log, "tell") if a[1] == step and a[2] == step] == [
    ["#read3\nread3_path = 'a.txt'\nread3_text = 'one'"]
  ]


async def test_a_text_is_told_by_the_lines_that_its_show_picks_under_the_word_text() -> None:
  """A text is told by the lines that its show picks, under the word text, and any other answer under the word value."""
  word = (
    "def kept(id):\n"
    "  while True:\n"
    "    match (yield):\n"
    "      case ('read', qid, _, _, path) if path.startswith('nums://'):\n"
    "        yield 'done', qid, [1, 2]\n"
    "\n"
    "act('nums', '', kept)\n"
    "read('nums://a')\n"
    "read('a.txt', span(2, 2))\n"
    "close(1)"
  )
  _, _, root = born(word)
  assert await engine.prompt(int, "a door of my own", on=root) == 1
  await settle()
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#read")] == [
    "#read1\nread1_path = 'nums://a'\nread1_value = [1, 2]",
    "#read2\nread2_path = 'a.txt'\nread2_text = 'two'",
  ]
