"""choice, the model that chooses what a compaction keeps."""

import re
from pathlib import Path

from extensions.conftest import compacting
from furb import engine


def chooser(word: str) -> tuple[str, str, list[str]]:
  """The rung of a word and the message of the prompt of choice that it made, as lines."""
  ran = next(str(a[4]) for a in engine.transcript("chain1") if a[0] == "run" and a[5] == word)
  message = next(str(a[5]) for a in engine.transcript("chain2") if a[0] == "prompt")
  return ran, message, message.split("\n")


async def test_choice_asks_a_model_which_acts_of_the_chain_the_work_still_needs(tmp_path: Path) -> None:
  """choice() asks a model, on a chain of its own, which acts of the chain the work still needs, in a prompt of the shape tuple[list[str], str], the ids to keep and a summary of what the rest holds, and gives that prompt."""
  sand, root, _ = compacting(tmp_path)
  sand.script.update(
    {
      root: ["got = await choice()\nassert isinstance(got, tuple)\nclose([got[0], got[1]])"],
      "chain2": ["close((['x'], 'gist'))"],
    }
  )
  assert await engine.prompt(list, "choose", on=root) == [["x"], "gist"]
  assert engine.get("chain2")[3:] == (root, "chosen", "")
  assert [a[4] for a in engine.transcript("chain2") if a[0] == "prompt"] == ["tuple[list[str], str]"]


async def test_the_prompt_says_what_to_keep_then_lists_each_open_prompt_then_each_act(tmp_path: Path) -> None:
  """The prompt says what to keep, then lists each open prompt with its message, then each act of the chain by its id and its words, with the thousands of characters of its text or its output when they pass a thousand, each act that a step made under that step, and no reply, run, wants, gate or stand."""
  sand, root, work = compacting(tmp_path)
  word = "read('a.txt')\nread('big.txt', HIDDEN)\nc = choice()\nclose(1)"
  sand.files[f"{work}/big.txt"] = "x" * 2500
  sand.script[root] = [word]
  act = engine.prompt(int, "list", on=root)
  assert await act == 1
  ran, message, lines = chooser(word)
  read = next(str(a[1]) for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt")
  big = next(str(a[1]) for a in engine.transcript(root) if a[0] == "read" and a[4] == "big.txt")
  assert f"  {big} 'big.txt' (2k chars)" in lines
  assert message.startswith(engine.module(root)["BRIEF"] + "\n\nOpen prompts:\n")
  assert (
    lines.index(f"{act} to its model: 'list'") < lines.index("Acts:") < lines.index(f"{act} 'int', 'list', '' (open)")
  )
  assert lines.index(
    f"{ran} step of {act} (open): read('a.txt') read('big.txt', HIDDEN) c = choice() close(1)"
  ) + 1 == lines.index(f"  {read} 'a.txt'")
  assert [one for one in lines if re.match(r"\s*(reply|run|wants|gate|stand)\d", one)] == []


async def test_it_lists_of_a_step_only_the_last_listed_acts_that_it_made(tmp_path: Path) -> None:
  """It lists of a step only the last LISTED acts that it made, and says how many it left out."""
  sand, root, _ = compacting(tmp_path)
  word = "LISTED = 1\nread('a.txt')\nread('a.txt')\nc = choice()\nclose(1)"
  sand.script[root] = [word]
  act = engine.prompt(int, "list", on=root)
  assert await act == 1
  ran, _, lines = chooser(word)
  reads = [str(a[1]) for a in engine.transcript(root) if a[0] == "read" and a[4] == "a.txt"]
  assert [f"  {one} 'a.txt'" in lines for one in reads] == [False, True]
  assert f"{ran} step of {act} (open): {' '.join(word.split())} (1 earlier acts of it unlisted)" in lines
