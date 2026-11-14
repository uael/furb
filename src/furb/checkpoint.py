"""The record of a life, which stands outside the engine: journal gives the journal that boot takes, which keeps the
record of the life and says the record that it is given again, and fed gives that journal the entries that follow the
tip of a life that stands. gathered gives the data of a checkpoint at the tip of a life, and seed gives the entries
that a later life boots on from that data, so the build that opens a checkpoint, and not the one that wrote it, says
how its own journal makes each chain again.

The journal is the journal that the engine held, in its form. Boot gives it the tables of the life, and at its first
run it binds numbered, offers, drive and lives again in the names of its engine, each over the one that stood. A boot
binds numbered, offers and drive again, and lives is no name that boot binds, so the journal of the next life takes
the one that stood under the last journal. LIFE holds what one life of this process needs past the call that made its
journal: that pair of lives, the function that takes a tail, and the numbered of that journal, by which fed knows
that the life that stands is its own.

This interpreter imports it, and the sandbox of the crate loads it in a module of its own, as it loads the Kernel. Each
call is given the names of its engine, which it reads at each call and never holds, since boot binds the bus into the
engine. No model reads it, so it costs no token of the system prompt, and its contract, checkpoint.pyi, holds its types,
as the contract of the engine holds those of the engine.
"""

import re
from collections import Counter

LIFE = {}


