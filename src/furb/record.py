"""The record as boot is given it: what stands in it for a value that no later life makes again as it was."""

import re

ADDRESS = r" at 0x[0-9a-fA-F]+"
SHOWN = r"Repr\(.*\)"


class Recorded:
  """What stands, in the record that a later life is given, for a value that no later life makes again as it was.

  A callable went out of the life that made it as a mark, or as its repr, which names an address of that life. An
  address of an object is a function of the heap and not of the record, so a text that holds one stands for each
  text, or each value, that shows the same with no address, and a mark stands for each callable. A value that has no
  way back, as a coroutine or an iterator, went out as monty shows it to its host, `Repr('<coroutine object c>')`,
  which names no address, so that text stands for each value that shows the same with no address inside that form.
  The journal then finds the act that a word makes again equal to the act of the record. The rule is this module's and no engine's, since each build
  reads it the same: the opening of the sandbox and a host of the engine of this interpreter give boot what recorded
  makes of a record, whatever build made it."""

  def __init__(self, held: str | None = None) -> None:
    self.held = held

  def __eq__(self, other: object) -> bool:
    if isinstance(other, Recorded):
      return self.held == other.held
    if self.held is None:
      return callable(other)
    shown = re.sub(ADDRESS, "", other if isinstance(other, str) else repr(other))
    held = re.sub(ADDRESS, "", self.held)
    return shown == held or (not isinstance(other, str) and f"Repr({shown!r})" == held)

  def __repr__(self) -> str:
    return "<a callable of an earlier life>" if self.held is None else repr(self.held)

  # It equals each value that shows the same with no address, so no hash agrees with its equality.
  __hash__ = None  # type: ignore[assignment]


def recorded(engine: dict[str, object], entries: list[tuple]) -> list[tuple]:
  """The entries of a record as boot is given them: in the words of each act that a word made, each text that holds
  an address, and each text in the form by which monty shows a value with no way back, stands as a Recorded of it,
  also inside a text, a tuple, a list or a dict."""
  text, question = engine["Text"], engine["question"]
  assert isinstance(text, type)
  assert callable(question)

  def again(word: object) -> object:
    if isinstance(word, str) and (re.search(ADDRESS, word) or re.fullmatch(SHOWN, word, re.DOTALL)):
      return Recorded(word)
    if isinstance(word, text):
      return text(getattr(word, "path"), again(getattr(word, "content")))  # noqa: B009
    if isinstance(word, tuple | list):
      return type(word)(map(again, word))
    if isinstance(word, dict):
      return {key: again(one) for key, one in word.items()}
    return word

  return [
    ((*x[:4], *map(again, x[4:])), *at) if question(x) and re.fullmatch(r"[a-z_]+\d+", str(x[2])) else (x, *at)
    for (x, *at) in entries
  ]
