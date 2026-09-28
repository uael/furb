import re
from asyncio import CancelledError, current_task, get_running_loop
from collections import Counter
from collections.abc import Callable, Generator
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template
from typing import Any, Never

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
  if isinstance(got := peek(a := act(kind, on, None, *words), Refused(a + " not done")), Refused):
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
  show is HIDDEN or tell("read", on, path, got, show)
  return got


def write(text: Text, on: str = "") -> Text:
  got = ask("write", on, Text(text.path, text.content))
  if not isinstance(got, Text) or got.lines != text.lines:
    tell("write", on, text.path, got, differs(text.lines))
  return got


def turns(on: str = "") -> list[tuple]:
  folded, user, cut = [], [], 0
  for it in transcript(on):
    match it:
      case ("reply", *_):
        cut = len(user)
      case ("done", about, _, (_, _, _, _) as turn) if question(("reply", about)):
        folded += [("user", "\n\n".join(user[:cut]), None, None), turn]
        del user[:cut]
      case (kind, *_, notes) if kind in ("tell", "pause", "wake", "cancel", "close"):
        user.append("\n".join(notes))
  return [*folded, ("user", "\n\n".join(user), None, None)]


def module(on: str = "") -> dict:
  return next((x[3] for x in reversed(transcript(on)) if x[0] == "module"), {})


def program(on: str = "") -> dict[str, str]:
  words = {}
  for x in transcript(on):
    match x:
      case ("module", *_):
        words = {}
      case ("run", *_, which, word, donor):
        words[donor or which] = word
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
  path = ask("cd", on, path)
  tell("cd", on, path)
  return path


def cwd(on: str = "") -> str:
  for x in reversed(transcript(on)):
    match x:
      case ("done", about, _, str() as path) if question(("cd", about)):
        return path
  return standing()[1]


def pause(id: str) -> None:
  control("pause", "paused", id)


def wake(id: str) -> None:
  control("wake", "woke", id)


def cancel(id: str) -> None:
  control("cancel", "cancelled", id)


def close(value: object, id: str = "") -> None:
  match get(who := acting()):
    case ("rung", _, by, _, _, retells, *_):
      if retells:
        raise CancelledError()
      if not id and question(("prompt", by)):
        id = by
  match get(id := id or who):
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
  got = module().setdefault(who + "_debug", [])
  at = f"[{len(got)}:]" if got else ""
  said = [repr(x.value) for x in template.interpolations]
  got += said
  if tells(who) and said:
    say(*told(who, "debugged " + " ".join(x.expression for x in template.interpolations), **{"debug" + at: said}))


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  return act("wait", on, ending(idle), seconds)


