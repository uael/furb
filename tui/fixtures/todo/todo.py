from collections.abc import Iterator

from furb.engine import Act, act


def todo(item: str, *, done: bool = False, on: str = "") -> Act:
  def ear(name: str) -> Iterator[tuple]:
    yield "done", name, None

  return act("todo", on, ear, item, done)
