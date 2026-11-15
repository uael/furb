"""The checkpointer of the engine in monty, with every name of checkpoint.pyi.

Each function runs in the sandbox, on the engine of the life that this process holds, which is the engine whose names
it is given: the sandbox holds those names, so the call carries only its other words.
"""

from collections.abc import Callable

from furb_monty.engine import held_engine


def seed(names: dict[str, object], data: object) -> object:  # noqa: ARG001
  """The entries that a life boots on from the data of a checkpoint, as the checkpointer in the sandbox gives them."""
  return held_engine().checkpointed("seed", [data])


def gathered(names: dict[str, object], define: Callable[[str], str], floor: dict[str, int]) -> object:  # noqa: ARG001
  """The data of a checkpoint of the life at its tip, as the checkpointer in the sandbox gathers it."""
  return held_engine().checkpointed("gathered", [define, floor])
