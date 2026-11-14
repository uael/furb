"""The checkpointer of the engine in monty, with every name of checkpoint.pyi.

Each function runs in the sandbox, on the engine of the life that this process holds, which is the engine whose names
it is given: the sandbox holds those names, so the call carries only its other words.
"""

from collections.abc import Callable, Iterable, Iterator, Sequence

from furb_monty.engine import held_engine


def journal(names: dict[str, object], record: Iterable[Sequence[object]] = ()) -> Callable[..., Iterator[object]]:  # noqa: ARG001
  """What boot of the door takes as the journal of a life on a record: a function that gives each fact of that
  record, which boot hands to the sandbox, since the sandbox makes the journal itself, where its engine runs."""
  held = list(record)

  def facts(*tables: object) -> Iterator[object]:  # noqa: ARG001
    for (fact,) in held:
      yield fact

  return facts


def fed(names: dict[str, object], entries: Iterable[Sequence[object]]) -> None:  # noqa: ARG001
  """The entries that follow the tip of the life, given to the journal in the sandbox, which says them again."""
  held_engine().feed(list(entries))


def seed(names: dict[str, object], data: object) -> object:  # noqa: ARG001
  """The entries that a life boots on from the data of a checkpoint, as the checkpointer in the sandbox gives them."""
  return held_engine().checkpointed("seed", [data])


def gathered(names: dict[str, object], define: Callable[[str], str], floor: dict[str, int]) -> object:  # noqa: ARG001
  """The data of a checkpoint of the life at its tip, as the checkpointer in the sandbox gathers it."""
  return held_engine().checkpointed("gathered", [define, floor])


def held(names: dict[str, object]) -> object:  # noqa: ARG001
  """The acts that the journal in the sandbox holds now, each that an ear of the outside took and that is not done."""
  return held_engine().checkpointed("held", [])
