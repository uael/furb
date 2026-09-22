"""The module of the sandbox: what monty does that CPython does not, for the host that embeds it.

The stand-in beside it reads it, and nothing else of the repository does: the sandbox holds the module, and
this stub is what the type checker of the repository reads the stand-in against. It stands here, where the
checker reads the first-party names of the repository, so it needs no path of its own.
"""

def rebound[T](x: T, source: dict[str, object], target: dict[str, object], memo: dict[int, object] | None = None) -> T:
  """A deep copy of x in which a function or a class made under the globals dict source is made again under
  target, an instance of such a class is one of the class made again, and what the session has one of, a module,
  an object of the host, is shared; a value of the memo is shared as deepcopy shares it."""

def instance[T](cls: type[T], fields: dict[str, object]) -> T:
  """An instance of a class of the session holding these attributes, made as the interpreter makes one, with no
  __init__ run."""
