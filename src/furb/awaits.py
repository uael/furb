"""The await of an act from outside a run, on the engine of this interpreter.

A word awaits an act by handing it to the Kernel, which steps the word, and the engine says no more of an await. The
operator awaits from a task of its own loop, which nothing of the engine steps: the Act that this package gives waits
there for the done of the act, by a generator of the life that hears each fact until the act is over. The package
binds that Act in the place of the Act of the engine, as a word rebinds a name of the engine, so each verb gives it
and a word that awaits it runs the await of the engine. The door of monty holds an await of its own.
"""

from asyncio import get_running_loop
from collections.abc import Generator

from furb import python as engine


class Act[T = object](engine.Act[T]):
  """The name of an act, which a word awaits as the engine says and the operator awaits on the loop of the life."""

  def __await__(self) -> Generator[object, None, T]:
    if engine.acting():
      return (yield from super().__await__())
    if not engine.get(self):
      raise engine.Refused(self + " never settles")
    settled = get_running_loop().create_future()

    def pend() -> Generator[None, tuple]:
      while not engine.over(self):
        yield
      got = engine.peek(self)
      if isinstance(got, BaseException):
        settled.set_exception(got)
      else:
        settled.set_result(got)

    engine.drive(waiter := pend(), self + " waits " + str(id(waiter)))
    try:
      return (yield from settled)
    finally:
      waiter.close()


vars(engine)["Act"] = Act
