import re
from asyncio import CancelledError, current_task, get_running_loop
from collections import Counter
from collections.abc import Callable, Generator
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template
from typing import Any

WINDOW = 200000
OPERATOR = "operator"
TIMEOUT = 600.0
ROOT = "chain1"
site = ContextVar("site", default=OPERATOR)
raised = None
type Show = Callable[[list[str]], list[int]]
type Filter = Callable[[list[tuple]], list[tuple]]


def say(kind, about, *words):
  raise RuntimeError("no life")


def act(kind, on, ear, *words):
  raise RuntimeError("no life")


def drive(g, name):
  raise RuntimeError("no life")


def get(about: str) -> tuple:
  raise RuntimeError("no life")


def peek(at: str, waiting: object = None) -> object:
  raise RuntimeError("no life")


def transcript(on: str = "") -> list[tuple]:
  raise RuntimeError("no life")


def ask(kind, on, *words):
  if isinstance(got := peek(a := act(kind, on, None, *words), Refused(f"{a} not done")), Refused):
    raise got
  return got


def span(lo: int, hi: int) -> Show:
  def picks(lines):
    i, j = (x + len(lines) + 1 if x < 0 else x for x in (lo, hi))
    return list(range(max(i, 1), min(j, len(lines)) + 1))

  return picks


def grep(pattern: str) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if re.search(pattern, line)]


def differs(old: list[str]) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if old[i - 1 : i] != [line]]


HEAD, TAIL, HIDDEN = span(1, 2000), span(-250, -1), span(0, 0)


def take(*ids: str, inside: bool = True) -> Filter:
  return lambda facts: [a for a in facts if any(under(a[1], one) for one in ids) == inside]


def read(path: str, show: Show = HEAD, on: str = "") -> Text:
  got = ask("read", on, path)
  if show is not HIDDEN:
    tell("read", path, *showing(got, show))
  return got


def write(text: Text, on: str = "") -> Text:
  got = ask("write", on, Text(text.path, text.content))
  if not isinstance(got, Text) or got.lines != text.lines:
    tell("write", text.path, *showing(got, differs(text.lines)))
  return got


def turns(on: str = "") -> list[tuple]:
  seen, folded, user, cut = {}, [], [], 0
  for it in transcript(on):
    match it:
      case ("reply", *_):
        cut = len(user)
      case ("done", about, _, (_, _, _, _) as turn) if question(("reply", about)):
        folded += [("user", "\n\n".join(user[:cut]), None, None), turn]
        del user[:cut]
      case (kind, *_, notes) if kind in ("tell", "pause", "wake", "cancel", "close") and notes:
        user.append("\n".join([shown(x, seen) for x in notes]))
  return [*folded, ("user", "\n\n".join(user), None, None)]


def module(on: str = "") -> dict:
  for x in reversed(transcript(on)):
    match x:
      case ("module", _, _, carried):
        return carried
  return {}


def program(on: str = "") -> dict[str, str]:
  words = {}
  for x in transcript(on):
    match x:
      case ("module", *_):
        words = {}
      case ("run", _, _, _, which, word, _):
        words[which] = word
  return words


def standing() -> list:
  for x in reversed(transcript(ROOT)):
    match x:
      case ("done", about, _, [_, _, _] as now) if question(("stand", about)):
        return now
  return []


def stand(on: str = ROOT) -> list:
  return ask("stand", on)


def clock(on: str = "") -> float:
  return ask("clock", on)


def chance(on: str = "") -> float:
  return ask("chance", on)


def gate(word: str, on: str = "") -> list[str]:
  return ask("gate", on, unquoted(word))


def cd(path: str, on: str = "") -> str:
  got = ask("cd", on, path)
  tell("cd", path)
  return got


def cwd(on: str = "") -> str:
  for x in reversed(transcript(on)):
    match x:
      case ("cd", _, _, _, path):
        return path
  return standing()[1]


