"""The boundary of a host that is not python: one generator of the outside, and the plain form of every value.

The engine takes the World and the Kernel as generators, each under the name it hears by. A host written in another
language is no generator, so this stands in its place: it hears every fact, makes it plain, hands it to the host,
and says back what the host answers. Nothing of the engine is bound here, and nothing here is bound in the engine:
the crate runs this in a module of its own and hands the two generators to boot, so the globals of a chain hold
what the file defines and nothing more.

A host that must ask the engine something while it answers says so and is asked, so it never calls into a life
that stands waiting for it.
"""

from collections.abc import Callable, Generator, Mapping

type Host = Callable[[str, object], object]
"""A host hears one fact, plain, under the name of the generator that carries it, and answers what to do."""
type Names = Mapping[str, Callable[..., object]]
"""The names of the engine, by which a plain value is made again and by which the boundary asks."""


TUPLE = "()"
"""TUPLE is how the plain form says a tuple, since a host that says a fact back must say it as the fact it was."""
SHOW = ""
"""SHOW is how the plain form says what no host reads: a show or a filter, which no record holds either."""


def wire(x: object) -> object:
  """The plain form of a value: an exception its name and what it was made with, a shape its name beside its
  fields, a tuple its entries under its own mark, a list its entries, a map its entries, and plain data is plain.

  What is none of these is a show or a filter, which no host reads and no record holds, and it crosses as a mark
  of what it was and nothing else.
  """
  match x:
    case None | bool() | int() | float() | str():
      return x
    case BaseException():
      return {"is": type(x).__name__, "args": [wire(i) for i in x.args]}
    case dict():
      return {k: wire(v) for k, v in x.items()}
    case tuple():
      return {"is": TUPLE, "args": [wire(i) for i in x]}
    case list():
      return [wire(i) for i in x]
  fields = getattr(x, "__dataclass_fields__", None)
  return (
    {"is": SHOW, "of": type(x).__name__}
    if fields is None
    else {"is": type(x).__name__} | {name: wire(getattr(x, name)) for name in fields}
  )


def unwire(x: object, names: Names) -> object:
  """The value again from the plain form, made by what its name is known by, and nothing for what cannot be made."""
  match x:
    case list():
      return [unwire(i, names) for i in x]
    case {"is": str(mark), "args": list(held)} if mark == TUPLE:
      return tuple(unwire(i, names) for i in held)
    case {"is": str(mark)} if mark == SHOW:
      return None
    case {"is": str(name), **rest}:
      held = rest.pop("args", [])
      args = [unwire(i, names) for i in held] if isinstance(held, list) else []
      return names[name](*args, **{str(k): unwire(v, names) for k, v in rest.items()})
    case dict():
      return {k: unwire(v, names) for k, v in x.items()}
  return x


def outside(name: str, host: Host, names: Names) -> Generator[tuple | None, tuple]:
  """One generator of the outside: every fact it hears goes to the host, and what the host says it says.

  The host answers one of three ways: it says facts, which this yields one by one, as any generator of a life does;
  it asks a question of the engine, which this puts and hands back the answer; or it says nothing at all.
  """
  while True:
    a = yield
    if a is None:
      continue
    reply = host(name, wire(a))
    while True:
      match reply:
        case ("ask", str(kind), str(on), list(words)):
          got = names["ask"](kind, on, *[unwire(w, names) for w in words])
          reply = host(name, wire(("answered", got[1] if isinstance(got, tuple) else got)))
        case ("say", list(facts)):
          for one in facts:
            said = unwire(one, names)
            if isinstance(said, (list, tuple)):
              yield tuple(said)
          break
        case _:
          break
