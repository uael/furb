"""furb for python, with the engine in monty.

`furb_monty.engine` gives every name of the contract over one life of the engine in the sandbox, and `gate` is the
gate of the crate: what the type checker of monty found on a sheet, each error by its line, which the Kernel of
this interpreter reads a word with too, so a word is judged once and the same. The extensions are the crate's too:
`extensions` gives the extensions a host takes for a project, from the configs and the cache of `places`,
`builtin_extensions` the builtins, `word_of` the word of the python part of an extension, `engine_source` the engine
a life runs with the words of its extensions after it, and `system_prompt` the system prompt of a life, which every
host makes the same.
"""

from furb_monty._monty import (
  Extension,
  builtin_extensions,
  engine_source,
  extensions,
  gate,
  places,
  system_prompt,
  word_of,
)

__all__ = [
  "Extension",
  "builtin_extensions",
  "engine_source",
  "extensions",
  "gate",
  "places",
  "system_prompt",
  "word_of",
]
