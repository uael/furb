import re
from asyncio import CancelledError, get_running_loop
from collections import Counter
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template
from typing import Any, Callable, Generator, Literal, Never  # noqa: UP035

WINDOW = 200000
OPERATOR = "operator"
TIMEOUT = 120.0
LARGEST = 524288
ROOT = "chain1"
site = ContextVar("site", default=OPERATOR)
raised = None
actor = ""
accepts = None
type Show = tuple | Callable[[list[str]], list[int]]
type Filter = Callable[[list[tuple]], list[tuple]]


def ask(kind, on, *words):
  if isinstance(got := peek(a := act(kind, on, None, *words), Refused(a + " not done")), Refused):
    raise got
  return got


def span(lo: int, hi: int) -> Show:
  return "span", lo, hi


def grep(pattern: str) -> Show:
  return "grep", pattern


def differs(old: list[str]) -> Show:
  return "differs", old


def picked(show: Show, lines: list[str]) -> list[int]:
  match show:
    case "span", lo, hi:
      return list(range(1, len(lines) + 1)[lo - (lo > 0) : hi if hi >= 0 else hi + 1 or None])
    case "grep", str() as pattern:
      return [i for i, line in enumerate(lines, 1) if re.search(pattern, line)]
    case "differs", list() as old:
      return [i for i, line in enumerate(lines, 1) if old[i - 1 : i] != [line]]
  return show(lines)  # ty: ignore[call-non-callable]


HEAD, TAIL, HIDDEN = span(1, 2000), span(-250, -1), span(0, 0)


def take(*ids: str, inside=True) -> Filter:
  return lambda facts: [a for a in facts if any(under(a[1], one) for one in ids) == inside]


def read(path: str, show: Show = HEAD, on="") -> Text:
  got = ask("read", on, path)
  if acting() and isinstance(got, Text) and (size := len(shown(got, show).encode())) > LARGEST:
    raise Refused(f"{path} shows {size} bytes, over {LARGEST}")
  show == HIDDEN or tell(on, path, got, show)
  return got


def write(text: Text, on="") -> Text:
  got = ask("write", on, Text(text.path, text.content))
  tell(on, text.path, got, differs(text.lines))
  return got


def turns(on="") -> list[tuple]:
  past, user, sent = [], [], []
  for it in transcript(on):
    match it:
      case ("reply", *_):
        sent += user
        user = []
      case ("done", about, _, [*_] as turn) if question(("reply", about)):
        past += [("user", "\n\n".join(sent), None, None), turn]
        sent = []
      case ("tell", *_, notes):
        user.append("\n".join(notes))
  return [*past, ("user", "\n\n".join(sent + user), None, None)]


def module(on="") -> dict:
  return (transcript(on, "module") or [[{}] * 4])[-1][3]


def program(on="") -> dict[str, str]:
  words = {}
  for x in transcript(on):
    match x:
      case ("module", *_):
        words = {}
      case ("run", *_, which, word, donor):
        words[donor or which] = word
  return words


def literal(v, into=None):
  isinstance(v, str) and get(v) and into is not None and into.add(v)
  match v:
    case Act():
      return f"Act({str(v)!r})" * (into is not None)
    case BaseException():
      return literal(v.args) and repr(v)
    case None | bool() | int() | float() | str() | Text() | Exit():
      return repr(v) * (not (isinstance(v, float) and v - v))
    case dict():
      s = [(literal(k, into), literal(y, into)) for k, y in v.items()]
      return all(p and o for p, o in s) and "{" + ", ".join(f"{p}: {o}" for p, o in s) + "}"
    case list() | tuple() | set():
      s = [literal(y, into) for y in v]
      j = ", ".join(s)
      return all(s) and (
        f"[{j}]"
        if isinstance(v, list)
        else f"({j}{',' * (len(s) == 1)})"
        if isinstance(v, tuple)
        else "{" + j + "}"
        if s
        else "set()"
      )
  return ""


def dump(*names: str, on="") -> str:
  here, out = module(on), {}
  for name in names:
    if name in here and callable(here[name]):
      text = getattr(here[name], "__name__", name) == name and next(
        (w for w in reversed(program(on).values()) if re.search(rf"(?m)^(async )?(def|class) {name}\b", w)), ""
      )
    else:
      text = name in here and (got := literal(here[name])) and f"{name} = {got}"
    if not text:
      raise Refused("no dump of " + name)
    out[text] = name
  return "\n".join(out)


def standing():
  return answer("stand", ROOT, [])


def automatic():
  models = [one for one in (standing()[:1] or [[]])[0] if one[0] != OPERATOR]
  placed = {}
  for entry in models:
    isinstance(place := (entry[4:] or [None])[0], int) and placed.setdefault(place, entry[0])
  return next(iter([placed[k] for k in sorted(placed)] + [two[0] for two in models if two[3:4] == ["cheap"]]), "")


def operated(where):
  return where.split("/")[0] == OPERATOR


def stand(on=""):
  isinstance(got := peek(a := act("stand", on, None)), list) and tell(on, roster=got[0], actor=got[2])
  return a


def clock(on="") -> float:
  return ask("clock", on)


def chance(on="") -> float:
  return ask("chance", on)


def gate(word: str, on="") -> list[str]:
  return ask("gate", on, unquoted(word))


