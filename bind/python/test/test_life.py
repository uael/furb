"""One life of the real engine, driven from python through the module the crate makes.

Every test here holds the whole boundary at once: a World of python hears the facts, a Voice says into the life
what the host finished later, and what crosses between them is the plain form and nothing of the engine.
"""

from pathlib import Path

import pytest
from furb_sand import Fact, Fault, Life, Refused, Shape, Voice

from yard import Strict, Yard, came, life


def test_a_life_opens_on_its_root_and_answers_what_the_root_stands_on(yard: Path) -> None:
  """A life opens on its root chain, and the World it opened on says where that chain stands."""
  held, world = life(yard)
  assert held.root == "chain://operator.1"
  assert held.word(f"cwd(on={held.root!r})") == str(yard)
  assert len(held.word(f"turns(on={held.root!r})")) == 1
  assert held.world is world


def test_a_world_serves_a_read_and_a_write(yard: Path) -> None:
  """A World answers a read and a write, and asks the chain where its paths resolve before it touches the disk."""
  held, _ = life(yard)
  got = held.word(f"write(Text('a.txt', 'one\\ntwo\\n'), on={held.root!r})")
  assert got.content == "one\ntwo\n"
  assert Path(yard, "a.txt").read_text() == "one\ntwo\n"

  got = held.word(f"read('a.txt', on={held.root!r})")
  assert got.path == str(Path(yard, "a.txt"))
  assert got.content.splitlines() == ["one", "two"]


def test_a_read_the_world_refuses_raises_in_the_caller(yard: Path) -> None:
  """A read that the World refuses raises Refused in the caller."""
  held, _ = life(yard)
  with pytest.raises(Refused) as caught:
    held.word(f"read('none.txt', on={held.root!r})")
  assert "no file at" in str(caught.value)


def test_a_model_answers_a_prompt_and_the_kernel_runs_the_word_it_wrote(yard: Path) -> None:
  """A model answers a prompt of the engine, and the Kernel runs the word it wrote in the module of its chain."""
  held, world = life(yard, ("close(len(read('a.txt').lines))",))
  held.word(f"write(Text('a.txt', 'one\\ntwo\\nthree\\n'), on={held.root!r})")
  act = held.word(f"prompt(int, 'count the lines', on={held.root!r})")
  assert came(held, act) == 3
  assert "count the lines" in world.read[0]


def test_the_gate_refuses_a_word_and_the_engine_asks_the_model_again(yard: Path) -> None:
  """The gate of the host refuses a word, and the engine asks the model again with what the gate found."""
  held, world = life(yard, ("close(BAD)", "close(7)"), gate=Strict())
  act = held.word(f"prompt(int, 'count', on={held.root!r})")
  assert came(held, act) == 7
  assert "BAD in rung" in world.read[1]


def test_the_operator_answers_a_prompt_by_closing_it(yard: Path) -> None:
  """A prompt to the operator asks no model: the World is shown it, and it waits to be closed."""
  held, _ = life(yard)
  act = held.word(f"prompt(str, 'say a word', to='operator', on={held.root!r})")
  assert came(held, act) == "the operator says so"


def test_what_the_world_keeps_opens_a_second_life(yard: Path) -> None:
  """A record a World kept opens a second life, which makes the same acts again."""
  held, world = life(yard, ("close(1)",))
  act = held.word(f"prompt(int, 'count', on={held.root!r})")
  assert came(held, act) == 1

  lines = "\n".join(world.kept)
  again, _ = life(yard, record=lines)
  assert again.root == held.root


def test_a_value_a_life_holds_none_of_is_refused(yard: Path) -> None:
  """A host that hands over a value the engine cannot hold is told what it handed over."""

  class Odd(Yard):
    def hears(self, fact: Fact) -> object:
      if fact.kind == "clock":
        return Fact("done", fact.about, "", object())
      return super().hears(fact)

  voice = Voice()
  held = Life(Odd(yard, voice), voice=voice)
  with pytest.raises(TypeError) as caught:
    held.word(f"clock(on={held.root!r})")
  assert "a life holds no object" in str(caught.value)


def test_a_voice_is_heard_by_one_life(yard: Path) -> None:
  """A Voice is heard by the life that holds its Ears, and by no other."""
  voice = Voice()
  world = Yard(yard, voice)
  Life(world, voice=voice)
  with pytest.raises(RuntimeError) as caught:
    Life(Yard(yard, voice), voice=voice)
  assert str(caught.value) == "a Voice is heard by one life"


def test_a_fault_of_the_world_stands(yard: Path) -> None:
  """A World that raises has said something a life cannot answer, and the fault reaches the caller whole."""

  class Broken(Yard):
    def hears(self, fact: Fact) -> object:
      if fact.kind == "clock":
        raise ValueError("the clock of this World is broken")
      return super().hears(fact)

  voice = Voice()
  held = Life(Broken(yard, voice), voice=voice)
  with pytest.raises(ValueError) as caught:
    held.word(f"clock(on={held.root!r})")
  assert str(caught.value) == "the clock of this World is broken"


def test_every_plain_value_crosses_both_ways(yard: Path) -> None:
  """Every value of the plain form crosses to a host and back: the python ones, a shape, a fault and a show."""
  held, _ = life(yard)
  assert held.word("None") is None
  assert held.word("True") is True
  assert held.word("3") == 3
  assert held.word("1.5") == 1.5
  assert held.word("'one'") == "one"
  assert held.word("[1, 'two']") == [1, "two"]
  assert held.word("(1, 'two')") == (1, "two")
  assert held.word("{'a': 1}") == {"a": 1}
  assert held.word("Text('a.txt', 'one')") == Shape("Text", {"path": "a.txt", "content": "one", "before": None})
  assert held.word("Refused('no')") == Fault("Refused", "no")
