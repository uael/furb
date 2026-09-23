"""shown, the lines of a text the model has not seen."""

from furb import engine
from furb.engine import HEAD, Text, span

THREE = Text("/w/n.txt", "one\ntwo\nthree\n")
"""A text of three lines."""


def test_the_lines_of_the_text_the_model_has_not_seen_and_how_many_of_the_rest_it_knows() -> None:
  """The lines of the text the model has not seen, and how many of the rest it knows."""
  seen: dict[str, dict[int, str]] = {}
  assert engine.shown((THREE, span(1, 2)), seen) == "# /w/n.txt, 0 known\n# 1 one\n# 2 two"
  assert seen == {"/w/n.txt": {1: "one", 2: "two"}}
  assert engine.shown((THREE, HEAD), seen) == "# /w/n.txt, 2 known\n# 3 three"
  assert seen == {"/w/n.txt": {1: "one", 2: "two", 3: "three"}}
  assert engine.shown((THREE, HEAD), seen) == "# /w/n.txt, 3 known"
  changed = Text("/w/n.txt", "one\nTWO\nthree\n")
  assert engine.shown((changed, HEAD), seen) == "# /w/n.txt, 2 known\n# 2 TWO"
  assert seen == {"/w/n.txt": {1: "one", 2: "TWO", 3: "three"}}
  assert engine.shown((Text("/w/m.txt", "one\n"), HEAD), seen) == "# /w/m.txt, 0 known\n# 1 one"