def journal(names, record=()):
  site, question, scope, covers, over, module, acting, drifted, refused = (
    names[x] for x in ("site", "question", "scope", "covers", "over", "module", "acting", "Drift", "Refused")
  )
  record = [*record]
  first, owned, pending = {e[1]: e for (e,) in record[::-1] if e[0] in ("started", "done")}, set(), {}
  due = [e for (e,) in record[::-1] if first.get(e[1]) is not e]
  ranks = {id(e): n for n, (e,) in enumerate(record)}
  saved, again = {e[1]: e for (e,) in record if question(e)}, {}
  asked, strays, back = {}, {}, []

  def ear(outside, known, taken, held, holds, log, left, top, a=(), last=None):
    say, act, get, drive, counted, offers, carried = (
      names[x] for x in ("say", "act", "get", "drive", "numbered", "offers", "lives")
    )
    if carried is LIFE.get("lives"):
      carried = LIFE["carried"]

    def holding(quest):
      top[quest[0]] = max(top[quest[0]], int(quest[1][len(quest[0]) :]))
      asked.setdefault(quest[:3:2], []).append((quest[1], quest[3:]))

    def more(entries):
      fresh = []
      for (e,) in entries:
        ranks[id(e)] = len(ranks)
        if e[0] in ("started", "done") and e[1] not in first and e[1] not in known:
          first[e[1]] = e
        else:
          fresh.append(e)
        if question(e) and saved.setdefault(e[1], e) is e:
          holding(e)
      due[:0] = fresh[::-1]

    for one in saved.values():
      holding(one)

    def drift(name, fact=None):
      ours, name = fact or get(name), strays.get(name) or name
      return drifted(
        f"{name} drifts: the life made {ours or 'no act'!s:.300}, the record holds {(fact and first[fact[1]]) or saved[name]!r:.300}"
      )

    def matched(sort, maker="", *said):
      ahead = asked.get((sort, maker), [])
      at = next((n for n, x in enumerate(ahead) if x[1] == said), None)
      if at is None:
        strays[name := counted(sort)] = ahead[0][0] if ahead else None
        return name
      return ahead.pop(at)[0]

    def numbered(kind, *said):
      name = matched(kind, *said)
      name in known or upto(saved.get(name))
      return name

    def owed():
      while due and question(x := due[-1]) and x[1] in known:
        due.pop()
      return due

    def upto(entry):
      while owed() and not question(x := due[-1]) and ranks[id(x)] < ranks.get(id(entry), 0):
        recount(due.pop())

    def repeat(kind, about, by, *words):
      with site.set(by):
        said = say(kind, about, *words)
      again[id(said)] = said
      held.setdefault((scope(about), "journal"), []).append(said)

    def recount(entry):
      match entry:
        case (kind, name, by, on, *words) if question((kind, name)):
          if (
            by in saved
            or (name not in first and by[-1:].isdigit())
            or (on and not get(on))
            or (by == "journal" and not holds(scope(on), [known.setdefault(name, (kind, name, by, on, *words))]))
          ):
            return
          if by not in outside and (not callable(verb := (module(on) or names).get(kind)) or by[-1:].isdigit()):
            raise drift(name)
          with site.set(by):
            try:
              act(kind, on, None, *words) if by in outside else verb(*words, on=on)
            except refused:
              pass
        case (_, about, *_) if scope(about):
          repeat(*entry)
        case (_, about, by, *_) if by in outside and about in saved and get(saved[about][3]) and not get(about):
          raise drift(about)

    def offered(made):
      kind, name = made[:2]
      if saved.get(name, made) == made:
        if name not in taken and (f := first.get(name)):
          if f[0] == "started":
            pending[name] = f
          upto(f)
          repeat(*f)
          while name in pending and owed() and due[-1][0] == "joined" and due.pop() and due:
            recount(due.pop())
        if kind in ("run", "chain") or not owed() or strays.get(name, name) is None:
          offers(made)

    def driven(g, name):
      fresh = acting() and name not in owned
      fresh and owned.add(name)
      try:
        drive(g, name)
      except refused:
        fresh and owned.discard(name)
        raise

    def lives(g, heard):
      if heard and again.get(id(heard), 0) is heard and heard[2] and site.get() not in known:
        heard = (*heard[:2], "journal", *heard[3:])
      return carried(g, heard)

    LIFE.update(lives=lives, carried=carried, more=more, numbered=numbered)
    names.update(numbered=numbered, offers=offered, drive=driven, lives=lives)
    try:
      while True:
        while owed() and (
          not (log or left)
          or (due[-1][0] == "joined" and not question(due[-2]) and due.pop() and due)
          or (a and a[::2] == ("wake", "journal") and due[-1][1] in taken and due[-1][2] in outside)
        ):
          recount(due.pop())
        while back and not owed():
          yield back.pop(0)
        b = a
        match a := (yield):
          case (kind, about, by, *_) if (
            (q := get(about)) and by != "journal" and (by or repr(first.get(about)) != repr(a))
          ):
            if not by and a != q and ((owed() and strays.get(about, 1)) or about in first):
              raise drift(about, about in first and a)
            if by not in known and by not in owned:
              glue = (
                b is last is not None
                and kind not in ("started", "done")
                and ("keep", "", (("joined", "", "journal", kind, about),))
              )
              if about not in saved:
                a is q and glue and back.append(glue)
                back.append(("keep", "", (saved.setdefault(about, q),)))
              if a is not q:
                glue and back.append(glue)
                back.append(("keep", "", (a,)))
              last = a
            elif a is q and about not in saved:
              back.append(("keep", "", (saved.setdefault(about, q),)))
            if kind == "wake" and not owed():
              for name in [x for x in pending if covers(a, x) and not over(x)]:
                del pending[name]
                taken.discard(name)
                offers(known[name])
    finally:
      for pile in log, left, due, first, saved, back:
        pile.clear()

  return ear


def fed(names, entries):
  def poke():
    return
    yield

  if names["numbered"] is not LIFE.get("numbered"):
    why = "no journal of the checkpointer keeps this life, so nothing takes a tail"
    raise names["Refused"](why)
  LIFE["more"](entries)
  names["drive"](poke(), "tail")


