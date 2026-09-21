"""furb, with its engine picked once at import.

`from furb import engine` gives the engine of this interpreter, `furb.engine`, unless `FURB_ENGINE` is `monty`, in
which case it gives `furb_monty.engine`: the same surface, over the engine running in the sandbox of monty with
the Kernel of the crate. `furb.python` is the engine of this interpreter whatever the switch says, since the
system prompt a model reads is that file and nothing else.
"""

import importlib
import os
import sys

python = globals().get("python") or importlib.import_module("furb.engine")
"""python is the engine of this interpreter, kept across a reload of the package, since a reload rebinds the switch."""
MONTY = os.environ.get("FURB_ENGINE") == "monty"
"""MONTY says whether the engine of this process runs in the sandbox of monty."""

# The engine of this process is bound under the name of the submodule, so that `from furb import engine` and
# `from furb.engine import ...` both give it, and nothing but this switch says which one it is.
sys.modules["furb.engine"] = importlib.import_module("furb_monty.engine") if MONTY else python
globals()["engine"] = sys.modules["furb.engine"]
