from furb.engine import (
  HEAD,
  Act,
  Exit,
  Filter,
  Refused,
  Text,
  act,
  acting,
  automatic,
  chain,
  close,
  controlled,
  differs,
  filled,
  get,
  over,
  peek,
  program,
  prompt,
  question,
  read,
  rung,
  say,
  scope,
  take,
  told,
  transcript,
  under,
)

compaction = """Compaction cuts this chain to the work that it still needs, and the chain goes on under its name.
- supersede(keep) cuts this chain to what the filter keep kept, and every act. compact() supersedes this chain with a
  filter that applies FILTERS in order, and its next word binds the summary
  that it is given and reads again each file whose read or write it kept, showing what changed. Its prompts, its
  rungs, the words that run and its module go on.
- A filter drops acts. latest() keeps the last read or write of each path, quiet() drops the commands, recent(n)
  keeps the acts of the last n replies, and await chosen() asks a model which acts to keep. together(*filters)
  applies filters in order.
- choice() asks a model which acts the work still needs, and what the rest holds, as ids and a summary.
- await condense() compacts this chain at once to what choice() chose, with the summary of the model, as compacted()
  does once. The command /compact of the TUI runs it on the chain on screen.
- clear() cuts this chain to nothing, with no summary: its model reads from zero, and the chain keeps its name, its
  module, its prompts and its threads. The command /clear of the TUI runs it on the chain on screen.
- compacted() wraps a chain: after an answer that reads more than SHARE of the window, it asks choice(), and at the
  next reply once that came, it compacts the chain to what the model chose. It moves, stops and pauses nothing.
- fitted() watches a chain: when its model refuses a reply because the prompt passes its window, it cuts the chain at
  once to its open prompts, so the next reply reads less and the chain goes on with no pause.
- The life word wraps each chain with compacted() and fitted(), so a chain compacts with a summary of the model
  before it passes its window, and the cut of fitted() is the last resort.
- A project changes the defaults in an extension of its own, whose word rebinds SHARE, FILTERS, CHOOSER, LISTED or
  BRIEF."""
SHARE = 0.6
CHOOSER = ""


LISTED = 40
OVERFLOW = "the prompt passes the window of the model"
OVERFLOWED = """The model refused the last reply of this chain, since its prompt passed the window of the model,
so compaction cut the chain to its open prompts: read again what the work still needs."""
BRIEF = """The chain below grows past its window, and it goes on with what the work still needs.
Choose what it keeps: each act that holds a decision or a ruling of the operator, a brief, a finding that the work
still needs, and the last state of each command that matters. A step keeps all that it made, and an act keeps the
step that made it. The chain keeps its module, its prompts and its steps, so keep no act for a value that the module
holds. It reads again each file whose read or write you keep and shows what changed since, so keep the last read or
write of each file that the work still needs, and no other.
Close with one call, close((ids, summary)): the ids to keep, as list[str] of the heads below, and a summary, in a
few short lines, of what the acts that you drop hold and the work still needs to know. A tuple written bare closes
nothing, and the choice ends refused after 4 words that close nothing."""


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
  heads = [item[1] for item in acts if item[0] != "rung"][:2] or [item[1] for item in acts][:2]
  example = "close(([" + ", ".join(map(repr, heads)) + "], 'what the acts that you drop hold'))"
  brief = (
    BRIEF
    + "\n\n"
    + "\n".join(lines)
    + "\n\nAnswer with this one call alone, the ids taken from the heads above:\n"
    + example
  )
  side = chain("chosen", on=here)
  asked: Act = prompt(tuple[list[str], str], brief, to=to or CHOOSER or automatic(), on=side)
  bound = 4

  def ear(id):
    yield "started", id
    replies = 0
    while not over(asked):
      match (yield {id, side, asked}):
        case ("done", about, _, _) if question(("reply", about)) and scope(about) == side:
          replies += 1
          if replies >= bound and not over(asked):
            close(Refused(f"{asked}: the model of choice wrote {replies} words on {side} and closed none"), asked)
    yield "done", id, None

  act("bounded", side, controlled(ear))
  return asked


async def chosen(to: str = "", on: str = "") -> Filter:
  got = await choice(to, on)
  assert isinstance(got, tuple)
  return take(*got[0])


FILTERS: list[Filter] = [latest(), recent(8)]


regrounded: set[str] = set()