def cd(path: str, on=""):
  path = ask("cd", on, path)
  tell(on, path)
  return path


def cwd(on="") -> str:
  return answer("cd", on, "")


def pause(id: str, why=None):
  if question(("rung", who := acting())) and covers(("pause", id), who):
    raise Refused(f"{who} cannot pause itself through {id}")
  control("pause", "paused", id, why)


def wake(id: str):
  control("wake", "woke", id)


def cancel(id: str, why=None):
  control("cancel", "cancelled", id, why)


def close(value: object, id=""):
  match get(who := acting()):
    case ("rung", _, by, _, _, back, *_):
      if back:
        raise CancelledError()
      id = id or by * prompted(by)
  match get(id := id or who):
    case ("prompt", _, _, on, shape, *_) if not isinstance(value, BaseException):
      try:
        s = eval(shape, module(on)) or object
        assert any(
          value in [a for a in x.__args__ if type(a) is type(value)]
          if (origin := getattr(x, "__origin__", x)) is Literal
          else isinstance(value, origin)
          for x in (s.__args__ if isinstance(s, type(int | str)) else [s])
        )
      except Exception:
        raise Refused(repr(value) + " not " + shape) from None
  control("close", "closed", id, value)
  if under(who, id):
    raise CancelledError()


def debug(template: Template):
  text = "".join(
    x if isinstance(x, str) else format(("%" + x.conversion) % (x.value,) if x.conversion else x.value, x.format_spec)
    for x in template
  )
  (who := acting()) and tells(who) and text and say(*told(who, "debugged", debug=text))


def wait(seconds: float = 0, *acts: str, on="") -> Act[str | None]:
  if not acts:
    raise Refused("no act: await the act, or wait(seconds, act)")

  def ear(id):
    while not (got := list(filter(over, acts))):
      yield {id, *acts}
    yield "done", id, got[0]

  return act("wait", on, controlled(ear), seconds, *acts)


def rung(word="", retells="", to="", on="") -> Act:
  def ear(id):
    yield "started", id
    mine = {retells, scope(id)}
    if word:
      while (c := crafted(id)) and c[0] not in lineage(id):
        yield {"rung"}
      if to:
        act("reply", "", lambda r: (yield "done", r, ("assistant", word, None, None)), to)
      elif tells(id):
        yield "tell", id, [word]
      while not over(retells) and get(retells) if retells else paused(get(id)[2]):
        yield mine
      yield "ready", id, word
    while True:
      match (yield mine):
        case ("done", about, _, value) if question(("reply", about)) and get(about)[2] == id and not word:
          yield ("done", id, value) if isinstance(value, BaseException) else ("ready", id, value[1])
        case ("wants", about, by, _, call) if get(by)[4] == id:
          yield "started", about
          while not over(call):
            yield {call}
          yield "done", about, peek(call)
        case ("done", about, _, value) if question(("run", about)) and get(about)[4] == id:
          if isinstance(value, CancelledError):
            if retells:
              value = None
            elif tells(id):
              yield told(id, "cancelled")
          elif isinstance(value, BaseException):
            module()["raised"] = value
            if tells(id):
              yield told(id, "raised", raised=value)
          yield "done", id, value

  return act("rung", on, controlled(ear), word, retells, to if word or to else module(on).get("actor", ""))


def prompt[T](shape: type[T] | object, message="", to="", on="") -> Act[T]:
  named = (
    shape if isinstance(shape, str) else re.sub(r"""<class '|'>|('[^']*'|"[^"]*")|[\w.:/]*\.""", r"\1", repr(shape))
  )
  thread = on * prompted(on)
  to = to or (thread and "".join(get(thread)[6:]))
  asks = (to or module(on).get("actor")) != OPERATOR
  says = asks and thread and named == "None" and not over(thread)

  def ear(id):
    if asks:
      yield "started", id
      while crafted(id):
        yield {"rung"}
    yield opened(id, named, message=message, thread=thread)
    a: tuple = ()
    while not says:
      got = a[:1] == ("done",) and get(a[1])
      heard = got and prompted(a[1]) and got[3:5] == (id, "None")
      heard or (
        got
        and got[2] == id
        and (
          owed := (
            "",
            *[
              x[1] * operated(x[2])
              for x in transcript(id)
              if (x[3:5] == (id, "None") and operated(x[2]))
              or (get(x[1]) is x and prompted(x[1]) and x[6] == OPERATOR and under(x[2], id))
              or (x[:3:2] == ("rung", id) and not over(x[1]))
            ],
          )[-1]
        )
        and prompt(None, owed + " unanswered", on=id)
      )
      asks and not (
        paused(id)
        or any(
          id in x[2:4] and not over(x[1]) and not (heard and any(y[2] == x[1] for y in transcript(id, "reply")))
          for x in transcript(id, "rung")
        )
      ) and rung(to=to)
      a = yield {id}
    yield "done", id, None

  return act("prompt", on, controlled(ear), named, message, to)


