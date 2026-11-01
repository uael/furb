from furb.engine import (
  Act,
  Show,
  Text,
  act,
  ask,
  controlled,
  differs,
  get,
  question,
  read,
  scope,
  span,
  tell,
  tells,
  transcript,
)


def lacking(known: set[str]) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if line not in known]


def memory(path: str = ".", on: str = "") -> list[Text]:
  was: dict[str, list[str]] = {}
  for x in transcript(on):
    match x:
      case ("done", about, _, list(texts)) if (
        question(("memory", about)) and get(maker := get(about)[2]) and tells(maker)
      ):
        for one in texts:
          was[one.path] = one.lines
      case ("done", about, _, Text(place, content)) if (
        (question(("read", about)) or question(("write", about))) and get(maker := get(about)[2]) and tells(maker)
      ):
        was[place] = content.splitlines()
  got = ask("memory", on, path)
  assert isinstance(got, list)
  for one in got:
    mine = set(one.lines)
    near = max(was.values(), key=lambda held: len(set(held) & mine), default=[]) if one.path not in was else []
    kept = set(near)
    if one.path in was:
      show = differs(was[one.path])
    elif mine and 2 * len(kept & mine) > len(mine):
      show = lacking(kept)
    else:
      show = span(1, -1)
    tell(on, one.path, one, show)
  return got


async def recall(on: str = "") -> Act:
  got = ask("memory", on, ".")
  assert isinstance(got, list)
  for one in got:
    read(one.path, on=on)
  return remember(on)


def remember(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    before = {x[1] for x in transcript(on, "read")}
    memory(on=on)
    while True:
      match (yield {id, "read", "stand"}):
        case ("done", about, _, Text(path)) if (
          question(("read", about)) and scope(about) == scope(id) and "://" not in path and about not in before
        ):
          memory(path, on)
        case ("done", about, *_) if question(("stand", about)) and not question(("rung", get(about)[2])):
          memory(on=on)

  return act("remember", on, controlled(ear))