def reground(on: str = "") -> None:
  here = on or scope(acting())
  voiced = {x[1] for x in transcript(here) if x[0] == "tell"}
  held: dict[str, tuple[Text, bool]] = {}
  for a in transcript(here):
    if (
      a[0] in ("read", "write")
      and a[1] in voiced
      and isinstance(t := peek(a[1]), Text)
      and "://" not in t.path
      and get(t.path.split("/")[0]) is None
    ):
      held[t.path] = (t, a[1] in regrounded)
  for path, (text, again) in held.items():
    seen = {x[1] for x in transcript(here) if x[0] == "read"}
    try:
      read(path, HEAD if again else differs(text.lines), on=on)
    except Refused:
      continue
    regrounded.update(x[1] for x in transcript(here) if x[0] == "read" and x[1] not in seen)


def headline(fact: tuple) -> str:
  """The first line of the first note of a tell, or else the kind of the fact."""
  match fact:
    case ("tell", _, _, [str(first), *_]):
      return first.split("\n")[0]
  return str(fact[0])


def supersede(keep: Filter | None = None, on: str = "") -> str:
  here = on or scope(acting())
  theirs = transcript(here)
  kin = {here}
  acts = []
  for x in theirs:
    match x:
      case (str(kind), str(name), str(by), str(where), *words) if question(x):
        acts.append((kind, name, by, where, *words))
  for one in (keep or list)(acts):
    name = one[1]
    while name not in kin and (b := get(name)):
      kin.add(name)
      name = b[2]

  heads = [headline(fact) for fact in theirs]
  lo = heads.index("chain()") if "chain()" in heads else len(theirs)
  hi = heads.index("#" + here, lo) if "#" + here in heads[lo:] else lo
  setup = {id(fact) for fact in theirs[lo:hi]}

  def stays(it: tuple) -> bool:
    if id(it) in setup:
      return True
    made = get(it[1])
    if made is None:
      return it[2] in kin
    return question(it) or (made[2] if question(("reply", it[1])) else it[1]) in kin

  say("prefix", here, [it for it in theirs if stays(it)])
  return here


def compact(filters: list[Filter] | None = None, summary: str = "", on: str = "") -> str:
  here = supersede(together(*(FILTERS if filters is None else filters)), on)
  name = "summary"
  while f"</s:{name}>" in summary:
    name += "_"
  quoted = f"<s:{name}>\n{summary}</s:{name}>\n" if summary else ""
  rung(f"{quoted}reground()", on=here)
  return here


def clear(on: str = "") -> str:
  return supersede(lambda _: [], on)


def compacted(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here, pending, seen, refused = scope(id), "", set(), False
    chose: tuple | None = None

    def keeping(ids: list[str], after: set) -> Filter:
      return lambda acts: take(*ids, *[a[1] for a in acts if a[1] not in after])(acts)

    while True:
      match (yield {id, here} | ({pending} if pending else set())):
        case ("done", about, _, Refused() as why) if about == pending:
          pending, refused = "", True
          yield told(id, "kept the chain as it is", why=why)
        case ("done", about, _, value) if about == pending:
          chose = (None, "")
          match value:
            case (list() as listed, str() as summary):
              chose = ([keeping([i for i in listed if isinstance(i, str) and scope(i)], set(seen)), latest()], summary)
        case ("done", about, *_) if (
          question(("reply", about)) and scope(about) == here and not (pending or chose or refused)
        ):
          if (filled(about) or 0) > SHARE:
            seen = {fact[1] for fact in transcript(here) if question(fact)}
            pending = str(choice(on=here))
        case ("reply", about, *_) if chose and question(("reply", about)) and scope(about) == here:
          compact(chose[0], summary=chose[1].strip(), on=here)
          pending, chose = "", None

  return act("compacted", on, controlled(ear))


def fitted(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here, row = scope(id), False
    while True:
      match (yield {id, here}):
        case ("done", about, _, Refused() as why) if question(("reply", about)) and scope(about) == here:
          if OVERFLOW in str(why) and not row:
            compact([lambda acts: [a for a in acts if a[0] == "prompt" and not over(a[1])]], OVERFLOWED, here)
          row = OVERFLOW in str(why)
        case ("done", about, _, tuple()) if question(("reply", about)) and scope(about) == here:
          row = False

  return act("fitted", on, controlled(ear))


async def condense(to: str = "", on: str = "") -> str:
  here = on or scope(acting())
  before = {fact[1] for fact in transcript(here) if question(fact)}
  match await choice(to, here):
    case (list() as listed, str() as summary):
      ids = [i for i in listed if isinstance(i, str) and scope(i)]

      def chose(acts: list) -> list:
        return take(*ids, *[a[1] for a in acts if a[1] not in before])(acts)

      return compact([chose, latest()], summary.strip(), here)
  return compact(on=here)
