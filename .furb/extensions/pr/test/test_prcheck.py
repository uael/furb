"""prcheck checks a title with script/title.py and a body with the rules of the template, on the real engine."""

import pytest

from extensions.conftest import Bench
from furb.engine import Refused


async def test_prcheck_runs_title_py_of_the_worktree_at_where_on_the_title_and_refuses_a_title_that_it_refuses(pulling: Bench) -> None:
  """prcheck(title, why, outcome) runs script/title.py of the worktree at where on the title, and refuses a title that it refuses, with its words."""
  assert await pulling.value('await prcheck("chore(pr): Fill the template", "It was copied.", ["One home."])') == 5
  with pytest.raises(Refused, match="The title is wrong: 'docs: No area'"):
    await pulling.value('await prcheck("docs: No area", "It was copied.", ["One home."])')


async def test_prcheck_counts_why_and_outcome_alone(pulling: Bench) -> None:
  """It counts the words of why and outcome, which hold 300 at most, and gives that count."""
  long = " ".join(["Twelve words fill this sentence of the outcome, and it ends here."] * 25)
  short = " ".join(["Twelve words fill this sentence of the outcome, and it ends here."] * 24)
  assert await pulling.value(f'await prcheck("chore(pr)!: Break", "Why.", [{short!r}], breaking=[{long!r}])') == 289
  with pytest.raises(Refused, match="the body holds 301 words, over 300"):
    await pulling.value(f'await prcheck("chore(pr): Grow", "Why.", [{long!r}])')


async def test_prcheck_refuses_a_long_sentence(pulling: Bench) -> None:
  """It refuses a sentence of more than 25 words, with its length and its start."""
  ok, long = " ".join(["word"] * 24) + " end.", " ".join(["word"] * 25) + " end."
  assert await pulling.value(f'await prcheck("chore(pr): Count", {ok!r}, ["One."])') == 26
  with pytest.raises(Refused, match="1 sentences over 25 words, split each one:\n26 words: word word"):
    await pulling.value(f'await prcheck("chore(pr): Count", "Why.", ["One.", {long!r}])')


async def test_prcheck_refuses_a_line_that_is_a_command(pulling: Bench) -> None:
  """It refuses a line of why or outcome that ends with no full stop, as a command does."""
  with pytest.raises(Refused, match="2 lines hold no sentence, as a command does: end each with a full stop: Run the build; a \\| b"):
    await pulling.value('await prcheck("chore(pr): Count", "Run the build", ["a | b", "Fine."])')


async def test_prcheck_refuses_a_break_with_no_bang(pulling: Bench) -> None:
  """It refuses a breaking change when the title carries no ! after its area."""
  with pytest.raises(Refused, match="carries a ! after the area of its title"):
    await pulling.value('await prcheck("feat(pr): Break", "Why.", ["One."], breaking=["The door moves."])')
