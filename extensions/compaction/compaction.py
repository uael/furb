from furb.engine import (
  OPERATOR,
  Act,
  Exit,
  Filter,
  Refused,
  Text,
  act,
  acting,
  chain,
  close,
  controlled,
  differs,
  filled,
  get,
  module,
  over,
  peek,
  program,
  prompt,
  question,
  read,
  rung,
  scope,
  site,
  take,
  transcript,
  under,
)

compaction = """Compaction goes on in a new chain that keeps less of this one.
- compact() makes that chain and gives it: its source is this chain, its filter applies FILTERS in order, and its
  first word binds again what the module of this chain holds, binds the summary that it is given, and reads again
  each file that it kept a read or write of, showing what changed. Prompt on it to go on.
- A filter drops acts. latest() keeps the last read or write of each path, quiet() drops the commands, recent(n)
  keeps the acts of the last n replies, and await chosen() asks a model which acts to keep. together(*filters)
  applies filters in order.
- choice() asks a model which acts the work still needs, and what the rest holds, as ids and a summary.
- compacted() wraps a chain: after an answer that reads more than SHARE of the window, it asks choice(), and at the
  next reply once that came, it compacts the chain to what the model chose, and moves each open prompt of a model to
  the new chain. A word that it stops runs again only when a word runs it again, and the summary names it. No chain
  is wrapped unless a word wraps it.
- A project changes the defaults in an extension of its own, whose word rebinds SHARE, FILTERS, CHOOSER, LISTED,
  BRIEF or LABEL."""
SHARE = 0.6
LABEL = "compacted"
CHOOSER = ""
LISTED = 40
BRIEF = """The chain below grows past its window, and a new chain goes on with what the work still needs.
Choose what it keeps: each act that holds a decision or a ruling of the operator, a brief, a finding that the work
still needs, and the last state of each command that matters. A step keeps all that it made, and an act keeps the
step that made it. The new chain holds the module of this one, so keep no act for a value that the module holds. It
reads again each file whose read or write you keep and shows what changed since, so keep the last read or write of
each file that the work still needs, and no other. Keep no open prompt: it moves to the new chain with its message.
An open step stops, and the summary gives its word: keep of it only the acts that the work still needs.
Close with a tuple: the ids to keep, as list[str], and a summary, in a few short lines, of what the acts that you
drop hold and the work still needs to know."""


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


def choice(to: str = "", on: str = "") -> Act:
  here = on or scope(acting())
  words = program(here)
  acts = [a for a in transcript(here) if question(a) and a[0] not in ("reply", "run", "wants", "gate", "stand")]
  made: dict[str, list[str]] = {}
  for entry in acts:
    made.setdefault(entry[2], []).append(entry[1])
  lines = ["Open prompts:"]
  lines += [f"{a[1]} to {a[6] or 'its model'}: {a[5]!r:.300}" for a in acts if a[0] == "prompt" and not over(a[1])]
  lines.append("Acts:")
  for entry in acts:
    held = peek(entry[1])
    kilo = (
      len(held.content)
      if isinstance(held, Text)
      else len(held.stdout.content + held.stderr.content)
      if isinstance(held, Exit)
      else 0
    ) // 1000
    state = ("" if over(entry[1]) else " (open)") + (f" ({kilo}k chars)" if kilo else "")
    if (get(entry[2]) or ("",))[0] == "rung":
      if entry[1] in made[entry[2]][-LISTED:]:
        lines.append(f"  {entry[1]} {', '.join(map(repr, entry[4:]))[:120]}{state}")
    elif entry[0] == "rung":
      left = len(made.get(entry[1], [])) - LISTED
      word = " ".join(words.get(entry[1], "").split())[:160] or "(no word)"
      lines.append(
        f"{entry[1]} step of {entry[2]}{state}: {word}" + (f" ({left} earlier acts of it unlisted)" if left > 0 else "")
      )
    else:
      lines.append(f"{entry[1]} {', '.join(map(repr, entry[4:]))[:200]}{state}")
  brief = BRIEF + "\n\n" + "\n".join(lines)
  return prompt(tuple[list[str], str], brief, to=to or CHOOSER, on=chain("chosen", on=here))


async def chosen(to: str = "", on: str = "") -> Filter:
  got = await choice(to, on)
  assert isinstance(got, tuple)
  return take(*got[0])


FILTERS: list[Filter] = [latest(), recent(8)]


