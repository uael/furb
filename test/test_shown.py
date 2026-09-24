"""shown, the lines of a showing the model has not seen."""

from furb import engine

THREE = "one\ntwo\nthree\n"
"""A content of three lines."""


def every(lines: list[str]) -> list[int]:
  """The show of every line."""
  return list(range(1, len(lines) + 1))


def test_the_lines_of_a_showing_the_model_has_not_seen_and_how_many_of_the_rest_it_knows() -> None:
  """The lines of a showing the model has not seen, and how many of the rest it knows."""
  seen: dict[str, dict[int, str]] = {}
  assert engine.shown(("/w/n.txt", THREE, lambda lines: [1, 2]), seen) == "# /w/n.txt, 0 known\n# 1 one\n# 2 two"
  assert seen == {"/w/n.txt": {1: "one", 2: "two"}}
  assert engine.shown(("/w/n.txt", THREE, every), seen) == "# /w/n.txt, 2 known\n# 3 three"
  assert seen == {"/w/n.txt": {1: "one", 2: "two", 3: "three"}}
  assert engine.shown(("/w/n.txt", THREE, every), seen) == "# /w/n.txt, 3 known"
  assert engine.shown(("/w/n.txt", "one\nTWO\nthree\n", every), seen) == "# /w/n.txt, 2 known\n# 2 TWO"
  assert seen == {"/w/n.txt": {1: "one", 2: "TWO", 3: "three"}}
  assert engine.shown(("/w/m.txt", "one\n", every), seen) == "# /w/m.txt, 0 known\n# 1 one"
  assert engine.shown("# as it is", seen) == "# as it is"
