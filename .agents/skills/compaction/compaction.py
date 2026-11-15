from furb.engine import (
  HEAD,
  Act,
  Filter,
  Overflow,
  Refused,
  Text,
  act,
  acting,
  controlled,
  differs,
  filled,
  get,
  jump,
  module,
  operated,
  over,
  peek,
  prompt,
  question,
  read,
  reading,
  scope,
  take,
  told,
  transcript,
  under,
)

compaction = """Compaction jumps this chain to a new chain that holds the work that it still needs, and each name of
this chain then names the new one, so the work goes on there, and the turns of this chain stay as its model read them.
- supersede(keep) jumps this chain to a chain that keeps what the filter keep kept, the setup, and each stand and
  cd. compact() supersedes this chain with a filter that applies FILTERS in order, and its first word there
  binds the summary that it is given and reads again each file whose read or write it kept, showing what changed. Its
  prompts, its threads and its module go on there. A jump waits until no word of the chain runs.
- A filter drops acts. latest() keeps the last read or write of each path, quiet() drops the commands, recent(n)
  keeps the acts of the last n replies, and await chosen() asks the model of this chain which acts to keep.
  together(*filters) applies filters in order.
- choice() asks on this chain the model that wrote its last reply, which reads the chain from its cache, which acts
  the work still needs, and what the rest holds, as ids and a summary. The turn of that answer stays in the chain.
- await condense() compacts this chain at once to what choice() chose, with the summary of the model, as compacted()
  does once.
- clear() jumps this chain to a chain with the setup alone, with no summary: its model reads from zero, and its
  prompts and its threads go on there.
- compacted() wraps a chain: after an answer that reads more than SHARE of the window, it asks choice(), and at the
  next reply once that came, it compacts the chain to what the model chose. It asks again only once an answer reads
  AGAIN more of the window than the first answer after that compaction. It stops and pauses nothing.
- fitted() watches a chain: when its model refuses a reply because the prompt passes its window, it compacts the
  chain at once to its open prompts, so the next reply reads less and the chain goes on with no pause.
- The life word wraps each chain with compacted() and fitted(), so a chain compacts with a summary of the model
  before it passes its window, and the compaction of fitted() is the last resort.
- A project changes the defaults in a skill of its own, whose word rebinds SHARE, AGAIN, FILTERS or BRIEF."""
SHARE = 0.6
AGAIN = 0.15
OVERFLOWED = """The model refused the last reply of this chain, since its prompt passed the window of the model,
so compaction kept of the chain its open prompts: read again what the work still needs."""
BRIEF = """The chain grows past its window, and it goes on with what the work still needs.
Choose what it keeps: each act that holds a decision or a ruling of the operator, a brief, a finding that the work
still needs, and the last state of each command that matters. A step keeps all that it made, and an act keeps the
step that made it. The chain keeps its module, its prompts and its steps, so keep no act for a value that the module
holds. It reads again each file whose read or write you keep and shows what changed since, so keep the last read or
write of each file that the work still needs, and no other.
Close this prompt with one call, close((ids, summary)): the ids to keep, as list[str], as ["read3", "bash7"], and a
summary, in a few short lines, of what the acts that you drop hold and the work still needs to know. Do no other work
in this word."""


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


def choice(on: str = "") -> Act:
  here = on or scope(acting())
  wrote = [str(a[4]) for a in transcript(here, "reply") if isinstance(peek(a[1]), tuple) and not operated(str(a[4]))]
  return prompt("tuple", BRIEF, to=wrote[-1] if wrote else module(here)["actor"], on=here)


async def chosen(on: str = "") -> Filter:
  got = await choice(on)
  assert isinstance(got, tuple)
  return take(*got[0])


FILTERS: list[Filter] = [latest(), recent(8)]


