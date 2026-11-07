from furb.engine import HEAD, Act, Show, Text, act, controlled, opened, read, shown, told


def skills(on: str = "") -> Text:
  return read("skills://", on=on)


def skill(name: str = "", show: Show = HEAD, on: str = "") -> Act[Text]:
  def ear(id: str):
    yield opened(id, "Text", name=name)
    while True:
      match (yield):
        case ("done", about, _, got) if about == id:
          if isinstance(got, Text):
            yield told(id, "done", text=shown(got, show))
          return

  return skill_made(Text, act("skill", on, controlled(ear, lasting=True), name))


def skill_made[T](_shape: type[T], made) -> Act[T]:
  return made


async def listed(on: str = "") -> None:
  skills(on)


def defined(*names: str) -> Show:
  def ident(text: str) -> str:
    size = 0
    while size < len(text) and (text[size].isalnum() or text[size] == "_"):
      size += 1
    return text[:size]

  def named(line: str) -> str:
    for prefix in ("async def ", "def ", "class ", "type "):
      if line.startswith(prefix):
        return ident(line[len(prefix) :])
    name = ident(line)
    return name if line[len(name) :].lstrip()[:1] in (":", "=") else ""

  def show(lines: list[str]) -> list[int]:
    out: list[int] = []
    at, start, decorated = 0, 0, False
    while at < len(lines):
      first = at
      at += 1
      while at < len(lines) and (not lines[at] or lines[at][0] in " \t)]}"):
        at += 1
      if lines[first].startswith("@") or (decorated and lines[first].startswith("#")):
        decorated = True
        continue
      decorated = False
      end = at
      while end > first + 1 and not lines[end - 1]:
        end -= 1
      if named(lines[first]) in names:
        out.extend(range(start + 1, end + 1))
      start = at
    return out

  return show