def seed(names, data):
  root, operator = names["ROOT"], names["OPERATOR"]
  entries, numbers = [], Counter()

  def put(sort, called, *words):
    last = int(called[len(sort) :])
    entries.extend(((sort, f"{sort}{n}", root, ""),) for n in range(numbers[sort] + 1, last))
    numbers[sort] = max(numbers[sort], last - 1) + bool(words)
    words and entries.append(((sort, called, *words),))

  asking = {a["on"] for a in data["acts"] if "value" not in a}
  # The journal makes an act again only on a chain or an act that it knows when the entry comes, and each act with a
  # value comes before each open prompt. home gives each act that it makes again the chain that the act is on, a
  # chain being on itself, and the facts of each other act, as one with a value on an open prompt, stay in the
  # prefix.
  home = {c["id"]: c["id"] for c in data["chains"]}
  kinds = {c["id"]: "chain" for c in data["chains"]}
  for x in sorted(data["acts"], key=lambda a: "value" not in a):
    if x["on"] in home:
      home[x["id"]], kinds[x["id"]] = home[x["on"]], x["kind"]
  # The maker of each chain and each act that the data names stands in the fact carried of the chain that the act is
  # on, or of the first chain, since the seed makes each again as the operator or the journal.
  made = {c["id"]: {} for c in data["chains"]}
  for x in [*data["chains"], *data["acts"]]:
    "by" in x and made[home.get(x["id"], data["chains"][0]["id"])].update({x["id"]: x["by"]})
  parts = {}
  for c in data["chains"]:
    put("chain", i := c["id"], operator, c["on"], c["label"], "")
    v = "".join(
      f"try:\n if {k!r} not in __bound or {k} is not __bound[{k!r}]: {k} = __bound[{k!r}] = eval(__carried[4][{k!r}])\nexcept Exception: pass\n"
      for k in c["values"]
    )
    parts[i] = [*filter(None, [v, *([d] if isinstance(d := c["definitions"], str) else d), v, *c.get("lives", ())])]
    kept = [
      t for t in c["turns"] if t[1] not in kinds or home[t[1]] != i or t[0] not in ("started", "done", kinds[t[1]])
    ]
    entries += [
      (("carried", i, "journal", c["turns"], c["values"], parts[i], *[made[i]][: bool(made[i])]),),
      (("prefix", i, i, kept),),
    ]
    (c["paused"] or i in asking) and entries.append((("pause", i, operator),))
  for x in data["acts"]:
    "value" in x and (
      put(x["kind"], i := x["id"], "journal", x["on"], *x.get("words", ())),
      entries.extend([(("started", i, "journal"),), (("done", i, "journal", x["value"]),)]),
    )
  for x in data["acts"]:
    "value" in x or (
      put("prompt", x["id"], operator, x["on"], x["shape"], x["message"], x["to"]),
      x["held"] and entries.append((("started", x["id"], x["held"]),)),
    )
  for kind, most in data["taken"].items():
    put(kind, f"{kind}{most + 1}")
  for c in data["chains"]:
    i, cut = c["id"], c["cut"] + [y["rung"] for y in c.get("again", ())]
    word = (
      "for __carried in [] if transcript(of='cut') else transcript(of='carried')[-1:]:\n"
      " __bound, __part = {}, None\n"
      " for __part in __carried[5]:\n  try: await rung(__part)\n  except Exception as __raised: debug(t'{__raised!r}')\n"
      " say('cut', scope(acting()), "
      "[x for x in reading() if x[0] != 'tell' or not under(x[1], acting()) or x[3][0].startswith(f'#{acting()} debugged')])\n"
      f' (__lost := [n for n in {c["lost"]} + [*__carried[4]] if n not in globals()]) + {cut} and debug(t"The life goes on from a checkpoint. '
      f"It lost {{__lost or 'no name'}}, and it cut {cut or 'no act'}.\")\n del __carried, __bound, __part, __lost"
    )
    seat = f"rung{numbers['rung'] + 1}"
    for maker, said in [(operator, word), *((seat, one) for one in parts[i])]:
      put("rung", f"rung{numbers['rung'] + 1}", maker, i, said, "", "")
  return entries + [(("wake", y["id"], operator),) for y in data["chains"] if y["id"] in asking and not y["paused"]]


