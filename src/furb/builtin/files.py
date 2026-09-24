import re
from dataclasses import dataclass

from furb.engine import Refused, Show, ask, commented, question, scope, site, tell


def span(lo: int, hi: int) -> Show:
  def picks(lines):
    i, j = (x + len(lines) + 1 if x < 0 else x for x in (lo, hi))
    return list(range(max(i, 1), min(j, len(lines)) + 1))

  return picks


def grep(pattern: str) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if re.search(pattern, line)]


def differs(old: list[str]) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if old[i - 1 : i] != [line]]


HEAD, HIDDEN = span(1, 2000), span(0, 0)


def door(path, on=""):
  return question(("prompt", path)) and scope(path) == (on or scope(site.get()))


def landed(got):
  match got:
    case {"path": str(path), "content": str(content)}:
      return Text(path, content)
  return got


def showing(got, show):
  return [(got.path, got.content, show) if isinstance(got, Text) else commented(repr(got))]


def read(path: str, show: Show = HEAD, on: str = "") -> Text:
  got = Text(path, ask("ladder", on, path)[1]) if door(path, on) else landed(ask("read", on, path)[1])
  if show is not HIDDEN:
    tell("read", path, *showing(got, show))
  return got


def write(text: Text, on: str = "") -> Text:
  if door(text.path, on):
    ask("ladder", on, text.path, text.content)
    return text
  got = landed(ask("write", on, text.path, text.content)[1])
  if not isinstance(got, Text) or got.lines != text.lines:
    tell("write", text.path, *showing(got, differs(text.lines)))
  return got


def cd(path: str, on: str = "") -> str:
  ask("cd", on, path)
  tell("cd", path)
  return path


def cwd(on: str = "") -> str:
  here = on or scope(site.get())
  match ask("transcript", here, here)[1], ask("stand", here)[1]:
    case list(heard), [_, str(where), _]:
      return next((a[4] for a in reversed(heard) if a[0] == "cd"), where)
  raise Refused(f"no chain {here}")


@dataclass
class Text:
  path: str
  content: str = ""
  before: Text | None = None

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()

  def grow(self, text: str) -> Text:
    return Text(self.path, self.content + text)

  def edit(self, lo: int, hi: int, lines: list[str]) -> Text:
    now = self.content.splitlines(True)
    if not 1 <= lo <= hi + 1 <= len(now) + 1:
      raise Refused(f"{self.path} no lines {lo}:{hi}")
    now[lo - 1 : hi] = [x.removesuffix("\n") + "\n" for x in lines]
    return Text(self.path, "".join(now), self)

  def replace(self, old: str, new: str, once: bool = False) -> Text:
    return Text(self.path, self.content.replace(old, new, 1 if once else -1), self)

  def undo(self, n: int = 1) -> Text:
    return self.before.undo(n - 1) if n > 0 and self.before else self

  def append(self, text: str) -> Text:
    return self.insert(len(self.lines) + 1, text)

  def insert(self, line: int, text: str) -> Text:
    return self.edit(line, line - 1, text.splitlines(True))

  def delete(self, lo: int, hi: int) -> Text:
    return self.edit(lo, hi, [])

  def find(self, pattern: str) -> list[int]:
    return grep(pattern)(self.lines)