def restore(source: str, on: str = "") -> None:
  def cloned(thing: object) -> object:
    match thing:
      case dict():
        return {key: cloned(one) for key, one in thing.items()}
      case list() | set():
        return type(thing)(map(cloned, thing))
    return thing

  here, fresh = module(on), []
  for name, value in module(source).items():
    if not (callable(value) or name.startswith("__") or here.get(name, here) is value):
      if name not in here:
        fresh.append(name)
      here[name] = cloned(value)
  if fresh:
    with site.set(scope(on or acting())):
      rung("\n".join(f"{one} = globals()[{one!r}]" for one in fresh), on=on)


def reground(on: str = "") -> None:
  here = on or scope(acting())
  held: dict[str, Text] = {}
  for a in transcript(here):
    if (
      a[0] in ("read", "write")
      and scope(a[1]) != here
      and isinstance(t := peek(a[1]), Text)
      and "://" not in t.path
      and get(t.path.split("/")[0]) is None
    ):
      held[t.path] = t
  for path, text in held.items():
    try:
      read(path, differs(text.lines), on=on)
    except Refused:
      continue


def compact(filters: list[Filter] | None = None, label: str = "", summary: str = "", on: str = "") -> str:
  here = on or scope(acting())
  made = chain(label or LABEL, here, together(*(FILTERS if filters is None else filters)), on)
  name = "summary"
  while f"</s:{name}>" in summary:
    name += "_"
  quoted = f"<s:{name}>\n{summary}</s:{name}>\n" if summary else ""
  rung(f"restore({here!r})\n{quoted}reground()", on=made)
  return made


def compacted(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here, old, moved, pending, seen = scope(id), set(), {}, "", set()
    chose: tuple | None = None

    def movable(entry: tuple) -> bool:
      return entry[0] == "prompt" and not over(entry[1]) and entry[6] != OPERATOR and not under(id, entry[1])

    def keeping(ids: list[str], after: set) -> Filter:
      return lambda acts: take(
        *[i for i in ids if not question(("rung", i)) or over(i)], *[a[1] for a in acts if a[1] not in after]
      )(acts)

    def move(target: str) -> None:
      rung(f"await Act({target!r})", on=target)
      for entry in transcript(scope(target)):
        if entry[0] == "rung" and entry[2] == target and not over(entry[1]):
          close(None, entry[1])
      shape, message, to = (str(word) for word in get(target)[4:])
      moved[str(prompt(shape, message, to, on=here))] = target

    def squeeze(moving: list[str], picked: tuple) -> str:
      words = program(here)
      awaited = {get(x[2])[4]: x[4] for x in transcript(here, "wants") if not over(x[1])}
      stopped = [
        f"The compaction stopped {entry[1]}, a step of {entry[2]}, as it awaited {awaited.get(entry[1], 'nothing')}."
        " Each line of its word before that await ran, and the module holds what it bound, so run none of them"
        " again: go on from that await, as by running again the loop that holds it. Its word was:\n" + words[entry[1]]
        for entry in transcript(here)
        if entry[0] == "rung" and entry[2] in moving and not over(entry[1]) and entry[1] in words
      ]
      return compact(picked[0], summary="\n\n".join([picked[1].strip(), *stopped]).strip(), on=here)

    while True:
      match (yield {id, here, *old, *moved} | ({pending} if pending else set())):
        case ("done", about, _, value) if about in moved:
          close(value, moved.pop(about))
        case ("done", about, _, value) if about == pending:
          chose = (None, "")
          match value:
            case (list() as listed, str() as summary):
              chose = ([keeping([i for i in listed if isinstance(i, str) and scope(i)], set(seen)), latest()], summary)
        case ("done", about, *_) if question(("reply", about)) and scope(about) == here and not (pending or chose):
          if (filled(about) or 0) > SHARE:
            seen = {fact[1] for fact in transcript(here) if question(fact)}
            pending = str(choice(on=here))
        case ("reply", about, *_) if question(("reply", about)) and scope(about) in old:
          maker = get(get(about)[2])[2]
          if movable(get(maker) or ("",)):
            move(maker)
          else:
            close(None, get(about)[2])
        case ("reply", about, *_) if chose and question(("reply", about)) and scope(about) == here:
          if going := [fact[1] for fact in transcript(here) if movable(fact) and scope(fact[1]) == here]:
            old.add(here)
            here = squeeze(going, chose)
            for each in going:
              move(each)
            pending, chose = "", None

  return act("compacted", on, controlled(ear))