def chain(label="", source="", filter: Filter | list[str] | None = None, on="") -> Act[Never]:
  if scope(source) != source:
    raise Refused("no chain " + source)
  kept = (
    filter
    if filter is None or isinstance(filter, list)
    else [one[1] for one in filter([x for x in transcript(source or ROOT) if question(x)])]
  )

  def ear(id):
    t = transcript(o := source or ROOT)
    h = [(x[0] == "tell" and x[3][0].splitlines()[0]) or x[0] for x in t] + ["chain()", "#" + o, "setup"]
    hi = h.index("#" + o, lo := h.index("chain()"))
    hi *= hi < len(t)
    blocked, running, waiting, last, heard = set(), {}, {}, standing(), []
    step = news = ran = reply = note = to = hush = dull = stuck = ""
    stood = module(source or get(id)[3]).get("actor", last and last[2])

    def placed(went):
      return rung("stand()" + (went and "\ncd(" + repr(went) + ")"), to=stood)

    def bind(whose):
      w = "\n\n".join(heard)
      heard.clear()
      return w and act("run", id, None, n := whose + "_told", unquoted(ladder.setdefault(n, w)), "")

    def replay(of="", words=""):
      fresh = not of
      for whose, said in list(ladder.items()):
        if under(whose, of):
          if (words + "\n").startswith(said + "\n"):
            words = words[len(said) + 1 :]
          else:
            fresh |= whose not in blocked
            del ladder[whose]
            cancel(whose)
      if fresh:
        yield "module", id, {**globals(), "__name__": id, "actor": stood}
        for whose, said in ladder.items():
          whose in blocked or (said and rung(said, whose))
      if words:
        with site.set(of):
          rung(words)

    if source or t[lo:hi]:
      kin = {
        source,
        id,
        *[n for one in ([x[1] for x in t if question(x)] if kept is None else kept) for n in lineage(one)],
      }
      later = {x[1] for x in t[h.index("setup") :] if question(x) and (not get(x[2]) or x[:3:2] == ("rung", ROOT))}
      later |= {x[1] for x in t if x[0] == "run" and not later.isdisjoint(lineage(x[4]))}
      yield (
        "prefix",
        id,
        [
          it
          for n, it in enumerate(t)
          if (lo <= n < hi and later.isdisjoint(lineage(it[1])))
          or (source and (not (b := get(it[1])) or (b[2] if b[0] == "reply" else it[1]) in kin))
        ],
      )
    ladder = {x: w for x, w in program(source or id).items() if over(x) or not get(x)}
    yield "started", id

    def opening():
      return opened(
        id,
        "Never",
        label=label,
        source=source,
        actor=stood * (not source),
        actor_via=(not source) * (get(id)[3] or "stand") or None,
      )

    top = opening()
    if last and (source or not last[3:]):
      yield top
      top = ""
    yield from replay()
    seat = top and last and placed(last[1])
    while True:
      r = get((a := (yield {id, "stand", "reply"}))[1]) or a
      match a:
        case ("prefix", _, by, _) if by != id:
          top = ""
        case ("done", *_) if r[0] == "stand" and standing() != last:
          was, last = last, standing()
          if last[2:3] != was[2:3]:
            stood = module()["actor"] = last[2]
            top = top and opening()
          if not was:
            yield ("tell", id, ["chain()"]) if last[3:] else top
            top *= bool(last[3:])
            if last[4:] and id == ROOT and not transcript(id, "reply") and not transcript(id, "carried"):
              yield from seeded(id, last, stood)
          elif last[3:] and not top:
            placed(last[1] * (was[1] != last[1]) * all(all(get(x[2])[4::2]) for x in transcript(id, "cd")))
        case ("done", about, _, value) if running.get(maker := r[2]) in (
          False,
          isinstance(value, Exception) and r[0] != "reply" and "",
        ) and repr(about) not in str(
          transcript(id, "wants") + [x[1] for x in transcript(id, "close") if scope(x[2]) == id != x[2]]
        ):
          scope(about) != id and (yield "tell", maker, [headed(about, "done", value=value)])
          dull = stuck = ""
          value is None or isinstance(value, CancelledError) or (
            news := (about + " done", maker, isinstance(value, Exception))
          )
        case ("done", _, by, Refused() as why) if (
          r[0] == "reply"
          and r[3] != id
          and (paused(r[3]) or not by)
          and (maker := (get(p := get(r[2])[2]) or a)[2]) in running
        ):
          yield "tell", maker, [headed(p, "paused", why=why)]
          running[maker] or (news := (p + " paused", maker, 0))
      if scope(a[1]) != id:
        continue
      if a[0] in ("ready", "done"):
        waiting.pop(a[1], None)
      if a[0] == "done" and (
        r[0] == "wants"
        or (
          question(("rung", m := r[2]))
          and not running.get(m)
          and not (get(m)[4] or isinstance(a[3], CancelledError | None | tuple))
        )
      ):
        hush = dull = stuck = ""
      if a[0] == "done" and prompted(r[2]) and a[1] in (ran, *blocked) and not over(r[2]):
        if hush == r[2] or (stuck := dull):
          ran = yield "tell", get(r[2])[2], [headed(r[2], "asked no more until a fact comes")]
        dull = 1
      match a:
        case ("read", about, by, _, path) if prompted(path) and scope(path) == id:
          yield "done", about, Text(path, "\n".join(w for x, w in ladder.items() if x != by and under(x, path)))
        case ("write", about, by, _, Text(path, content) as text) if prompted(path) and scope(path) == id:
          under(by, path) and ladder.pop(by, None)
          yield from replay(path, content.rstrip("\n"))
          yield "done", about, text
        case ("read", about, *_, path) if (door := re.fullmatch(r"(bash\d+)/(stdout|stderr)", path)) and get(door[1]):
          yield (
            "done",
            about,
            getattr(
              peek(door[1]),
              door[2],
              Text(path, "".join(x[3] for x in transcript(door[1], "out") if x[1::3] == door.groups())),
            ),
          )
        case ("done", *_) if top and r[0] == "rung" and not crafted(id):
          if seat:
            yield top
            top = ""
          else:
            seat = placed(last[1])
        case ("rung", rid, by, _, "", *_):
          waiting[rid] = running[rid] = ""
          prompted(by) or (hush := "")
        case ("tell", about, by, notes) if notes not in (["chain()"], [*r[4:5]]):
          heard.append("\n".join(notes))
          if a is note:
            note = bind(step)
          elif not (
            a is ran
            or (not over(about) and question(("rung", r[2])))
            or r[0] == "write"
            or (
              prompted(by) and prompted(get(by)[2]) and [x[5] for x in transcript(id, "prompt")].count(get(by)[5]) > 1
            )
          ):
            hush = ""
            by == id or (dull := (stuck := ""))
          ran = ""
        case ("done", about, *_) if about == note and step in waiting:
          with site.set(step):
            reply = act("reply", id, controlled(idle), to)
        case ("ready", rid, _, word):
          try:
            rid == step or bind(rid)
            ladder[(donor := r[5]) or rid] = word
            found = (ran := tells(rid) and rid) and gate(word)
            if not found:
              running[rid] = act("run", id, None, rid, unquoted(word), donor)
          except Refused as e:
            found = [str(e)]
          if found:
            yield told(rid, "refused", findings="\n".join(found))
            blocked.add(rid)
            close(Refused(*found), rid)
        case ("done", _, "", Refused() as why) if r[0] == "reply":
          pause(id, why)
      if r[0] == "run":
        running[r[4]] = a[0] != "done"
      elif r[0] == "wants":
        running[get(r[2])[4]] = a[0] != "started"
      if any(running.values()):
        continue
      waits = [c for *_, c in transcript(id, "wants") if not over(c)]
      if (
        (step not in waiting or paused(step))
        and not stuck
        and (
          (
            got := next(
              (
                x
                for x in waiting
                if not (
                  paused(x)
                  or over(x)
                  or x in {y[2] for y in transcript(id, "reply") if not over(y[1])}
                  or get(x)[2] == hush
                )
              ),
              "",
            )
          ),
          step := got or step * (step in waiting),
        )[0]
      ):
        maker, to = get(step)[2::4]
        if offered(last[0], to) is None:
          close(Refused(to + " no actor"), maker * prompted(maker) or step)
        else:
          hush = maker
          note = yield told(step, "advance on " + maker, share=filled(reply) or None, waits=waits or None)
          news = ""
      if news and not waiting and (r[2] in running or over(r[2]) or a[0] == "tell"):
        ours = get(news[1])[2]
        prompted(ours) and (news[2] or not over(ours)) and prompt(None, news[0], on=ours)
        news = ""

  return act("chain", on, ear, label, source, *[kept] * (kept is not None))


