from dataclasses import dataclass
from typing import Final, Literal

from furb.engine import Note, Show

def span(lo: int, hi: int) -> Show:
  """span(lo, hi) is the show of the lines lo through hi, where a line under one is counted back from the end, so that span(1, 20) is the first twenty lines and span(-20, -1) is the last twenty.
  A span that holds no line shows none, which HIDDEN is, so what a hidden show shows stands in no turns: an act that takes one tells its header and its binding alone, and a query that takes one tells nothing.
  span, grep and differs make the shows of the file.
  """

def grep(pattern: str) -> Show:
  """grep(pattern) is the show of the lines that the pattern matches, each with its number."""

def differs(old: list[str]) -> Show:
  """differs(lines) is the show of the lines that differ from the lines it holds, which is what a write shows of what came back."""

HEAD: Final[Show] = span(1, 2000)
"""HEAD is the span of the first 2000 lines, which a read without a show is told as."""
HIDDEN: Final[Show] = span(0, 0)
"""HIDDEN is the span of no line, which an act takes to tell nothing of itself but its header and its binding."""

def door(path: str, on: str = "") -> bool:
  """Whether a path is the door of a ladder of its chain: the name of a prompt that the chain it is on has heard on itself, which that chain answers a ladder of.
  A path of no name, and the name of a prompt of another chain, is no door of the chain.
  """

def landed(got: object) -> object:
  """What a read or a write gives of what it was answered: a Text of the path and the content that plain data holds, and anything else as it came."""

def showing(got: object, show: Show) -> list[Note]:
  """What a paragraph shows of what a door answered: the text by the lines the model has not seen, and anything that is no text as a comment of how python shows it."""

def read(path: str, show: Show = HEAD, on: str = "") -> Text:
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines the model has not seen.
  A read that the World refuses raises Refused in the caller.
  A read on a chain with a source tells the lines of a skipped read again, since they are not known there.
  read judges no scheme, so a path of an unknown scheme goes to the World too.
  read is given a path and a show.
  read gives a Text.
  A text without a show is told as HEAD, which is the span of its first 2000 lines.
  A read of a door of a ladder asks its chain for the ladder, and gives the program of that ladder as a text of that path, so the World is asked nothing.
  One asked from inside an act tells itself, with its path and what it was answered, on the scope of that act; one asked from outside an act tells nothing, and neither does one whose show is hidden.
  A read answered with what is no text gives that value, and tells it as python shows it.
  A read answered with plain data gives the Text of its path and its content.
  read tells a line again after the content of the line changed.
  A second read of a text tells the model no line that an earlier read of the chain told.
  """

def write(text: Text, on: str = "") -> Text:
  """A write: whoever serves the path of the text takes its content.
  write is given a text, and gives the text as it is on disk after the write.
  A write that the World refuses raises Refused in the caller.
  write tells of a write of a text only the lines that differ from what the caller asked, and of a write a door answers with a value, that value.
  A write takes no show, since what a write would show the word of the model already said: it tells the lines of what came back that differ from what it asked for, and of those, the lines the model has not seen, so a write that the disk took as it was asked tells nothing at all.
  A door that answers a write with more than it was asked for tells the lines it added and no line the model read before.
  A write to a door of a ladder gives the ladder its content as a word, so the chain makes the rungs of that ladder again from it, and the write gives the text it was given and tells nothing.
  A write asks with the path and the content of its text, and with no Text, so the record holds plain data.
  A new file is a write of a Text made of its path and its content.
  """

def cd(path: str, on: str = "") -> str:
  """A cd: the paths of its chain resolve against its path from then on, and it does nothing else.
  cd completes at once and gives the new working directory.
  It gives the path it was given, and the chain holds the query it asks, so what a chain heard is where its working directory stands.
  It is a query and no fact, since a fact a running word says is heard when the word yields, where a query is heard at once, so the paths of that word resolve against the new directory from then on.
  cd tells the path it was given.
  """

def cwd(on: str = "") -> str:
  """The working directory of a chain is the closest cd back in its transcript.
  The working directory of a chain is the directory of the standing it stands on while no cd stands in its transcript, so a later standing moves no chain that a cd moved.
  The World resolves the path of a read and a write against the working directory that cwd gives on the chain of the question.
  cwd gives the working directory that the paths of the chain resolve against.
  cwd reads where the paths of a chain resolve off the transcript that the chain answers, which is the closest cd back in what it heard.
  A chain has a working directory of its own.
  """

@dataclass
class Text:
  """A text: its path, what stands at it, of which its lines are the lines, and the text it came from.
  The lines of a text derive from its content.
  Every edit of it gives another text, which came from this one.
  """

  path: str
  content: str = ""
  before: Text | None = None
  @property
  def lines(self) -> list[str]: ...
  def grow(self, text: str) -> Text:
    """The text as more of it is told, which is how a stream of a command grows, and which comes from no text, since a stream that grows is no edit of one."""
  def edit(self, lo: int, hi: int, lines: list[str]) -> Text:
    """The text with one more edit; an edit whose lines are not there is refused.
    The edits that say themselves are one edit each: a replace of a string, an append at the end, an insert before a line, and a delete of lines.
    """
  def replace(self, old: str, new: str, once: bool = False) -> Text:
    """replace(old, new, once) gives a new text with the edit added."""
  def undo(self, n: int = 1) -> Text:
    """undo(n) gives a new text without its last n edits.
    The text before its last n edits, and the text itself when it came from none.
    """
  def append(self, text: str) -> Text:
    """append(text) gives a new text with the text added at its end."""
  def insert(self, line: int, text: str) -> Text:
    """insert(line, text) gives a new text with the text put at that line."""
  def delete(self, lo: int, hi: int) -> Text:
    """delete(lo, hi) gives a new text without the lines lo through hi."""
  def find(self, pattern: str) -> list[int]:
    """find(pattern) gives the numbers of the lines that the pattern matches.
    The numbers of the lines the pattern matches, which is what grep picks of them.
    """

type Read = tuple[Literal["read"], str, str, str, str]
"""A read is the question of the text at a path."""
type Write = tuple[Literal["write"], str, str, str, str, str]
"""A write is the question of putting a content at a path, which carries the path and the content."""
type Cd = tuple[Literal["cd"], str, str, str, str]
"""A cd is a query of a path, which the chain holds in its transcript, and which nobody need answer."""
