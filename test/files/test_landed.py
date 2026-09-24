"""landed, what a read or a write gives of what it was answered."""

from conftest import FILES, STANDS, Sand, World, life, texted, verb
from furb import engine


class Old(Sand):
  """A World that answers a read with the mark a record of 0.1.0 holds a text by."""

  def hears(self) -> World:
    """The World that answers a standing, and a read with a text as the mark of its class and its fields."""
    while True:
      match (yield):
        case ("stand", qid, *_):
          yield "done", qid, self.stands or [[], "", ""]
        case ("read", qid, _, _, path):
          yield "done", qid, {"is": "Text", "path": path, "content": "old\n", "before": None}


async def test_what_a_read_or_a_write_gives_of_what_it_was_answered() -> None:
  """What a read or a write gives of what it was answered: a Text of the path and the content that plain data holds, and anything else as it came."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  word = (
    "t = landed({'path': 'a', 'content': 'b'})\n"
    "close([type(t) is Text, t.path, t.content, landed([1, 2]), landed(None), landed({'path': 'a'}), "
    "landed(t) is t])"
  )
  assert await engine.rung(word, on=root) == [True, "a", "b", [1, 2], None, {"path": "a"}, True]
  old = Old(stands=STANDS, words=FILES)
  _, root = life(old)
  assert texted(verb("read", root)("a.txt")) == ("a.txt", "old\n")
