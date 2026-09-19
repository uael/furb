"""A sandbox of the tests: one python namespace behind a pipe, which is what a Session must be.

The crate says what a sandbox does and nothing of what it is: it runs python in one namespace that stands from
one call to the next, it binds one name to a call of the host, and it gives back the value of the last expression
of the code. This is one of those, made of the interpreter that runs it, so the tests of the crate drive the real
engine long before a sandbox of monty can.

It is no part of the crate and it sandboxes nothing: what runs in here runs with everything this interpreter can
reach. It is a fixture, and the tests are the only thing that ever opens it.

The pipe carries one json object to a line:

    in   {"run": "<code>"}      run this code
    in   {"said": <value>}      the answer to the call it last made
    out  {"call": [name, said]} the code called the host
    out  {"gave": <value>}      the code is over, and this is what its last expression gave
    out  {"raised": <value>}    the code raised, and this is the exception, plain
"""

import ast
import asyncio
import json
import os
import sys
import traceback

HELD: dict[str, object] = {}
"""HELD is the one namespace of the sandbox, which stands from one run to the next."""
LOUD = bool(os.environ.get("FURB_LOUD"))
"""LOUD says every call of the host to the stderr of the test, which is how a test of the crate is read."""


def plain(value: object) -> object:
  """One value as it crosses, which the preamble makes plain once the preamble is in the namespace."""
  wire = HELD.get("wire")
  return wire(value) if callable(wire) else value


def says(said: dict[str, object]) -> None:
  """One object out, and on the pipe before this gives back."""
  sys.stdout.write(json.dumps(said) + "\n")
  sys.stdout.flush()


def host(name: str, said: object) -> object:
  """The one name the sandbox reaches its host by: one call out, and the answer back."""
  says({"call": [name, said]})
  line = sys.stdin.readline()
  if not line:
    raise SystemExit(0)
  got = json.loads(line)["said"]
  if LOUD:
    sys.stderr.write(f"{name} heard {said!r}\n  and was answered {got!r}\n")
  return got


def ran(code: str) -> object:
  """The code, run in the one namespace, and what the last expression of it gave.

  A body that ends in an expression gives what that expression gave, as a session does; one that ends in
  anything else gives nothing.
  """
  tree = ast.parse(code)
  last = tree.body.pop() if tree.body and isinstance(tree.body[-1], ast.Expr) else None
  if tree.body:
    exec(compile(tree, "<sandbox>", "exec"), HELD)  # noqa: S102
  if last is None:
    return None
  return eval(compile(ast.Expression(last.value), "<sandbox>", "eval"), HELD)  # noqa: S307


async def living() -> None:
  """Every run the host asks for, until the host is gone.

  It lives on a running loop, since the engine settles an await of its acts from outside a run in the loop it is
  opened in, and boot outside a running loop raises before it makes anything. Nothing else of the loop is used:
  the run of a word is stepped by hand, and this reads its pipe where it stands.
  """
  HELD["host"] = host
  for line in sys.stdin:
    if not line.strip():
      continue
    try:
      says({"gave": plain(ran(json.loads(line)["run"]))})
    except BaseException as no:
      # The fault goes out as the exception the host reads, and its whole trace to the stderr of the test.
      traceback.print_exc(file=sys.stderr)
      says({"raised": {"is": type(no).__name__, "args": [plain(one) for one in no.args]}})


asyncio.run(living())