def reground(on: str = "") -> None:
  here = on or scope(acting())
  voiced = {x[1] for x in reading(here) if x[0] == "tell"}
  held: dict[str, tuple[Text, bool]] = {}
  for a in reading(here):
    if (
      a[0] in ("read", "write")
      and a[1] in voiced
      and isinstance(t := peek(a[1]), Text)
      and "://" not in t.path
      and get(t.path.split("/")[0]) is None
    ):
      whole = t.content.removesuffix("\n")
      held[t.path] = (t, all(whole not in "".join(map(str, x[3])) for x in reading(here) if x[:2] == ("tell", a[1])))
  for path, (text, again) in held.items():
    try:
      read(path, HEAD if again else differs(text.lines), on=on)
    except Refused:
      continue


def supersede(keep: Filter | None = None, on: str = "", word: str = "") -> Act[str]:
  here = on or scope(acting())
  acts = [(x[0], x[1], x[2], str(x[3]), *x[4:]) for x in reading(here) if question(x)]
  kept = {x[1] for x in (keep(acts) if keep else acts)}
  return jump([x[1] for x in acts if x[1] not in kept], word, here)


def compact(filters: list[Filter] | None = None, summary: str = "", on: str = "") -> Act[str]:
  name = "summary"
  while f"</s:{name}>" in summary:
    name += "_"
  quoted = f"<s:{name}>\n{summary}</s:{name}>\n" if summary else ""
  return supersede(together(*(FILTERS if filters is None else filters)), on, f"{quoted}reground()")


def clear(on: str = "") -> Act[str]:
  return supersede(lambda _: [], on)


def compacted(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    pending, seen, refused, jumping, floor = "", set(), False, "", 0.0
    chose: tuple | None = None

    def keeping(ids: list[str], after: set) -> Filter:
      return lambda acts: take(*ids, *[a[1] for a in acts if a[1] not in after])(acts)

    while True:
      match (yield {id, (here := scope(id))} | ({pending} if pending else set())):
        case ("done", about, _, Refused() as why) if about == pending:
          pending, refused = "", True
          yield told(id, "kept the chain as it is", why=why)
        case ("done", about, _, value) if about == pending:
          chose = (None, "")
          match value:
            case ([*listed], str() as summary):
              chose = ([keeping([i for i in listed if isinstance(i, str) and scope(i)], set(seen)), latest()], summary)
        case ("done", about, *_) if (
          question(("reply", about)) and scope(about) == here and not (pending or chose or refused)
        ):
          share = filled(about) or 0
          if jumping and get(about)[3] != jumping:
            jumping, floor = "", share + AGAIN
          if not jumping and share > max(SHARE, floor):
            seen = {fact[1] for fact in transcript(here) if question(fact)}
            pending = str(choice(on=here))
        case ("reply", about, *_) if chose and question(("reply", about)) and scope(about) == here:
          compact(chose[0], summary=chose[1].strip(), on=here)
          pending, chose, jumping = "", None, here

  return act("compacted", on, controlled(ear))


def fitted(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    row = False
    while True:
      match (yield {id, (here := scope(id))}):
        case ("done", about, _, Refused() as why) if question(("reply", about)) and scope(about) == here:
          if isinstance(why, Overflow) and not row:
            compact([lambda acts: [a for a in acts if a[0] == "prompt" and not over(a[1])]], OVERFLOWED, here)
          row = isinstance(why, Overflow)
        case ("done", about, _, tuple()) if question(("reply", about)) and scope(about) == here:
          row = False

  return act("fitted", on, controlled(ear))


async def condense(on: str = "") -> Act[str]:
  here = on or scope(acting())
  before = {fact[1] for fact in transcript(here) if question(fact)}
  match await choice(here):
    case ([*listed], str() as summary):
      ids = [i for i in listed if isinstance(i, str) and scope(i)]

      def chose(acts: list) -> list:
        return take(*ids, *[a[1] for a in acts if a[1] not in before])(acts)

      return compact([chose, latest()], summary.strip(), here)
  return compact(on=here)
