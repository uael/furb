"""prcheck refuses a sentence of more than 25 words and a title that breaks its rule, and a why or an outcome line that ends with no full stop, and says how far each one is."""

import re
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent.parent
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)


class Refused(Exception):
  pass


ns: dict = {"Refused": Refused, "span": None, "HIDDEN": None}
exec(compile(IMPORT.sub("", (HERE / "pr.py").read_text(encoding="utf-8")), "pr.py", "exec"), ns)  # noqa: S102
prcheck = ns["prcheck"]
TITLE = "fix(pr): Check a sentence"
LONG = " ".join(["word"] * 26)


def test_a_sentence_of_25_words_passes_and_the_count_of_words_is_given() -> None:
  assert prcheck(TITLE, " ".join(["word"] * 25) + ".", ["X."]) == 26


def test_a_sentence_of_26_words_in_the_why_is_refused_with_its_length_and_start() -> None:
  with pytest.raises(Refused, match=r"1 sentences over 25 words.*\n26 words: word word word word word word word word \.\.\."):
    prcheck(TITLE, LONG + ". Short one.", ["X."])


def test_a_long_sentence_in_an_outcome_or_a_breaking_change_is_refused_too() -> None:
  for outcome, breaking in ((["X.", LONG], None), (["X."], [LONG])):
    with pytest.raises(Refused, match="1 sentences over 25 words"):
      prcheck(TITLE, "Why.", outcome, breaking)


def test_two_short_sentences_that_hold_26_words_together_pass() -> None:
  assert prcheck(TITLE, " ".join(["word"] * 13) + ". " + " ".join(["word"] * 13) + ".", ["X."]) == 27


def test_a_refused_title_gives_its_length_and_the_types() -> None:
  with pytest.raises(Refused, match=r"holds 12 characters.*feat, fix, perf or chore"):
    prcheck("fix: no area", "Why.", ["X."])


def test_a_why_that_is_a_command_and_an_outcome_line_with_a_pipe_are_refused() -> None:
  old = "git status --short\nuv run pytest -q 2>&1 | grep -E passed | head"
  with pytest.raises(Refused, match=r"1 lines hold no sentence.*git status --short"):
    prcheck(TITLE, old, ["X."])
  with pytest.raises(Refused, match=r"1 lines hold no sentence.*pytest -q 2>&1 \| grep"):
    prcheck(TITLE, "Why.", ["uv run pytest -q 2>&1 | grep passed"])