def pause(id: str) -> None:
  control("pause", "paused", id)


def wake(id: str) -> None:
  control("wake", "woke", id)


def cancel(id: str) -> None:
  control("cancel", "cancelled", id)


def close(value: object, id: str = "") -> None:
  who = acting()
  match get(who):
    case ("rung", _, by, _, _, retells, *_):
      if retells:
        raise CancelledError()
      if not id and question(("prompt", by)):
        id = by
  id = id or who
  match get(id):
    case ("prompt", _, _, on, shape, *_) if not isinstance(value, BaseException):
      try:
        fits = isinstance(value, (s := eval(shape, module(on))) or object)
      except TypeError:
        fits = isinstance(value, s.__origin__)
      if not fits:
        raise Refused(f"{value!r} not {shape}")
  control("close", "closed", id, value)
  if under(who, id):
    raise CancelledError()


def debug(template: Template) -> None:
  if not (who := acting()):
    raise Refused("no act")
  for i in template.interpolations:
    tell(who, f"debugged {i.expression} = {i.value!r}")


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  return act("wait", on, ending(idle), seconds)


def rung(word: str = "", retells: str = "", actor: str = "", on: str = "") -> Act:
  def ear(id):
    yield "started", id
    if word:
      if tells(id):
        yield told(id, "", word)
      yield "ready", id, word
    while True:
      match (yield):
        case ("done", about, _, value) if question(("reply", about)) and get(about)[2] == id:
          if isinstance(value, BaseException):
            yield "done", id, value
            return
          yield "ready", id, value[1]
        case ("wants", about, by, _, call) if get(by)[4] == id:
          yield "started", about
          while (yield)[:2] != ("done", call):
            pass
          yield "done", about, peek(call)
        case ("done", about, _, value) if question(("run", about)) and get(about)[4] == id:
          if retells and isinstance(value, CancelledError):
            value = None
          if isinstance(value, BaseException) and tells(id):
            yield told(id, "raised " + re.sub(r"^[\w.]*\.", "", repr(value)))
          yield "done", id, value
          return

  if not (word or actor):
    actor = module(on).get("actor", "")
  return act("rung", on, ending(pausing(ear)), word, retells, actor)


