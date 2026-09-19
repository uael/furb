"""shown, the lines of a text the model has not seen."""

from furb import engine
from furb.engine import HEAD, Text, span

THREE = Text("/w/n.txt", "one\ntwo\nthree\n")


def test_the_lines_of_the_text_the_model_has_not_seen_and_how_many_of_the_rest_it_knows() -> None:
  """The lines of the text the model has not seen, and how many of the rest it knows."""
  seen: dict[str, dict[int, str]] = {}
  assert engine.shown((THREE, span(1, 2)), seen) == ("shown", [("path", "/w/n.txt"), ("known", 0)], "1 one\n2 two")
  assert seen == {"/w/n.txt": {1: "one", 2: "two"}}
  assert engine.shown((THREE, HEAD), seen) == ("shown", [("path", "/w/n.txt"), ("known", 2)], "3 three")
  assert seen == {"/w/n.txt": {1: "one", 2: "two", 3: "three"}}
  assert engine.shown((THREE, HEAD), seen) == ("shown", [("path", "/w/n.txt"), ("known", 3)], "")
  changed = Text("/w/n.txt", "one\nTWO\nthree\n")
  assert engine.shown((changed, HEAD), seen) == ("shown", [("path", "/w/n.txt"), ("known", 2)], "2 TWO")