def seeded(id, last, to):
  rung('debug(t"{standing()[4]=}")', to=to)
  rung(f"read({last[3]!r})", to=to)
  for k in 0, 1:
    k and (yield "setup", id)
    for x in last[4]:
      x[1] and x[3] == k and rung(f"rung(read({x[1]!r}, HIDDEN).content)", to=to)
    for y in last[4]:
      y[3] == k and y[2] and rung(y[2], to=to)


HEARD = (
  "say('prefix', scope(acting()), [*transcript(of='module')[-1:], *transcript(of='carried')[-1:], "
  "*transcript(of='carried')[-1][3], *transcript(of='stand')[-1:], *transcript(of='pause'), "
  "*[x for x in transcript() if acting() in x[1::3] and x[0] != 'tell']])"
)
KEPT = (
  "say('prefix', scope(acting()), [x for x in transcript() if x[0] != 'tell' or x in transcript(of='carried')[-1][3]])"
)


def seed(data):
  entries, numbers = [], Counter()

  def put(sort, called, *words):
    last = int(called[len(sort) :])
    entries.extend(((sort, f"{sort}{n}", ROOT, ""),) for n in range(numbers[sort] + 1, last))
    numbers[sort] = max(numbers[sort], last - 1) + bool(words)
    words and entries.append(((sort, called, *words),))

  asking = {a["on"] for a in data["acts"] if "value" not in a}
  for c in data["chains"]:
    put("chain", c["id"], OPERATOR, c["on"], c["label"], "")
    entries.append(
      (("carried", c["id"], "journal", c["turns"], {k: t for k, t in c["values"].items() if t[LARGEST // 16 :]}),)
    )
    (c["paused"] or c["id"] in asking) and entries.append((("pause", c["id"], OPERATOR),))
  for x in data["acts"]:
    "value" in x and (
      put(x["kind"], i := x["id"], "journal", x["on"]),
      entries.extend([(("started", i, "journal"),), (("done", i, "journal", x["value"]),)]),
    )
  for x in data["acts"]:
    "value" in x or (
      put("prompt", x["id"], OPERATOR, x["on"], x["shape"], x["message"], x["to"]),
      x["held"] and entries.append((("started", x["id"], x["held"]),)),
    )
  for kind, most in data["taken"].items():
    put(kind, f"{kind}{most + 1}")
  for c in data["chains"]:
    i, cut = c["id"], c["cut"] + [y["rung"] for y in c.get("again", ())]
    loud = bool(c["lost"] or cut)
    note = [
      f'debug(t"The life goes on from a checkpoint. It lost {c["lost"] or "no name"}, and it cut {cut or "no act"}.")'
    ]
    word = "\n".join(
      [
        HEARD,
        c["definitions"],
        *(
          f"{named} = " + (f"eval(transcript(of='carried')[-1][4][{named!r}])" if text[LARGEST // 16 :] else text)
          for named, text in c["values"].items()
        ),
        *(f"await rung({one!r})" for one in c.get("lives", ())),
        *note[loud:],
        KEPT,
        *note[:loud],
      ]
    )
    seat = f"rung{numbers['rung'] + 1}"
    for maker, said in [(OPERATOR, word), *((seat, one) for one in c.get("lives", ()))]:
      put("rung", f"rung{numbers['rung'] + 1}", maker, i, said, "", "")
      put("gate", gated := f"gate{numbers['gate'] + 1}", i, i, said)
      entries.append((("done", gated, "gate", []),))
  return entries + [(("wake", y["id"], OPERATOR),) for y in data["chains"] if y["id"] in asking and not y["paused"]]


def gathered(define, floor):
  def number(called):
    return int(re.sub(r"^\D+", "", called) or 0)

  chains = [x for x in listening() if question(("chain", x))]
  asked = [*map(get, chains)] + [x for h in chains for x in transcript(h) if x is get(x[1])]
  cut = sorted((x[1] for x in asked if x[0] == "run" and not over(x[1]) and get(x[4])), key=number)
  gone = {x[4] for x in asked if x[0] == "wants" and x[2] in cut and prompted(x[4]) and not over(x[1])}
  asks = [x[1] for x in asked if x[0] == "prompt" and not over(x[1]) and x[1] not in gone]
  values, lost, named = {}, {}, set(asks)
  for c in chains:
    here, values[c], lost[c] = module(c), {}, []
    for key, value in here.items():
      if not (key.startswith("__") or callable(value) or globals().get(key, here) is value):
        found = set()
        try:
          text = literal(value, found)
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
  verbs = ("prompt", "bash", "read", "write", "wait", "cd", "chance")
  words = {y[2] for y in (standing()[4:5] or [()])[0]}
  done = {x[1] for x in asked if x[0] == "bash"}
  borne = sorted((x for x in named - gone | done if get(x)[0] in verbs), key=lambda r: (get(r)[0], number(r)))
  stands = [x[1] for x in transcript(ROOT, "stand") if over(x[1])][-1:]
  ticks = [x[1] for x in transcript(ROOT, "clock") if isinstance(peek(x[1]), float)][-1:]
  acts = []
  for one in [*stands, *ticks, *borne]:
    a = get(one)
    d = {"kind": a[0], "id": one, "on": a[3]}
    if prompted(one) and not over(one):
      held = next((x[2] for x in transcript(scope(one), "started") if x[1] == one and get(x[2]) is None), "")
      acts.append({**d, "shape": a[4], "message": a[5], "to": a[6], "held": held})
    elif over(one):
      literal(peek(one), set()) and acts.append({**d, "value": peek(one)})
    else:
      cut.append(one)
      acts.append({**d, "value": Refused(one + " was interrupted: the life that ran it ended before it did")})
  out = []
  for c in chains:
    a = get(c)
    saved = [
      x
      for x in transcript(c)
      if x[0] not in ("module", "carried", "made")
      and literal(x, set())
      and (not question(x) or x[0] == "reply" or x[1] in borne)
    ]
    held = {*chains, *borne, *(x[1] for x in saved if question(x))}
    out.append(
      {
        "id": c,
        "on": a[3] if a[3] in chains else "",
        "label": a[4],
        "source": a[5],
        "paused": paused(c),
        "turns": [x for x in saved if x[0] not in ("started", "done") or x[1] in held],
        "definitions": "\n\n".join(
          filter(None, [define(w) for r, w in program(c).items() if (get(r) or [0] * 3)[2] != "extensions"])
        ),
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


def grant(usd: float | None = None, share: float | None = None, on="") -> Act[None]:
  def ear(id):
    here = scope(id)
    if not ((usd or share) and (usd or 0) >= 0 <= (share or 0) <= 1):
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    yield "started", id
    spent = 0
    for x in transcript(here, "grant"):
      x[3] != here or x[1] == id or close(None, x[1])
    yield opened(id, "None", usd=usd, share=share)
    while True:
      match (yield {id, here}):
        case ("done", about, _, (_, _, (*_, cost), _)) if question(("reply", about)):
          spent += cost
          yield told(id, "ledger", spent=spent)
          if (usd and spent >= usd) or (share and (filled(about) or 0) >= share):
            pause(here, "ceiling")

  return act("grant", on, controlled(ear), usd, share)


def bash(
  command: str, stdin: str | None = "", timeout=TIMEOUT, ok=None, show: Show = TAIL, show_err: Show | None = None, on=""
) -> Act[Exit]:
  if any(map(callable, (ok, show, show_err))):
    raise Refused("no callable")

  def ear(id):
    yield opened(id, "Exit", command="" if show == HIDDEN else command)
    while True:
      match (yield {id, "merged"}):
        case ("merged", qid, *_, about) if about == id:
          yield "done", qid, show_err is None
        case ("done", about, _, got) if about == id:
          match got:
            case Exit(code, out, err) if show != HIDDEN:
              yield told(
                id,
                f"timed out after {timeout}s" if code is None else f"exited {code}",
                stdout=cut(shown(out, show or TAIL)),
                stderr=show_err and cut(shown(err, show_err)),
              )
          return

  def cut(text, most=LARGEST // 16):
    return text if len(text) <= 2 * most else text[:most] + f"\n... ({len(text)} chars)\n" + text[-most:]

  return act(
    "bash",
    on,
    controlled(ear, True),
    command,
    stdin,
    timeout,
    ok,
    *(show, show_err) * ((show, show_err) != (TAIL, None)),
  )


@dataclass
class Text:
  path: str
  content: str = ""
  before = None

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()

  def edit(self, lo: int, hi: int, text: str) -> Text:
    now = (self.content + "\n").splitlines(True)[: len(self.lines)]
    if not 1 <= lo <= hi + 1 <= len(now) + 1:
      raise Refused(f"{self.path} no lines {lo}:{hi}")
    now[lo - 1 : hi] = [text and text.removesuffix("\n") + "\n"]
    return self.replace(self.content, "".join(now))

  def replace(self, old: str, new: str, count=-1) -> Text:
    if old not in self.content:
      raise Refused(self.path + " no " + repr(min(old.splitlines(), key=self.content.count)))
    (made := Text(self.path, self.content.replace(old, new, count))).before = self
    return made

  def undo(self, n=1) -> Text:
    for _ in range(n):
      self = self.before or self  # noqa: PLW0642
    return self

  def append(self, text: str) -> Text:
    return self.insert(len(self.lines) + 1, text)

  def insert(self, line: int, text: str) -> Text:
    return self.edit(line, line - 1, text)

  def delete(self, lo: int, hi: int) -> Text:
    return self.edit(lo, hi, "")


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Act[T = object](str):
  def __await__(self) -> Generator[object, Any, T]:
    if (acting() and question(("chain", self))) or not get(self):
      raise Refused(self + " never settles")
    if who := acting():
      if not over(self) and (self == who or (question(("run", self)) and get(self)[4] == who)):
        raise Refused(f"{who} cannot await its own completion through {self}")
      return (yield self)
    f = get_running_loop().create_future()

    def pend():
      while not over(self):
        yield
      (f.set_exception if isinstance(got := peek(self), BaseException) else f.set_result)(got)

    drive(g := pend(), self + " waits " + str(id(g)))
    try:
      return (yield from f)
    finally:
      g.close()


class Refused(Exception): ...


class Drift(Exception): ...


def lineage(name):
  yield name
  while a := get(name):
    yield (name := a[2])


def under(name, of):
  return "" != of in lineage(name)


def acting():
  return site.get() * bool(get(site.get()))


def prompted(name):
  return question(("prompt", name))


def question(a):
  return bool(re.fullmatch(a[0] + r"\d+", a[1]))


def scope(name):
  while (a := get(name)) and a[0] != "chain":
    name = a[3]
  return name * bool(a)


def tell(on, path="", got=None, show=HEAD, **words):
  (who := acting()) and tells(who) and say(
    *told(
      transcript(on, who)[-1][1],
      path=path,
      **words,
      **{
        "numbers": at if (at := picked(show, got.lines)) and at[-1] - at[0] >= len(at) else None,
        "text": shown(got, show),
      }
      if isinstance(got, Text)
      else {"value": got},
    )
  )


def answer(kind, on, empty):
  return next((got for x in reversed(transcript(on, kind)) if isinstance(got := peek(x[1]), type(empty))), empty)


def filled(about):
  match peek(about):
    case (_, _, (tokens, *_), _):
      return round(tokens / (offered(standing()[0], get(about)[4]) or WINDOW), 2)


def over(id):
  return peek(id, ...) is not ...


def tells(id):
  return get(id)[2] != scope(id) or all(get(id)[4::2])


def crafted(id):
  return [x[1] for x in transcript(id, "rung") if all(x[4::2]) and not over(x[1])]


def told(id, what="", *notes, **words):
  return "tell", id, [headed(id, what, **words), *notes]


def control(kind, name, id, *words):
  if not get(id):
    raise Refused("no act " + repr(id))
  over(id) or scope(acting()) == scope(id) or (kind == "wake" and not paused(id)) or say(
    *told(id, name + " by " + site.get(), value=(*words, None)[0])
  )
  return say(kind, id, *words)


def headed(id, what="", **words):
  text = f"#{id} {what}".rstrip()
  for word, value in words.items():
    name = id + "_" + word
    if isinstance(value, str) and "\n" in value:
      while "</s:" + name + ">\n" in value + "\n":
        name += "_"
      text += f"\n<s:{name}>\n{value}</s:{name}>"
    elif value not in (None, ""):
      text += f"\n{name} = " + re.sub(r"^[\w.]+\.(?=[A-Z]\w*\()", "", repr(value))
  return text


def opened(id, of, **words):
  return told(id, "", id + ": Act[" + of + "] = Act(" + repr(id) + ")", **words)


def shown(text, show):
  lines = text.lines
  return "\n".join(x[:1000] + f"... ({len(x)} chars)" if (x := lines[i - 1])[1000:] else x for i in picked(show, lines))


def unquoted(word):
  made, at, odd = [], 0, set()
  for o in re.finditer(r"(?m)^<s:(\w+)>\n?", word):
    if o.start() >= at and o[1] not in odd:
      if m := re.compile("(?m)</s:" + o[1] + ">$").search(word, o.end()):
        made += [
          word[at : o.start()],
          o[1] + " = " + repr(word[o.end() : m.start()]) + "\n" * word.count("\n", o.start(), m.end()),
        ]
        at = m.end()
      else:
        odd.add(o[1])
  return "".join(made) + word[at:]


def offered(roster, to):
  return next((x[2] for x in roster if to in (x[0], *[x[0] + "/" + e for e in x[1]])), None)


def covers(a, id):
  kind, of, *_ = a

  def on(name):
    while name != of and prompted(name):
      name = get(name)[3]
    return name == of

  if kind == "close":
    if question(("reply", id)):
      id = get(id)[2]
    return on(id) or (question(("rung", id)) and on(get(id)[2]))
  return any(map(on, lineage(id))) or scope(id) == of


def paused(id):
  return next((x[0] == "pause" for x in reversed(transcript(id, "pause")) if covers(x, id)), False)


def idle(id):
  while True:
    yield {id}


def lives(g, a):
  try:
    while isinstance(a := g.send(a), tuple):
      a = say(*a)
  except StopIteration:
    return False
  return a


def controlled(ear, lasting=False):
  def lived(id):
    g, later, halt = ear(id), [], paused(id)
    w = lives(g, None)
    while w is not False:
      a = later.pop(0) if later and not halt else (yield w)
      if over(id) and not lasting:
        return
      match a:
        case ("pause" | "wake", *_) if covers(a, id) and (halt := a[0] == "pause"):
          pass
        case ("cancel" | "close", _, by, *_) if covers(a, id) and not (over(id) or (halt and under(by, id))):
          yield (
            "done",
            id,
            CancelledError(*filter(None, a[3:])) if a[0] == "cancel" else a[3] if a[1] == id else CancelledError(),
          )
        case _ if halt and get(a[1]) is not a:
          later.append(a)
        case _:
          w = lives(g, a)

  return lived


def boot(record=(), **outside):
  global say, act, drive, get, peek, transcript, listening
  get_running_loop()
  made, known, done, held, log, alive, born, busy, left, heed = Counter(), {}, {}, {}, [], {}, {}, set(), [], {}
  first, owned, taken = {e[1]: e for (e,) in record[::-1] if e[0] in ("started", "done")}, set(), {}
  due = [e for (e,) in record[::-1] if first.get(e[1]) is not e]
  ranks = {id(e): n for n, (e,) in enumerate(record)}
  saved, again = {e[1]: e for (e,) in record if question(e)}, {}
  schemas = [(*e[5:6], None)[0] for (e,) in record if e[0] == "made"]
  stable = not schemas or tuple(map(int, str(schemas[-1] or "0.0").split("."))) >= (1, 1)
  top, asked, strays, back = Counter(), {}, {}, []
  for quest in saved.values():
    top[quest[0]] = max(top[quest[0]], int(quest[1][len(quest[0]) :]))
    asked.setdefault(quest[:3:2], []).append((quest[1], quest[3:]))

  def drift(name, fact=None):
    ours, name = fact or get(name), strays.get(name) or name
    return Drift(
      f"{name} drifts: the life made {ours or 'no act'!s:.300}, the record holds {(fact and first[fact[1]]) or saved.get(name)!r:.300}"
    )

  def matched(sort, maker, *said):
    ahead = asked.get((sort, maker), [])
    at = next((n for n, x in enumerate(ahead) if x[1] == said), None)
    if at is None:
      top[sort] += 1
      strays[name := sort + str(top[sort])] = ahead[0][0] if ahead else None
      return name
    return ahead.pop(at)[0]

  def owed():
    while due and question(due[-1]) and due[-1][1] in known:
      due.pop()
    return due

  def repeat(kind, about, by, *words):
    with site.set(by):
      a = say(kind, about, *words)
    again[id(a)] = a
    held.setdefault((scope(about), "journal"), []).append(a)

  def recount(entry):
    match entry:
      case (kind, name, by, on, *words) if question((kind, name)):
        if (
          by in saved
          or (name not in first and re.fullmatch(r"\w+?\d+", by))
          or (on and not get(on))
          or (by == "journal" and not holds(scope(on), [known.setdefault(name, (kind, name, by, on))]))
        ):
          born.setdefault(kind, {})[name] = name
          return
        if by not in outside and (
          not callable(verb := (module(on) or globals()).get(kind)) or re.fullmatch(r"\w+?\d+", by)
        ):
          raise drift(name)
        with site.set(by):
          try:
            act(kind, on, None, *words) if by in outside else verb(*words, on=on)
          except Refused:
            pass
      case (_, about, *_) if scope(about):
        repeat(*entry)
      case (_, about, by, *_) if (
        stable and by in outside and about in saved and get(saved[about][3]) and not get(about)
      ):
        raise drift(about)

  def journal(a=(), last=None):
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
              for name in [x for x in taken if taken[x] and covers(a, x) and not over(x)]:
                del taken[name]
                offers(known[name])
    finally:
      for pile in log, left, due, first, saved, back:
        pile.clear()

  def say(kind, about, *words):
    a = (kind, about, site.get(), *words)
    if about in known and kind in ("started", "done"):
      taken.setdefault(about, False)
      kind == "done" and done.setdefault(about, words[0])
    kind == "prefix" and [held[x].clear() for x in held if x[0] == about]
    holds(scope(about), words[0] if kind == "prefix" else [a])
    log.append((a, ()))
    dispatch()
    return a

  def act(kind, on, ear, *words):
    speaker = site.get()
    made[speaker, kind] += 1
    by = (question(("rung", speaker)) and known[speaker][5]) or speaker
    on = on or scope(speaker)
    if not on and kind != "chain":
      raise Refused("no chain for " + kind)
    if by and not (get(by) or by in outside or callable((module(on) or globals()).get(kind))):
      raise Refused(f"no verb {kind} on {on}, so a later life cannot make the act again")
    names, key = born.setdefault(kind, {}), (by, made[speaker, kind])
    if key not in names:
      names[key] = matched(kind, by, on, *words) if stable else kind + str(len(names) + 1)
    a = (kind, name := names[key], by, on, *words)
    if known.setdefault(name, a) is a:
      holds(scope(on), [a])
      log.append((a, heard := set(outside) - busy))
      busy.add(id(a))
      try:
        for n, g in ears(a):
          ear or n in heard or n == "journal" or n in busy or name in taken or hear(n, g, a) or heard.add(n)
        if ear:
          heard.add(name)
          drive(ear(name), name)
        if saved.get(name, a) == a:
          if name not in taken and (f := first.get(name)):
            taken[name] = f[0] == "started"
            while owed() and not question(due[-1]) and ranks[id(due[-1])] < ranks[id(f)]:
              recount(due.pop())
            repeat(*f)
            while taken[name] and owed() and due[-1][0] == "joined" and due.pop() and due:
              recount(due.pop())
          if kind in ("run", "chain") or not owed() or strays.get(name, name) is None:
            offers(a)
        if name not in taken and not busy & set(outside) & set(dict(ears(a))):
          with site.set(""):
            say("done", name, Refused("nothing takes " + kind))
      finally:
        busy.discard(id(a))
      dispatch()
    return Act(name)

  def offers(a):
    for n, g in outside.items():
      (a[1] in taken and a[0] != "chain") or hear(n, g, a)

  def ears(a):
    q = get(a[1]) or a[:1] * 4
    k = {a[0], *q[:4], *map(scope, q[2:4])}
    return sorted(
      (x for x in alive.items() if k & (heed.get(x[0]) or k) or a[0] in ("pause", "wake", "cancel", "close")),
      key=lambda x: (x[0] not in known, x[0] in outside),
    )

  def holds(on, facts):
    for x in on and facts:
      for k in {"", x[0].replace("wake", "pause"), x[2]}:
        held.setdefault((on, k), []).append(x)

  def dispatch():
    while not busy:
      if left:
        name, g, a = left.pop(0)
        hear(name, g, a)
      elif log:
        a, heard = log.pop(0)
        left.extend((*x, a) for x in ears(a) if x[0] not in heard)
      elif ((g := alive.get("journal")) and hear("journal", g, None)) or not log:
        return

  def hear(name, g, a):
    if name not in busy and alive.get(name) is g:
      with site.set(name):
        busy.add(name)
        try:
          heed[name] = w = lives(
            g, (*a[:2], "journal", *a[3:]) if again.get(id(a), 0) is a and a[2] and name not in known else a
          )
          w is False and alive.pop(name)
        except (Exception, CancelledError) as fault:
          if name == "journal" or isinstance(fault, Drift):
            raise
          alive.pop(name)
          stop = isinstance(fault, CancelledError)
          if not (stop and over(name)):
            shut = [x[3] for x in transcript(name, "close") if stop and x[1] == name]
            say(
              "done" if get(name) and not over(name) else "dropped",
              name,
              (*shut, Refused(f"{name} raised {fault!r}"))[0],
            )
        finally:
          busy.discard(name)

  def drive(g, name):
    if name in alive:
      raise Refused(name + " hears")
    acting() and owned.add(name)
    alive[name] = g
    hear(name, g, None)
    dispatch()

  def listening():
    return alive

  def get(about):
    return known.get(about)

  def peek(at, waiting=None):
    return done.get(at, waiting)

  def transcript(on="", of=""):
    return held.get((scope(on or site.get()), of), [])[:]

  for who, listener in [(OPERATOR, idle(OPERATOR)), *outside.items(), ("journal", journal())]:
    drive(listener, who)
  root = Act(ROOT) if known else chain("root")
  stand(ROOT)
  return root
