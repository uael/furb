"""filled, the share of the window that a reply read."""

from dataclasses import dataclass
from functools import partial

from conftest import STANDS, Sand, World, born, life, said, settle
from furb import engine


@dataclass
class Bare(Sand):
  """A World whose model answers each reply with a turn that holds no usage."""

  def hear(self, a: tuple) -> World:
    """The World of the suite, but for a reply, which it answers with close(1) and no usage."""
    match a:
      case ("reply", about, *_):
        yield "started", about
        self.loop.call_soon(partial(engine.say, "done", about, ("assistant", "close(1)", None, [])))
      case _:
        yield from super().hear(a)


async def test_the_share_of_the_window_that_a_reply_read_to_two_places() -> None:
  """The share of the window that a reply read, to two places: the tokens that it read over the window that the roster of the standing gives its actor, and over WINDOW when the roster holds no such actor, and nothing when the reply gave no turn that holds a usage."""
  sand, log, root = born("close(1)", cost=(100001, 0, 0, 0, 0.0))
  assert await engine.prompt(int, "count", on=root) == 1
  (reply,) = [a[1] for a in said(log, "reply")]
  assert engine.get(reply)[4] == "m/low" and engine.filled(reply) == 0.25
  sand.stands = [[[name, efforts, window] for name, efforts, window in STANDS[0] if name != "m"], *STANDS[1:]]
  engine.stand(on=root)
  assert engine.filled(reply) == 0.5 and engine.filled("reply9") is None
  log, root = life(Bare())
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  (reply,) = [a[1] for a in said(log, "reply")]
  assert engine.filled(reply) is None