def rung(word: str = "", retells: str = "", actor: str = "", on: str = "") -> Act:
  def ear(id):
    yield "started", id
    if word:
      if tells(id):
        yield told(id, word=word)
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
            yield told(id, "raised", raised=value)
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
    yield told(id, "", bound(id, named), message=message)
    while True:
      while actor == OPERATOR or paused(id):
        yield
      asking = rung(actor=actor)
      while (yield)[:2] != ("done", asking):
        pass

  return act("prompt", on, pausing(ending(ear)), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act[Never]:
  if scope(source) != source:
    raise Refused("no chain " + source)

  def ear(id):
    rungs, refused, running, waiting = program(source), set(), {}, {}
    asking, unseen, last, heard = "", "", standing(), []

    def takes(roster, where, actor):
      module()["actor"] = actor
      return told(id, "standing", roster=roster, cwd=where, actor=actor)

    def replay(of="", words="", writer=""):
      yield "module", id, {**globals(), "__name__": id, "actor": last[2] if last else ""}
      for whose, said in rungs.items():
        if under(whose, of):
          if whose == writer or not (words + "\n").startswith(said + "\n"):
            continue
          words = words[len(said) + 1 :]
        rung(said, whose)
      rungs.clear()
      if words:
        with site.set(of):
          rung(words)

    if source:
      theirs = transcript(source)
      picked = (filter or list)([x for x in theirs if question(x)])
      yield (
        "prefix",
        id,
        [
          it
          for it in theirs
          if it[1] == source
          or any(under(one[1], get(it[1])[2] if question(("reply", it[1])) else it[1]) for one in picked)
        ],
      )
    yield "started", id
    yield told(id, "", bound(id), label=label, source=source)
    yield from replay()
    if last and not source:
      yield takes(*last)
    while True:
      match a := (yield):
        case ("done", about, *_) if question(("stand", about)) and standing() != last:
          yield takes(*(last := standing()))
        case ("done", about, _, value) if running.get(maker := (get(about) or a)[2]) is False and about not in [
          x[4] for x in transcript() if x[0] == "wants"
        ]:
          if scope(about) != id:
            yield "tell", maker, [headed(about, "done", value=value)]
          if not isinstance(value, CancelledError):
            unseen = about
      if scope(a[1]) != id:
        continue
      match a:
        case ("read", about, by, _, path) if question(("prompt", path)) and scope(path) == id:
          yield "done", about, Text(path, "\n".join(w for x, w in rungs.items() if x != by and under(x, path)))
        case ("write", about, by, _, Text(path, content) as text) if question(("prompt", path)) and scope(path) == id:
          yield from replay(path, content, by)
          yield "done", about, text
        case ("rung", rid, maker, _, "", _, to):
          waiting[rid] = maker, to
        case (kind, *_, notes) if kind in ("tell", "pause", "wake", "cancel", "close"):
          heard.append("\n".join(notes))
        case ("ready", rid, _, word):
          waiting.pop(rid, None)
          if heard:
            act("run", id, None, name := rid + "_told", unquoted(rungs.setdefault(name, "\n\n".join(heard))), "")
            heard = []
          rungs[(donor := get(rid)[5]) or rid] = word
          if not tells(rid):
            found = donor in refused
          elif found := gate(word):
            yield told(rid, "refused", findings="\n".join(found))
          if found:
            refused.add(rid)
            close(Refused(), rid)
          else:
            act("run", id, None, rid, unquoted(word), donor)
        case ("done", about, by, Refused()) if not by and question(("reply", about)):
          pause(id)
        case ("done", about, _, value):
          waiting.pop(about, None)
          if isinstance(value, Exception) and question(("run", about)):
            module()["raised"] = value
      if (r := get(a[1]))[0] in ("run", "wants"):
        running[r[4] if r[0] == "run" else get(r[2])[4]] = (a[0] == "started") == (r[0] == "run")
      if any(running.values()):
        continue
      if asking not in waiting and (asking := next((x for x in waiting if not paused(x)), "")):
        maker, to = waiting[asking]
        if offered(last[0], to) is None:
          close(Refused(f"{to} no actor"), maker if question(("prompt", maker)) else asking)
        else:
          yield told(asking, f"advance on {maker}")
          with site.set(asking):
            act("reply", id, ending(idle), to)
          unseen = ""
      if unseen and all(peek(x[1], ...) is not ... for x in transcript() if x[0] == "prompt" and x[3] == id):
        prompt(None, unseen + " done")
        unseen = ""

  return act("chain", on, ear, label, source)


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  def ear(id):
    here = scope(id)
    if not (usd or share) or (usd or 0) < 0 or not 0 <= (share or 0) <= 1:
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    yield "started", id
    spent = 0
    for x in transcript(here):
      if x[0] == "grant" and x[3] == here and x[1] != id:
        close(None, x[1])
    yield told(id, "", bound(id, "None"), usd=usd, share=share)
    while True:
      match (yield):
        case ("done", about, _, (_, _, (tokens, *_, cost), _)) if question(("reply", about)) and scope(about) == here:
          spent += cost
          filled = tokens / (offered(standing()[0], get(about)[4]) or WINDOW)
          yield told(id, "ledger", spent=spent, filled=filled)
          if (usd and spent >= usd) or (share and filled >= share):
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
    streams, mute = {x: Text(x) for x in (id + "/stdout", id + "/stderr")}, "" if fed else "not fed"
    yield told(id, "", bound(id, "Exit"), command="" if show is HIDDEN else command)
    while True:
      match a := (yield):
        case ("merged", qid, *_, about) if about == id:
          yield "done", qid, show_err is None
        case ("read", qid, *_, path) if path in streams:
          yield "done", qid, streams[path]
        case ("write", qid, *_, Text(path, text) as took) if path == id + "/stdin":
          if mute:
            took = Refused(mute)
          else:
            yield "feed", id, text or None
            mute = "" if text else "closed"
          yield "done", qid, took
        case _ if mute == "ended":
          continue
        case ("out", about, _, text, stream) if about == id:
          into = id + "/" + (stream if show_err else "stdout")
          streams[into] = streams[into].grow(text)
        case ("done", about, _, Exit(code, out, err)) if about == id:
          mute, streams = "ended", dict(zip(streams, (out, err), strict=True))
          if show is not HIDDEN:
            yield told(id, f"exited {code}", stdout=shown(out, show), stderr=show_err and shown(err, show_err))
        case ("cancel" | "close", *_) if covers(a, id):
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
        raise Refused(self + " never settles")
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
  return name == of != ""


def acting():
  return site.get() if get(site.get()) else ""


def question(a):
  return bool(re.fullmatch(a[0] + r"\d+", a[1]))


def scope(name):
  match get(name):
    case (kind, _, _, on, *_):
      return name if kind == "chain" else on
  return ""


def tell(kind, on, path, got=None, show=HEAD):
  if (who := acting()) and tells(who):
    say(
      "tell",
      who,
      [
        headed(
          [x[1] for x in transcript(on) if x[0] == kind and x[2] == who][-1],
          path=path,
          **{"text": shown(got, show)} if isinstance(got, Text) else {"value": got},
        )
      ],
    )


def tells(id):
  return get(id)[2] != scope(id)


def told(id, what="", *notes, **words):
  return "tell", id, [headed(id, what, **words), *notes]


def control(kind, name, id, *words):
  return (
    get(id)
    and (peek(id, ...) is ... or (kind == "wake" and paused(id)))
    and say(kind, id, *words, [headed(id, name, value=(*words, None)[0])])
  )


def headed(id, what="", **words):
  lines = [f"#{id} {what}".rstrip()]
  for word, value in words.items():
    name = f"{id}_{word}"
    if isinstance(value, str) and "\n" in value:
      while f"</s:{name}>\n" in value + "\n":
        name += "_"
      lines.append(f"<s:{name}>\n{value}</s:{name}>")
    elif value not in (None, ""):
      lines.append(f"{name} = " + re.sub(r"^[\w.]+\.(?=[A-Z]\w*\()", "", repr(value)))
  return "\n".join(lines)


def bound(id, of="object"):
  return f"{id}: Act[{of}] = Act({id!r})"


def shown(text, show):
  return "\n".join(text.lines[i - 1] for i in show(text.lines))


def unquoted(word):
  while m := re.search(r"(?ms)^<s:(\w+)>\n?(.*?)</s:\1>$", word):
    word = word[: m.start()] + f"{m[1]} = {m[2]!r}" + "\n" * m[0].count("\n") + word[m.end() :]
  return word


def offered(roster, to):
  return next((w for name, efforts, w in roster if to in (name, *[name + "/" + x for x in efforts])), None)


def covers(a, id):
  kind, over, *_ = a
  if kind != "close":
    return under(id, over) or scope(id) == over
  while id != over and (question(("rung", id)) or question(("reply", id))):
    id = get(id)[2]
  return id == over


def paused(id):
  return next(
    (x[0] == "pause" for x in reversed(transcript(scope(id))) if x[0] in ("pause", "wake") and covers(x, id)), False
  )


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
  due = [e for (e,) in record[::-1] if e[0] != "started" and first.get(e[1]) is not e]
  kept = {e[1]: e for (e,) in record if question(e)}

  def owed():
    while due and question(due[-1]) and due[-1][1] in known:
      due.pop()
    return due

  def journal():
    try:
      while True:
        while owed() and not (log or left):
          match due.pop():
            case (kind, name, by, on, *words) if question((kind, name)):
              if by in outside or by in kept or scope(on) != on:
                born.setdefault(kind, {})[name] = name
                continue
              if not callable(verb := (module(on) or globals()).get(kind)) or re.fullmatch(r"\w+?\d+", by):
                raise Drift(name + " drifts")
              with site.set(by):
                try:
                  verb(*words, on=on)
                except Refused:
                  pass
            case (kind, about, _, *words) if scope(about):
              yield kind, about, *words
        match a := (yield):
          case (kind, about, by, *_) if about in known and by != "journal":
            if not by and (owed() or about in first):
              raise Drift(about + " drifts")
            if by not in known and by not in driven:
              if about not in kept:
                yield "keep", "", (kept.setdefault(about, known[about]),)
              if a is not known[about]:
                yield "keep", "", (a,)
            if kind == "wake":
              for name in [x for x in holding if covers(a, x) and peek(x, ...) is ...]:
                holding.remove(name)
                taken.discard(name)
                offers(known[name])
    finally:
      for pile in log, left, due, first, kept:
        pile.clear()

  def says(kind, about, *words):
    a = (kind, about, site.get(), *words)
    if about in known and kind in ("started", "done"):
      taken.add(about)
      kind == "done" and done.setdefault(about, words[0])
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
      raise Refused("no chain for " + kind)
    names = born.setdefault(kind, {})
    a = (kind, name := names.setdefault((by, made[speaker, kind]), kind + str(len(names) + 1)), by, on, *words)
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
        if kept.get(name, a) == a:
          if name not in taken and (f := first.get(name)):
            if f[0] == "done":
              with site.set("journal"):
                says("done", name, f[3])
            else:
              taken.add(name)
              holding.append(name)
          if kind == "run" or not owed():
            offers(a)
        if name not in taken:
          with site.set(""):
            says("done", name, Refused("nothing takes " + kind))
      finally:
        busy.discard(id(a))
      dispatch()
    return Act(name)

  def offers(a):
    for n, g in outside.items():
      a[1] in taken or hears(n, g, a)

  def ears():
    return sorted(alive.items(), key=lambda pair: (pair[0] not in known, pair[0] in outside))

  def dispatch():
    while not busy:
      if left:
        hears(*left.pop(0))
      elif log:
        a, heard = log.pop(0)
        left.extend((*x, a) for x in ears() if x[0] not in heard)
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
      raise Refused(name + " hears")
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


doctrine = """You're furb, an AI harness that reads and speaks only Python and quotes, nothing else.

Your reply
- Your reply is one word: Python that calls the verbs of the engine, which the chain runs in its module.
- The gate reads each word before it runs. It refuses a word that is not Python or that fails its type check, and
  the prompt asks you again. Before you use a value that can be None or object, as peek and re.search give, narrow
  it with assert or isinstance: assert isinstance(got, Exit).
- Write a long text as a quote: <s:name> at the start of a line, then the text, then </s:name> at the end of a line.
  The quote binds the name to the text as a str, with no escapes. Give each quote a name that says what it holds.
- The transcript binds each value that it tells under a name, as bash2_command or prompt3_value: a string of more than
  one line as a quote, and any other value as a statement. Use these names as the values that they hold.
- The transcript binds the name of each act that it shows. Await an act for its value.
- A comment in the transcript is what the chain tells you. It binds nothing.

Your prompt
- The line "#rungN advance on promptM" names the prompt that you answer, and its binding shows its shape, as
  promptM: Act[str]. Work until its task is done, then close it with a value of its shape: your final report for a
  str, as close(report).
- A word that closes nothing ends its step, and the prompt asks you again at once, with all that the word told. Use
  this to look before you answer. To wait for an act, await it in your word.
- When the gate refuses your word, fix it through the door of your prompt. The name of your prompt is the door of its
  ladder, and the refused word stands last in it: write(read("promptM", HIDDEN).replace(old, new)), with an old that
  only the refused word holds. The chain runs the fixed word in place of the refused one. Write a door in a word of
  its own.
- A prompt is your one channel to speak. To ask the operator, when you need a decision or the prompt is not clear,
  await prompt(str, question, to=OPERATOR) for its answer. To give work to another model, prompt it on a chain of
  its own: prompt(str, brief, on=chain(label)). A new chain holds nothing of yours, so put in its brief all that it
  needs.

How to work
- Read before you write. To see a file, read it, with a show for a part of it: read(path, grep(pattern)) or
  read(path, span(lo, hi)). A read shows lines without their numbers: text.find(pattern) gives the number of a line.
- Change a file through the Text that read gave: replace, edit, insert or delete, then write. A write tells the lines
  that landed otherwise than you asked.
- Use bash to run commands and to find paths, as bash("grep -rln pattern src"). Let its show pick the lines of a long
  command, as bash(command, show=span(-40, -1)), since its stream flows while it runs. Give a long command a timeout
  that fits. The code None, as in "exited None", means that the timeout or a signal ended the command.
- Start all independent acts first, then await them. Give each independent part of a large task its own chain.
- A report of another chain is a claim. Examine a claim before you give it as a fact, and give each result from what
  an act showed.
- To see a value, debug it: debug(t"{value}") shows each value of its template. When a word raises, the chain binds
  the exception as raised.
- Keep each word small. The chain reads each token that you write again at each reply.
- Do what the prompt asks. Before an act that changes anything else, ask the operator.
"""
