from furb.engine import (
  HIDDEN,
  Act,
  Refused,
  Show,
  Text,
  act,
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

NAMES = ("AGENTS.md", "CLAUDE.md")
LOCAL = ("AGENTS.local.md", "CLAUDE.local.md")


def lacking(known: set[str]) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if line not in known]


def remembered(path: str) -> bool:
  return path.replace("\\", "/").rsplit("/", 1)[-1] in (*NAMES, *LOCAL)


def joined(folder: str, name: str) -> str:
  return folder + name if folder.endswith(("/", "\\")) else folder + "/" + name


def looked(path: str, on: str = "") -> Text | None:
  try:
    return read(path, HIDDEN, on)
  except Refused:
    return None


def pieces(path: str) -> list[str]:
  kept: list[str] = []
  for one in path.replace("\\", "/").split("/"):
    if one == ".." and kept and kept[-1] not in ("", ".."):
      kept.pop()
    elif one not in (".", "") or not kept:
      kept.append(one)
  return kept


def folders(path: str = ".", on: str = "") -> list[Text]:
  up: list[Text] = []
  at = "./"
  while (got := looked(at, on)) is not None and not (up and got.path == up[-1].path):
    up.append(got)
    at = joined(got.path, "../")
  down: list[Text] = []
  if up and "://" not in path:
    here = pieces(up[0].path)
    absolute = path.startswith(("/", "\\", "~")) or path[1:3] in (":/", ":\\")
    full = pieces(path if absolute else joined(up[0].path, path))
    held = up[0]
    for one in full[len(here) :] if full[: len(here)] == here else []:
      if one + "/" not in held.lines or (deeper := looked(joined(held.path, one + "/"), on)) is None:
        break
      down.append(held := deeper)
  user = looked("config://", on)
  return [*([user] if user else []), *up[::-1], *down]


def memory(path: str = ".", on: str = "") -> list[Text]:
  was: dict[str, list[str]] = {}
  held: set[str] = set()
  for x in transcript(on):
    match x:
      case ("done", about, _, Text(place, content)) if (
        (question(("read", about)) or question(("write", about))) and get(maker := get(about)[2]) and tells(maker)
      ):
        was[place] = content.splitlines()
        if remembered(place):
          held.add(content)
  got: list[Text] = []
  for folder in folders(path, on):
    names = set(folder.lines)
    for group in NAMES, LOCAL:
      name = next((one for one in group if one in names), "")
      if not name or (found := looked(joined(folder.path, name), on)) is None:
        continue
      if found.content in held or any(one.path == found.path or one.content == found.content for one in got):
        continue
      got.append(found)
      mine = set(found.lines)
      near = max(was.values(), key=lambda kept: len(set(kept) & mine), default=[]) if found.path not in was else []
      if found.path in was:
        show = differs(was[found.path])
      elif mine and 2 * len(set(near) & mine) > len(mine):
        show = lacking(set(near))
      else:
        show = span(1, -1)
      tell(on, found.path, found, show)
  return got


async def recall(on: str = "") -> Act:
  return remember(on)


def visited(id: str, before: set[str], about: str, got: object) -> str:
  made = get(about)
  if made is None or made[0] not in ("read", "write", "cd") or scope(about) != scope(id) or about in before:
    return ""
  if not question(("rung", made[2])) or (made[0] == "read" and str(made[4]).endswith(("/", "\\"))):
    return ""
  text = made[4] if made[0] == "write" and isinstance(got, Text) else got
  path = text if isinstance(text, str) else getattr(text, "path", "")
  return "" if "://" in path or remembered(path) else path


def remember(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    before = {x[1] for kind in ("read", "write", "cd") for x in transcript(on, kind)}
    memory(on=on)
    while True:
      match (yield {id, "read", "write", "cd", "stand"}):
        case ("done", about, _, got) if path := visited(id, before, about, got):
          memory(path, on)
        case ("done", about, *_) if question(("stand", about)) and not question(("rung", get(about)[2])):
          memory(on=on)

  return act("remember", on, controlled(ear))