def prompt[T](shape: type[T] | object, message: str = "", to: str = "", on: str = "") -> Act[T]:
  named = shape if isinstance(shape, str) else re.sub(r"<class '|'>|[\w.:/]*\.", "", repr(shape))
  actor = to or module(on).get("actor")

  def ear(id):
    if actor != OPERATOR:
      yield "started", id
    yield told(id, message, bound(id, named))
    while True:
      while actor == OPERATOR or paused(id):
        yield
      asking = rung(actor=actor)
      while (yield)[:2] != ("done", asking):
        pass

  return act("prompt", on, pausing(ending(ear)), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act:
  if scope(source) != source:
    raise Refused(f"no chain {source}")

  def ear(id):
    rungs, refused, running, waiting = {}, set(), {}, {}
    asking, unseen, last = "", "", standing()

    def takes(roster, where, actor):
      module(id)["actor"] = actor
      yield told(id, f"roster {roster!r}", headed(id, f"cwd {where}"), headed(id, f"actor {actor}"))

    def replay(of="", words="", writer=""):
      yield "module", id, {**globals(), "__name__": id, "actor": last[2] if last else ""}
      for whose, said in rungs.items():
        donor = get(whose)[5] or whose
        if under(donor, of):
          if whose == writer:
            continue
          if not (words + "\n").startswith(said + "\n"):
            break
          words = words[len(said) + 1 :]
        rung(said, donor)
      rungs.clear()
      if words:
        with site.set(of):
          rung(words)

    if source:
      theirs = transcript(source)
      picked = (filter or (lambda x: x))([x for x in theirs if question(x)])
      picked += [x for x in theirs if x[0] == "reply" and any(under(one[1], x[2]) for one in picked)]
      yield "prefix", id, [it for it in theirs if it[1] == source or any(under(one[1], it[1]) for one in picked)]
      rungs.update(program(source))
    yield "started", id
    yield told(id, f"{label} from {source}".strip() if source else label, bound(id))
    yield from replay()
    if last and not source:
      yield from takes(*last)
    while True:
      a = yield
      if a[0] == "done" and question(("stand", a[1])) and (stood := standing()) != last:
        yield from takes(*(last := stood))
      if scope(a[1]) != id:
        continue
      match a:
        case ("read", about, by, _, path) if question(("prompt", path)) and scope(path) == id:
          yield (
            "done",
            about,
            Text(path, "\n".join(w for x, w in rungs.items() if x != by and under(get(x)[5] or x, path))),
          )
        case ("write", about, by, _, Text(path, content) as text) if question(("prompt", path)) and scope(path) == id:
          yield from replay(path, content, by)
          yield "done", about, text
        case ("cd", about, *_, path):
          yield "done", about, path
        case ("rung", rid, _, _, "", *_):
          waiting[rid] = a
        case ("ready", rid, _, word):
          waiting.pop(rid, None)
          rungs[rid] = word
          if not tells(rid):
            found = get(rid)[5] in refused
          elif found := gate(word, id):
            yield told(rid, "refused", commented("\n".join(found)))
          if found:
            refused.add(rid)
            close(Refused(), rid)
          else:
            act("run", id, None, rid, unquoted(word), get(rid)[5])
        case ("done", about, by, Refused()) if by == about and question(("reply", about)):
          pause(id)
        case ("done", about, _, value):
          waiting.pop(about, None)
          if isinstance(value, Exception) and question(("run", about)):
            module(id)["raised"] = value
          if not isinstance(value, CancelledError) and running.get(get(about)[2]) is False:
            unseen = about
      if a[0] in ("started", "done") and (r := get(a[1])) and r[0] in ("run", "wants"):
        running[r[4] if r[0] == "run" else get(r[2])[4]] = (a[0] == "started") == (r[0] == "run")
      if asking not in waiting and (asking := next((x for x in waiting if not paused(x)), "")):
        maker = get(asking)[2]
        if offered(last[0], to := get(asking)[6]) is None:
          close(Refused(f"{to} no actor"), maker if question(("prompt", maker)) else asking)
        else:
          yield told(asking, f"advance on {maker}")
          if binds := "\n".join(re.findall(r"(?m)^\w+: Act\[.*\] = Act\('\w+'\)$", turns(id)[-1][1])):
            rung(binds)
          with site.set(asking):
            act("reply", id, ending(idle), to)
          unseen = ""
      if (
        unseen
        and not any(running.values())
        and all(peek(x[1], ...) is not ... for x in transcript(id) if x[0] == "prompt" and x[3] == id)
      ):
        prompt(None, f"{unseen} done", on=id)
        unseen = ""

  return act("chain", on, ear, label, source)


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  def ear(id):
    here = scope(id)
    if not (usd or share) or (usd or 0) < 0 or not 0 <= (share or 0) <= 1:
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    yield "started", id
    spent = 0.0
    for old in [x[1] for x in transcript(here) if x[0] == "grant" and x[3] == here and x[1] != id]:
      close(None, old)
    yield told(id, f"usd={usd} share={share}", bound(id, "None"))
    while True:
      match (yield):
        case ("done", about, _, (_, _, (tokens, *_, dollars), _)) if (
          question(("reply", about)) and scope(about) == here
        ):
          spent += dollars
          filled = tokens / (offered(standing()[0], get(about)[4]) or WINDOW)
          yield told(get(about)[2], f"ledger spent={spent} filled={filled}")
          if (usd is not None and spent >= usd) or (share is not None and filled >= share):
            pause(here)

  return act("grant", on, ending(ear), usd, share)


def bash(
  command: str,
  fed: bool = False,
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  def ear(id):
    streams, mute = {x: Text(x) for x in (f"{id}/stdout", f"{id}/stderr")}, "" if fed else "not fed"
    yield told(id, "" if show is HIDDEN else command, bound(id, "Exit"))
    while True:
      match (yield):
        case ("merged", qid, _, _, about) if about == id:
          yield "done", qid, show_err is None
        case ("read", qid, _, _, path) if path in streams:
          yield "done", qid, streams[path]
        case ("write", qid, _, _, Text(path, text) as took) if path == f"{id}/stdin":
          if mute:
            took = Refused(mute)
          else:
            yield "feed", id, text or None
            mute = "" if text else "closed"
          yield "done", qid, took
        case _ if mute == "ended":
          continue
        case ("out", about, _, text, stream) if about == id:
          into = f"{id}/{stream if show_err else 'stdout'}"
          streams[into] = streams[into].grow(text)
        case ("done", about, _, Exit(code, out, err)) if about == id:
          mute, streams = "ended", dict(zip(streams, (out, err), strict=True))
          if show is not HIDDEN:
            yield told(id, f"exited {code}", *[x for x in ((out, show), (err, show_err)) if x[1] not in (None, HIDDEN)])
        case ("cancel" | "close", *_) as a if covers(a, id):
          mute = "ended"
          yield "done", id, ended(a, id)

  return act("bash", on, pausing(ear), command, fed, timeout)


@dataclass
class Text:
  path: str
  content: str = ""
  before: Text | None = None

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()

  def grow(self, text: str) -> Text:
    return Text(self.path, self.content + text)

  def edit(self, lo: int, hi: int, lines: list[str]) -> Text:
    now = self.content.splitlines(True)
    if not 1 <= lo <= hi + 1 <= len(now) + 1:
      raise Refused(f"{self.path} no lines {lo}:{hi}")
    now[lo - 1 : hi] = [x.removesuffix("\n") + "\n" for x in lines]
    return Text(self.path, "".join(now), self)

  def replace(self, old: str, new: str, once: bool = False) -> Text:
    return Text(self.path, self.content.replace(old, new, 1 if once else -1), self)

  def undo(self, n: int = 1) -> Text:
    return self.before.undo(n - 1) if n > 0 and self.before else self

  def append(self, text: str) -> Text:
    return self.insert(len(self.lines) + 1, text)

  def insert(self, line: int, text: str) -> Text:
    return self.edit(line, line - 1, text.splitlines(True))

  def delete(self, lo: int, hi: int) -> Text:
    return self.edit(lo, hi, [])

  def find(self, pattern: str) -> list[int]:
    return grep(pattern)(self.lines)


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Act[T = object](str):
  def __await__(self) -> Generator[object, Any, T]:
    if acting():
      if question(("chain", self)):
        raise Refused(f"{self} never settles")
      return (yield self)
    f = get_running_loop().create_future()

    def waits():
      while (got := peek(self, ...)) is ...:
        yield
      (f.set_exception if isinstance(got, BaseException) else f.set_result)(got)

    drive(g := waits(), f"{self} waits {id(current_task())}")
    try:
      return (yield from f)
    finally:
      g.close()


class Refused(Exception): ...


class Drift(Exception): ...


def under(name, of):
  while name != of and (a := get(name)):
    name = a[2]
  return bool(of) and name == of


def acting():
  return site.get() if get(site.get()) else ""


def question(a):
  return bool(re.fullmatch(rf"{a[0]}\d+", a[1]))


def scope(name):
  match get(name):
    case (kind, _, _, on, *_):
      return name if kind == "chain" else on
  return ""


def tell(name, text="", *notes):
  if (who := acting()) and tells(who):
    say("tell", who, [headed(name, text), *notes])


def tells(id):
  return get(id)[2] != scope(id)


def told(id, text="", *notes):
  return "tell", id, [headed(id, text), *notes]


def control(kind, name, id, *words):
  return (
    get(id)
    and (peek(id, ...) is ... or (kind == "wake" and paused(id)))
    and say(kind, id, *words, [headed(id, " ".join([name, *map(repr, words)]))])
  )


def headed(name, text=""):
  return "#" + commented(f"{name} {text}".rstrip())[2:]


def commented(text):
  return "\n".join(f"# {x}" if x else "#" for x in str(text).split("\n"))


def bound(id, of="object"):
  return f"{id}: Act[{of}] = Act({id!r})"


def showing(got, show):
  return [(got, show) if isinstance(got, Text) else commented(repr(got))]


def shown(pair, seen):
  match pair:
    case (Text() as text, show):
      old, lines = seen.setdefault(text.path, {}), text.lines
      picked = show(lines)
      new = {i: line for i in picked if old.get(i) != (line := lines[i - 1])}
      old.update(new)
      return commented(
        f"{text.path}, {len(picked) - len(new)} known" + "".join(f"\n{i} {line}" for i, line in new.items())
      )
  return pair


def unquoted(word):
  while m := re.search(r"(?ms)^<(S\d+)>\n?(.*)</\1>$", word):
    word = word[: m.start()] + f"{m[1]} = {m[2]!r}" + "\n" * m[0].count("\n") + word[m.end() :]
  return word


def offered(roster, to):
  return next((w for name, efforts, w in roster if to in (name, *[f"{name}/{x}" for x in efforts])), None)


def covers(a, id):
  if a[0] != "close":
    return under(id, a[1]) or scope(id) == a[1]
  while id != a[1] and (question(("rung", id)) or question(("reply", id))):
    id = get(id)[2]
  return id == a[1]


def paused(id):
  for x in reversed(transcript(scope(id))):
    match x:
      case (("pause" | "wake") as kind, *_) if covers(x, id):
        return kind == "pause"
  return False


def ended(a, id):
  return a[3] if a[:2] == ("close", id) else CancelledError()


def idle(id):
  while True:
    yield


def lives(g, a):
  try:
    while (a := g.send(a)) is not None:
      a = say(*a)
  except StopIteration:
    return False
  return True


def pausing(ear):
  def lived(id):
    g, later, stopped = ear(id), [None], paused(id)
    while True:
      while later and not (stopped and later[0]):
        if not lives(g, later.pop(0)):
          return
      match a := (yield):
        case ("pause" | "wake", *_) if covers(a, id):
          stopped = a[0] == "pause"
        case _ if get(a[1]) is a or (a[0] in ("cancel", "close") and covers(a, id) and not under(a[2], id)):
          if not lives(g, a):
            return
        case _:
          later.append(a)

  return lived


def ending(ear):
  def lived(id):
    g, a = ear(id), None
    while lives(g, a):
      a = yield
      if peek(id, ...) is not ...:
        return
      if a[0] in ("cancel", "close") and covers(a, id):
        yield "done", id, ended(a, id)
        return

  return lived


def boot(record=(), **outside):
  get_running_loop()
  made, known, done, held, log, alive, born, busy, left = Counter(), {}, {}, {}, [], {}, {}, set(), []
  first, driven, taken, holding = {e[1]: e for (e,) in record[::-1] if e[0] in ("started", "done")}, set(), set(), []

  def journal():
    kept, due = {e[1]: e for (e,) in record if question(e)}, [*reversed(record)]
    while True:
      while due and not (log or left):
        match due.pop():
          case ((kind, name, by, on, *words),) if question((kind, name)):
            if name in known:
              continue
            if by in outside or by in kept or scope(on) != on:
              born.setdefault(kind, {})[name] = name
              continue
            if not callable(verb := (module(on) or globals()).get(kind)) or re.fullmatch(r"\w+?\d+", by):
              raise Drift(f"{name} drifts")
            with site.set(by):
              try:
                verb(*words, on=on)
              except Refused:
                pass
          case ((kind, about, _, *words) as f,) if kind != "started" and first.get(about) is not f and scope(about):
            yield kind, about, *words
      match a := (yield):
        case (kind, about, by, *_) if about in known and by != "journal":
          if a is known[about] and a[4:] != kept.get(about, a)[4:]:
            raise Drift(f"{about} drifts")
          if by not in known and by not in driven:
            if about not in kept:
              kept[about] = known[about]
              yield "keep", "", (kept[about],)
            if a is not known[about]:
              yield "keep", "", (a,)
          if kind == "wake":
            for name in [x for x in holding if covers(a, x) and peek(x, ...) is ...]:
              holding.remove(name)
              taken.discard(name)
              offers(known[name])

  def says(kind, about, *words):
    a = (kind, about, site.get(), *words)
    if about in known and kind in ("started", "done"):
      taken.add(about)
      if kind == "done":
        done.setdefault(about, words[0])
    held.get(scope(about), []).extend(words[0] if kind == "prefix" else [a])
    log.append((a, ()))
    dispatch()
    return a

  def makes(kind, on, ear, *words):
    speaker = site.get()
    made[speaker, kind] += 1
    by = (question(("rung", speaker)) and known[speaker][5]) or speaker
    on = on or scope(speaker)
    if not on and kind != "chain":
      raise Refused(f"no chain for {kind}")
    names = born.setdefault(kind, {})
    a = (kind, name := names.setdefault((by, made[speaker, kind]), f"{kind}{len(names) + 1}"), by, on, *words)
    if known.setdefault(name, a) is a:
      held.get(on, []).append(a)
      if kind == "chain":
        held[name] = []
      log.append((a, heard := [*outside]))
      busy.add(id(a))
      try:
        for n, g in ears():
          if not (ear or n in heard or n == "journal" or n in busy or name in taken):
            hears(n, g, a)
            heard.append(n)
        if ear:
          heard.append(name)
          live(ear(name), name)
        if name not in taken and (f := "journal" in alive and first.get(name)):
          if f[0] == "done":
            with site.set("journal"):
              says("done", name, *f[3:])
          else:
            taken.add(name)
            holding.append(name)
        offers(a)
        if name not in taken:
          with site.set(name):
            says("done", name, Refused(f"nothing takes {kind}"))
      finally:
        busy.discard(id(a))
      dispatch()
    return Act(name)

  def offers(a):
    for n, g in outside.items():
      if a[1] not in taken:
        hears(n, g, a)

  def ears():
    return sorted(alive.items(), key=lambda pair: (pair[0] not in known, pair[0] in outside))

  def dispatch():
    while not busy:
      if left:
        hears(*left.pop())
      elif log:
        a, heard = log.pop(0)
        left.extend([(*x, a) for x in ears() if x[0] not in heard][::-1])
      else:
        if g := alive.get("journal"):
          hears("journal", g, None)
        if not log:
          return

  def hears(name, g, a):
    if name in busy or alive.get(name) is not g:
      return
    with site.set(name):
      living = False
      busy.add(name)
      try:
        living = lives(g, a)
      finally:
        busy.discard(name)
        living or alive.pop(name)

  def live(g, name):
    if name in alive:
      raise Refused(f"{name} hears")
    if acting():
      driven.add(name)
    alive[name] = g
    hears(name, g, None)
    dispatch()

  globals().update(
    say=says,
    act=makes,
    drive=live,
    get=lambda about: known.get(about),
    peek=lambda at, waiting=None: done.get(at, waiting),
    transcript=lambda on="": [*held.get(on or scope(site.get()), [])],
  )
  for who, hearer in [(OPERATOR, idle(OPERATOR)), *outside.items(), ("journal", journal())]:
    live(hearer, who)
  root = Act(ROOT) if known else chain("root")
  stand()
  return root
