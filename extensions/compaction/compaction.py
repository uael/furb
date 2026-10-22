from furb.engine import (
  OPERATOR,
  Act,
  Filter,
  Refused,
  Text,
  act,
  acting,
  cancel,
  chain,
  close,
  controlled,
  filled,
  get,
  over,
  pause,
  peek,
  prompt,
  question,
  read,
  rung,
  scope,
  take,
  transcript,
  under,
)

compaction = """Compaction goes on in a new chain that keeps less of this one.
- compact() makes that chain and gives it: its source is this chain, its filter applies FILTERS in order, and its
  first word reads again each file that this chain read or wrote. Prompt on it to go on.
- A filter drops acts. latest() keeps the last read or write of each path, quiet() drops the commands, recent(n)
  keeps the acts of the last n replies, and await chosen() asks a model which acts to keep. together(*filters)
  applies filters in order.
- compacted() wraps a chain: after an answer that reads more than SHARE of the window, it compacts the chain at its
  next reply, moves each open prompt of a model to the new chain, and wraps that chain. No chain is wrapped unless a
  word wraps it.
- A project changes the defaults in an extension of its own, whose word rebinds SHARE, FILTERS, CHOOSER or LABEL."""
SHARE = 0.6
LABEL = "compacted"
CHOOSER = ""


def together(*filters: Filter) -> Filter:
  def keep(acts: list) -> list:
    for one in filters:
      acts = one(acts)
    return acts

  return keep


def latest() -> Filter:
  def keep(acts: list) -> list:
    last = {t.path: a[1] for a in acts if a[0] in ("read", "write") and isinstance(t := peek(a[1]), Text)}
    return [
      a for a in acts if a[0] not in ("read", "write") or a[1] in last.values() or not isinstance(peek(a[1]), Text)
    ]

  return keep


def quiet() -> Filter:
  return lambda acts: [a for a in acts if a[0] != "bash"]


def recent(n: int) -> Filter:
  def keep(acts: list) -> list:
    rungs = [a[2] for a in acts if a[0] == "reply"][-n:] if n > 0 else []
    return [a for a in acts if any(under(a[1], one) for one in rungs)]

  return keep


async def chosen(to: str = "", on: str = "") -> Filter:
  listed = "\n".join(f"{a[1]} {', '.join(map(repr, a[4:]))[:100]}" for a in transcript(on) if question(a))
  brief = "Give the ids of the acts that the work still needs, as list[str]. An act keeps all that it made.\n"
  ids = await prompt(list[str], brief + listed, to=to or CHOOSER, on=chain("chosen"))
  assert isinstance(ids, list)
  return take(*ids)


FILTERS: list[Filter] = [latest(), recent(8)]


def reground(source: str, on: str = "") -> None:
  for path in dict.fromkeys(
    t.path
    for a in transcript(source)
    if a[0] in ("read", "write")
    and isinstance(t := peek(a[1]), Text)
    and "://" not in t.path
    and get(t.path.split("/")[0]) is None
  ):
    try:
      read(path, on=on)
    except Refused:
      continue


def compact(filters: list[Filter] | None = None, label: str = "", on: str = "") -> str:
  here = on or scope(acting())
  made = chain(label or LABEL, here, together(*(FILTERS if filters is None else filters)), on)
  rung(f"reground({here!r})", on=made)
  return made


def compacted(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here, due, moved = scope(id), False, {}
    while True:
      match (yield {id, here, *moved}):
        case ("done", about, _, value) if about in moved:
          close(value, moved.pop(about))
        case ("done", about, *_) if question(("reply", about)) and scope(about) == here:
          due = (filled(about) or 0) > SHARE
        case ("reply", about, *_) if due and question(("reply", about)) and scope(about) == here:
          going = [
            x[1]
            for x in transcript(here)
            if x[0] == "prompt" and scope(x[1]) == here and not over(x[1]) and x[6] != OPERATOR and not under(id, x[1])
          ]
          if going:
            made = compact(on=here)
            for one in going:
              pause(one)
              shape, message, to = (str(x) for x in get(one)[4:])
              moved[prompt(shape, message, to, on=made)] = one
            if any(under(about, p) for p in going):
              cancel(about)
            here, due = made, False

  return act("compacted", on, controlled(ear))