def gathered(names, define, floor):
  get, peek, over, scope, question, prompted, lineage, paused, root = (
    names[x] for x in ("get", "peek", "over", "scope", "question", "prompted", "lineage", "paused", "ROOT")
  )
  transcript, reading, module, program, standing, listening, refused = (
    names[x] for x in ("transcript", "reading", "module", "program", "standing", "listening", "Refused")
  )

  def literal(v, into=None):
    isinstance(v, str) and get(v) and into is not None and into.add(v)
    if isinstance(v, names["Act"]):
      return f"Act({str(v)!r})" * (into is not None)
    if isinstance(v, BaseException):
      return literal(v.args) and repr(v)
    if v is None or isinstance(v, (int, float, str, names["Text"], names["Exit"])):
      return repr(v) * (not (isinstance(v, float) and v - v))
    if not isinstance(v, (dict, list, tuple, set)):
      return ""
    pairs = v.items() if isinstance(v, dict) else [(y,) for y in v]
    s = [": ".join(made) if all(made := [literal(y, into) for y in pair]) else "" for pair in pairs]
    ends = "[]" if isinstance(v, list) else "()" if isinstance(v, tuple) else "{}"
    whole = (
      "set()"
      if isinstance(v, set) and not v
      else ends[0] + ", ".join(s) + "," * (ends == "()" and len(s) == 1) + ends[1]
    )
    return whole if all(s) else ""

  def number(called):
    return int(re.sub(r"^\D+", "", called) or 0)

  chains = [x for x in listening() if question(("chain", x))]
  made = {k: v for c in chains for x in transcript(c, "carried") for k, v in (x[6:] or [{}])[0].items()}

  def maker(called):
    return made.get(called, get(called)[2])

  asked = [*map(get, chains)] + [x for h in chains for x in transcript(h) if x is get(x[1])]
  cut = sorted((x[1] for x in asked if x[0] == "run" and not over(x[1]) and get(x[4])), key=number)
  gone = {x[4] for x in asked if x[0] == "wants" and x[2] in cut and prompted(x[4]) and not over(x[1])}
  values, lost, named = {}, {}, {x[1] for x in asked if x[0] in ("prompt", "bash")}
  for c in chains:
    values[c], lost[c] = {}, []
    for key, value in module(c).items():
      if not (key.startswith("__") or callable(value) or names.get(key, values) is value):
        try:
          text = literal(value, found := set())
        except RecursionError:
          text = ""
        if text:
          values[c][key] = text
          named |= found
        else:
          lost[c].append(key)
  taken = dict(floor)
  for one in asked:
    taken[one[0]] = max(taken.get(one[0], 0), number(one[1]))
  words = {y[2] for y in (standing()[4:5] or [()])[0]}
  borne = sorted(
    (x for x in named - gone if get(x)[0] in ("prompt", "bash", "read", "write", "wait", "cd", "chance")),
    key=lambda r: (get(r)[0], number(r)),
  )
  acts = []
  for one in [
    *[x[1] for x in transcript(root, "stand") if over(x[1])][-1:],
    *[x[1] for x in transcript(root, "clock") if isinstance(peek(x[1]), float)][-1:],
    *borne,
  ]:
    a = get(one)
    d = {"kind": a[0], "id": one, "on": a[3], "by": maker(one)}
    if prompted(one) and not over(one):
      acts.append(
        {
          **d,
          "shape": a[4],
          "message": a[5],
          "to": a[6],
          "held": next((x[2] for x in transcript(scope(one), "started") if x[1] == one and get(x[2]) is None), ""),
        }
      )
    elif over(one):
      literal(peek(one), set()) and acts.append({**d, "words": a[4:], "value": peek(one)})
    else:
      cut.append(one)
      acts.append({**d, "value": refused(one + " was interrupted: the life that ran it ended before it did")})
  out = []
  for c in chains:
    a, ended = get(c), {y[1] for y in transcript(c, "done")}
    saved = [
      x
      for x in reading(c)
      if x[0] not in ("module", "carried", "made")
      and literal(x, set())
      and (not question(x) or (x[0] == "reply" and x[1] in ended) or x[1] in borne)
    ]
    held = {*chains, *borne, *(x[1] for x in saved if question(x))}
    out.append(
      {
        "id": c,
        "by": maker(c),
        "on": a[3] * (a[3] in chains),
        "label": a[4],
        "source": a[5],
        "paused": paused(c),
        "turns": [x for x in saved if x[0] not in ("started", "done") or x[1] in held],
        "definitions": [
          *filter(None, [define(w) for r, w in program(c).items() if get(r) and get(r)[2] != "extensions"])
        ],
        "values": values[c],
        "lost": lost[c],
        "cut": [x for x in cut if scope(x) == c],
        "lives": [
          *dict.fromkeys(
            get(r)[4]
            for x in listening()
            if scope(x) == c
            for r in lineage(x)
            if question(("rung", r)) and get(r)[4] in words
          )
        ],
      }
    )
  return {"taken": taken, "chains": out, "acts": acts}
