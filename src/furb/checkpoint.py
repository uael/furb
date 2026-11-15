"""The record of a life, which stands outside the engine: journal gives the journal that boot takes, which keeps the
record of the life and says the record that it is given again, and fed gives that journal the entries that follow the
tip of a life that stands. gathered gives the data of a checkpoint at the tip of a life, and seed gives the entries
that a later life boots on from that data, so the build that opens a checkpoint, and not the one that wrote it, says
how its own journal makes each chain again.

The journal is the journal that the engine held, in its form. Boot calls it with numbered, offers, drive and lives,
which it defines again at each boot, and with the tables of the life by name, binds again in their place the four
that the call gives, and drives the ear that the call gives last under the name journal. fed and held are given the
journal that a boot took, so no life holds anything of another and nothing outlives the life.

This interpreter imports it, and the sandbox of the crate loads it in a module of its own, as it loads the Kernel. Each
call is given the names of its engine, which it reads at each call and never holds, since boot binds the bus into the
engine. No model reads it, so it costs no token of the system prompt, and its contract, checkpoint.pyi, holds its types,
as the contract of the engine holds those of the engine.
"""

import re
from collections import Counter


class Journal:
  """The journal of one life: boot calls it with the four names that it binds again, numbered, offers, drive and
  lives, and with the tables of the life, binds again the four that the call gives, and drives the ear that it gives
  last under the name journal, which says the record again and keeps what the life says. more and flying stand once a
  boot took it, and fed and held reach the life through them."""

  def __init__(self, names, record=()):
    self.names, self.record, self.more, self.flying = names, [*record], None, None

  def __call__(self, counted, offers, drive, carried, *, outside, known, taken, held, holds, log, top):
    names, record = self.names, self.record
    site, question, scope, covers, over, module, acting, drifted, refused, say, act, get, transcript = (
      names[x]
      for x in (
        *("site", "question", "scope", "covers", "over", "module", "acting", "Drift", "Refused"),
        *("say", "act", "get", "transcript"),
      )
    )
    first, owned, pending = {e[1]: e for (e,) in record[::-1] if e[0] in ("started", "done")}, set(), {}
    due = [e for (e,) in record[::-1] if first.get(e[1]) is not e]
    ranks = {id(e): n for n, (e,) in enumerate(record)}
    saved, again = {e[1]: e for (e,) in record if question(e)}, {}
    asked, strays, back = {}, {}, []
    a, last = (), None

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
      while owed() and not question(x := due[-1]) and ranks[id(x)] < ranks.get(id(entry), len(ranks)):
        recount(due.pop())

    def repeat(kind, about, by, *words):
      with site.set(by):
        said = say(kind, about, *words)
      again[id(said)] = said
      held.setdefault((scope(about), "journal"), []).append(said)
      return said

    def answered(f):
      repeat(*f)
      while f[1] in pending and owed() and due[-1][0] == "joined" and due.pop() and due:
        recount(due.pop())

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
        case ("carried", about, _, _, values, definitions, *_):
          repeat(*entry)
          bind(module(about), about, values, definitions)
        case ("cut", about, "journal", turns, lost, cut):
          lost = [n for n in lost if n not in module(about)]
          note = f"The life goes on from a checkpoint. It lost {lost or 'no name'}, and it cut {cut or 'no act'}."
          repeat("cut", about, "journal", [*turns, *[("tell", about, "journal", [note])][: bool(lost + cut)]])
        case (_, about, *_) if first.get(about) is entry:
          answered(entry)
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
          # The answer waits at its place in the record, while the ears of the World catch up on the log as they did
          # when the question was made: the ear that took it and each ear after it do not hear it, and a start that
          # the taker said later is said at its place, so the question is taken with no fact until then.
          due.insert(sum(ranks[id(x)] > ranks[id(f)] for x in due), f)
          if f[2] in outside:
            for x in log:
              x[0] is made and x[1].update([*outside][[*outside].index(f[2]) :])
            offers(made)
          if name not in taken:
            upto(f)
            if owed()[-1] is f or not (name in pending and f[2] in outside):
              del due[next(n for n, x in enumerate(due) if x is f)]
              answered(f)
            else:
              taken.add(name)
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
      n, by = site.get(), heard and again.get(id(heard), 0) is heard and heard[2]
      if by and n not in known:
        heard = (*heard[:2], "journal", *heard[3:])
      w, burst = carried(g, heard), []
      # An ear that hears a fact of another speaker that the journal says again said, in that step, each fact of its
      # own that the record holds next, each joined to the fact before it, but an answer, which its question takes.
      # The journal says them in that step and gives each to the ear, as an ear hears what it says, so the ear does in
      # that step what it did then.
      if by and by != n and n in outside and owed() and due[-1][2] == n:
        while not question(x := due[-1]) and first.get(x[1]) is not x:
          burst.append(repeat(*due.pop()))
          if not due[1:] or due[-1][0] != "joined" or due[-2][2] != n or question(due[-2]):
            break
          due.pop()
      for said in burst:
        for x in log:
          x[0] is said and x[1].add(n)
        w = w is not False and carried(g, (*said[:2], "journal", *said[3:]))
      return w

    def flying():
      for x in [*taken]:
        if not over(x) and any(f[1] == x and f[2] in outside for f in held.get((scope(x), "started"), ())):
          pending[x] = ()
      return [*pending]

    def keeps():
      nonlocal a, last
      try:
        while True:
          while owed() and (
            not log
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
          # The program of a born chain starts with its birth, so each module that a chain makes again, as a fork of a
          # born chain makes its own, takes the bind of the last fact carried of that chain, or of the chain it forks.
          if a and a[0] == "module":
            born = a[1]
            while not transcript(born, "carried") and get(born)[5]:
              born = get(born)[5]
            for x in transcript(born, "carried")[-1:]:
              x[5:] and bind(a[3], a[1], x[4], x[5])
      finally:
        for pile in log, due, first, saved, back:
          pile.clear()

    self.more, self.flying = more, flying
    return numbered, offered, driven, lives, keeps()


def journal(names, record=()):
  return Journal(names, record)


def bind(module, about, values, definitions):
  # The module of a chain at the tip, made again as data: each value from its text, each definition, then each value
  # that a definition bound again, while a value whose object the module still holds stays that object, which a
  # definition took. A value or a definition that raises loses only itself, which the cut of the birth names.
  bound = {}
  for word in [None, *definitions, None]:
    for k, text in values.items() if word is None else ():
      if k not in bound or module.get(k) is not bound[k]:
        try:
          module[k] = bound[k] = eval(text, module)  # noqa: S307
        except Exception:  # noqa: S110
          pass
    try:
      word and eval(compile(word, about, "exec"), module)  # noqa: S307
    except Exception:  # noqa: S110
      pass


def fed(journal, entries):
  def poke():
    return
    yield

  booted(journal, "nothing takes a tail").more(entries)
  journal.names["drive"](poke(), "tail")


def held(journal):
  return booted(journal, "it holds no act").flying()


def booted(journal, so):
  if journal.more is None:
    raise journal.names["Refused"]("no boot took this journal, so " + so)
  return journal


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
  # Each chain is made before any chain holds its prefix, so a chain with a source takes no turn of its source again.
  for c in data["chains"]:
    put("chain", c["id"], operator, c["on"], c["label"], c["source"])
  for c in data["chains"]:
    i = c["id"]
    definitions = [d] if isinstance(d := c["definitions"], str) else d
    kept = [
      t for t in c["turns"] if t[1] not in kinds or home[t[1]] != i or t[0] not in ("started", "done", kinds[t[1]])
    ]
    entries += [(("carried", i, "journal", c["turns"], c["values"], definitions, made[i]),), (("prefix", i, i, kept),)]
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
    entries.append((("cut", i, "journal", c["turns"], c["lost"] + [*c["values"]], cut),))
    word = (
      f"if not get(acting())[5]:\n for __part in {c.get('lives', [])!r}:\n"
      "  try: await rung(__part)\n  except Exception as __raised: debug(t'{__raised!r}')\n"
      " say('cut', scope(acting()), "
      "[x for x in reading() if x[0] != 'tell' or not under(x[1], acting()) or x[3][0].startswith(f'#{acting()} debugged')])\n"
      " del __part"
    )
    c.get("lives") and put("rung", f"rung{numbers['rung'] + 1}", operator, i, word, "", "")
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
      and (
        not question(x)
        or (x[0] == "reply" and x[1] in ended)
        or x[1] in borne
        or (x[0] == "run" and x[4][-5:] == "_told")
      )
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
          *filter(None, [define(d) for x in transcript(c, "carried")[-1:] for d in (x[5:] or [()])[0]]),
          *filter(None, [define(w) for r, w in program(c).items() if get(r) and get(r)[2] != "extensions"]),
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
