"""The module of the sandbox: what monty does that CPython does not, for the host that embeds it.

The stand-in reads it, and nothing else of the repository does: the sandbox holds the module, and this stub
is what the type checker of the repository reads the stand-in against.
"""

def instance[T](cls: type[T], fields: dict[str, object]) -> T:
  """An instance of a class of the session holding these attributes, made as the interpreter makes one, with no
  __init__ run."""
