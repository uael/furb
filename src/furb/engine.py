import re
from asyncio import CancelledError, get_running_loop
from collections import Counter
from collections.abc import Callable, Generator
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template
from typing import Any, Never

WINDOW = 200000
OPERATOR = "operator"
TIMEOUT = 600.0
LARGEST = 524288
ROOT = "chain1"
site = ContextVar("site", default=OPERATOR)
raised = None
actor = ""
type Show = Callable[[list[str]], list[int]]
type Filter = Callable[[list[tuple]], list[tuple]]


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
  if acting() and isinstance(got, Text) and (size := len(shown(got, show).encode())) > LARGEST:
    raise Refused(f"{path} shows {size} bytes, over {LARGEST}")
  show is HIDDEN or tell(on, path, got, show)
  return got


def write(text: Text, on: str = "") -> Text:
  got = ask("write", on, Text(text.path, text.content))
  tell(on, text.path, got, differs(text.lines))
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
      case ("tell", *_, notes):
        user.append("\n".join(notes))
  return [*folded, ("user", "\n\n".join(user), None, None)]


def module(on: str = "") -> dict:
  return next((x[3] for x in reversed(transcript(on, "module"))), {})


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
  return answer("stand", ROOT, [])


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
  tell(on, path)
  return path


def cwd(on: str = "") -> str:
  return answer("cd", on, "") or standing()[1]


def pause(id: str) -> None:
  control("pause", "paused", id)


def wake(id: str) -> None:
  control("wake", "woke", id)


def cancel(id: str) -> None:
  control("cancel", "cancelled by " + site.get(), id)


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
        s = eval(shape, module(on)) or object
        fits = any(
          isinstance(value, getattr(x, "__origin__", x))
          for x in (s.__args__ if isinstance(s, type(int | str)) else [s])
        )
      except Exception:
        fits = False
      if not fits:
        raise Refused(f"{value!r} not {shape}")
  control("close", "closed", id, value)
  if under(who, id):
    raise CancelledError()


def debug(template: Template) -> None:
  text = "".join(
    x
    if isinstance(x, str)
    else format(("{!" + x.conversion + "}").format(x.value) if x.conversion else x.value, x.format_spec)
    for x in template
  )
  if (who := acting()) and tells(who) and text:
    say(*told(who, "debugged", debug=text))


def wait(seconds: float = 0.0, *acts: str, on: str = "") -> Act[str | None]:
  def ear(id):
    while not (got := [x for x in acts if over(x)]):
      yield {id, *acts}
    yield "done", id, got[0]

  return act("wait", on, controlled(ear), seconds, *acts)


def rung(word: str = "", retells: str = "", to: str = "", on: str = "") -> Act:
  def ear(id):
    yield "started", id
    mine = {retells, scope(id)}
    if word:
      if tells(id):
        yield told(id, "by " + get(id)[2], word=word)
      while not over(retells) and get(retells) if retells else paused(get(id)[2]):
        yield mine
      yield "ready", id, word
    while True:
      match (yield mine):
        case ("done", about, _, value) if question(("reply", about)) and get(about)[2] == id:
          yield ("done", id, value) if isinstance(value, BaseException) else ("ready", id, value[1])
        case ("wants", about, by, _, call) if get(by)[4] == id:
          yield "started", about
          while not over(call):
            yield mine | {call}
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

  if not (word or to):
    to = module(on).get("actor", "")
  return act("rung", on, controlled(ear), word, retells, to)


def prompt[T](shape: type[T] | object, message: str = "", to: str = "", on: str = "") -> Act[T]:
  named = shape if isinstance(shape, str) else re.sub(r"<class '|'>|[\w.:/]*\.", "", repr(shape))
  thread = on if question(("prompt", on)) else ""
  to = to or (thread and get(thread)[6])
  asks = (to or module(on).get("actor")) != OPERATOR
  says = asks and thread and named == "None" and not over(thread)

  def ear(id):
    if asks:
      yield "started", id
    yield told(id, "", bound(id, named), message=message, thread=thread)
    if says:
      yield "done", id, None
      return
    live = set()
    while True:
      if asks and not (live or paused(id)):
        live.add(rung(to=to))
      match (yield {id}):
        case ("rung", r, *_):
          live.add(r)
        case ("done", r, *_):
          live.discard(r)

  return act("prompt", on, controlled(ear), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act[Never]:
  if scope(source) != source:
    raise Refused("no chain " + source)

  def ear(id):
    rungs, refused, running, waiting, last, heard = program(source), set(), {}, {}, standing(), []
    asking = unseen = ran = reply = ""
    stood = module(source or get(id)[3]).get("actor", last and last[2])

    def takes():
      return told(id, "standing", roster=[x[:3] for x in last[0]], cwd=last[1], actor=module().get("actor"))

    def replay(of="", words=""):
      fresh = not of
      for whose, said in [*rungs.items()]:
        if under(whose, of):
          if (words + "\n").startswith(said + "\n"):
            words = words[len(said) + 1 :]
          else:
            fresh |= whose not in refused
            del rungs[whose]
            cancel(whose)
      if fresh:
        yield "module", id, {**globals(), "__name__": id, "actor": stood}
        for whose, said in rungs.items():
          if whose not in refused:
            rung(said, whose)
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
      yield takes()
    while True:
      match a := (yield {id, "stand", "reply"}):
        case ("done", about, *_) if question(("stand", about)) and standing() != last:
          if standing()[2:] != last[2:]:
            stood = module()["actor"] = standing()[2]
          last = standing()
          yield takes()
        case ("done", about, _, value) if running.get(maker := (get(about) or a)[2]) is False and about not in [
          x[4] for x in transcript(id, "wants")
        ]:
          if scope(about) != id:
            yield "tell", maker, [headed(about, "done", value=value)]
          if not isinstance(value, CancelledError):
            unseen = about + " done", get(maker)[2]
        case ("done", about, by, Refused() as why) if (
          question(("reply", about))
          and scope(about) != id
          and (paused(scope(about)) or not by)
          and (maker := (get(p := get(get(about)[2])[2]) or a)[2]) in running
        ):
          yield "tell", maker, [headed(p, "paused", why=why)]
          if not running[maker]:
            unseen = p + " paused", get(maker)[2]
      if scope(a[1]) != id:
        continue
      match a:
        case ("read", about, by, _, path) if question(("prompt", path)) and scope(path) == id:
          yield "done", about, Text(path, "\n".join(w for x, w in rungs.items() if x != by and under(x, path)))
        case ("write", about, by, _, Text(path, content) as text) if question(("prompt", path)) and scope(path) == id:
          if under(by, path):
            rungs.pop(by, None)
          yield from replay(path, content.rstrip("\n"))
          yield "done", about, text
        case ("read", about, *_, path) if (door := re.fullmatch(r"(bash\d+)/(stdout|stderr)", path)) and get(
          command := door[1]
        ):
          part = ""
          for seen in transcript(scope(command)):
            if seen[:2] == ("started", command):
              part = ""
            elif seen[:2] == ("out", command) and seen[4] == door[2]:
              part += seen[3]
          yield "done", about, getattr(peek(command), door[2], Text(path, part))
        case ("rung", rid, maker, _, "", _, to):
          waiting[rid] = maker, to
        case ("tell", *_, notes):
          heard.append("\n".join(notes))
        case ("ready", rid, _, word):
          waiting.pop(rid, None)
          if heard:
            act("run", id, None, name := rid + "_told", unquoted(rungs.setdefault(name, "\n\n".join(heard))), "")
            heard = []
          ran = tells(rid) and rid
          rungs[(donor := get(rid)[5]) or rid] = word
          if ran and (found := gate(word)):
            yield told(rid, "refused", findings="\n".join(found))
            refused.add(rid)
            close(Refused(), rid)
          else:
            running[rid] = act("run", id, None, rid, unquoted(word), donor)
        case ("done", about, by, Refused()) if not by and question(("reply", about)):
          pause(id)
        case ("done", about, *_):
          waiting.pop(about, None)
      if (r := get(a[1]))[0] == "run":
        running[r[4]] = a[0] != "done"
      elif r[0] == "wants":
        running[get(r[2])[4]] = a[0] != "started"
      if any(running.values()):
        continue
      if asking not in waiting and (asking := next((x for x in waiting if not paused(x)), "")):
        maker, to = waiting[asking]
        if offered(last[0], to) is None:
          close(Refused(f"{to} no actor"), maker if question(("prompt", maker)) else asking)
        else:
          if ran and not heard:
            yield told(ran, "told nothing")
          taught = offered(last[0], to, 3)
          if taught in [offered(last[0], t, 3) for t in {x[4] for x in transcript(id, "reply")}]:
            taught = ""
          yield told(asking, f"advance on {maker}", share=filled(reply) or None, doctrine=taught)
          with site.set(asking):
            reply = act("reply", id, controlled(idle), to)
          unseen = ""
      if unseen and not waiting and (r[2] in running or over(r[2]) or a[0] == "tell"):
        prompt(None, unseen[0], on=ours if question(("prompt", ours := unseen[1])) else "")
        unseen = ""

  return act("chain", on, ear, label, source)


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  def ear(id):
    here = scope(id)
    if not ((usd or share) and (usd or 0) >= 0 <= (share or 0) <= 1):
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    yield "started", id
    spent = 0
    for x in transcript(here, "grant"):
      if x[3] == here and x[1] != id:
        close(None, x[1])
    yield told(id, "", bound(id, "None"), usd=usd, share=share)
    while True:
      match (yield {id, "reply"}):
        case ("done", about, _, (_, _, (*_, cost), _)) if question(("reply", about)) and scope(about) == here:
          spent += cost
          yield told(id, "ledger", spent=spent)
          if (usd and spent >= usd) or (share and filled(about) >= share):
            pause(here)

  return act("grant", on, controlled(ear), usd, share)


def bash(
  command: str,
  stdin: str | None = "",
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  def ear(id):
    yield told(id, "", bound(id, "Exit"), command="" if show is HIDDEN else command)
    while True:
      match (yield {id, "merged"}):
        case ("merged", qid, *_, about) if about == id:
          yield "done", qid, show_err is None
        case ("done", about, _, got) if about == id:
          match got:
            case Exit(code, out, err) if show is not HIDDEN:
              yield told(
                id,
                f"timed out after {timeout}s" if code is None else f"exited {code}",
                stdout=shown(out, show),
                stderr=show_err and shown(err, show_err),
              )
          return

  return act("bash", on, controlled(ear, True), command, stdin, timeout)


@dataclass
class Text:
  path: str
  content: str = ""
  before: Text | None = None

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()

  def edit(self, lo: int, hi: int, text: str) -> Text:
    now = (self.content + "\n").splitlines(True)[: len(self.lines)]
    if not 1 <= lo <= hi + 1 <= len(now) + 1:
      raise Refused(f"{self.path} no lines {lo}:{hi}")
    now[lo - 1 : hi] = [text and text.removesuffix("\n") + "\n"]
    return Text(self.path, "".join(now), self)

  def replace(self, old: str, new: str, count: int = -1) -> Text:
    if old not in self.content:
      raise Refused(f"{self.path} no {old!r}")
    return Text(self.path, self.content.replace(old, new, count), self)

  def undo(self, n: int = 1) -> Text:
    return self.before.undo(n - 1) if n > 0 and self.before else self

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
    if acting():
      if question(("chain", self)):
        raise Refused(self + " never settles")
      return (yield self)
    f = get_running_loop().create_future()

    def waits():
      while (got := peek(self, ...)) is ...:
        yield
      (f.set_exception if isinstance(got, BaseException) else f.set_result)(got)

    drive(g := waits(), f"{self} waits {id(g)}")
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
      return name if kind == "chain" else scope(on)
  return ""


def asked(id, on=""):
  return [x[1] for x in transcript(on, id) if question(x)]


def tell(on, path, got=None, show=HEAD):
  if (who := acting()) and tells(who):
    say(
      "tell",
      q := asked(who, on)[-1],
      [
        headed(
          q,
          path=path,
          **{"numbers": at if (at := show(got.lines)) and at[-1] - at[0] >= len(at) else None, "text": shown(got, show)}
          if isinstance(got, Text)
          else {"value": got},
        )
      ],
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
  return get(id)[2] != scope(id)


def told(id, what="", *notes, **words):
  return "tell", id, [headed(id, what, **words), *notes]


def control(kind, name, id, *words):
  return get(id) and (over(id) or say(*told(id, name, value=(*words, None)[0]))) and say(kind, id, *words)


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
  lines = text.lines
  return "\n".join(lines[i - 1][:1000] for i in show(lines))


def unquoted(word):
  while m := re.search(r"(?ms)^<s:(\w+)>\n?(.*?)</s:\1>$", word):
    word = word[: m.start()] + f"{m[1]} = {m[2]!r}" + "\n" * m[0].count("\n") + word[m.end() :]
  return word


def offered(roster, to, at=2):
  return next(((*x, "")[at] for x in roster if to in (x[0], *[x[0] + "/" + e for e in x[1]])), None)


def covers(a, id):
  kind, of, *_ = a

  def on(name):
    while name != of and question(("prompt", name)):
      name = get(name)[3]
    return name == of

  if kind != "close":
    up = id
    while not on(up) and (x := get(up)):
      up = x[2]
    return on(up) or scope(id) == of
  if question(("reply", id)):
    id = get(id)[2]
  return on(id) or (question(("rung", id)) and on(get(id)[2]))


def paused(id):
  return next((x[0] == "pause" for x in reversed(transcript(scope(id), "pause")) if covers(x, id)), False)


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
    g, later, stopped = ear(id), [], paused(id)
    w = lives(g, None)
    while w is not False:
      a = later.pop(0) if later and not stopped else (yield w)
      if over(id) and not lasting:
        return
      match a:
        case ("pause" | "wake", *_) if covers(a, id):
          stopped = a[0] == "pause"
          stopped or (w := lives(g, a))
        case ("cancel" | "close", _, by, *_) if covers(a, id) and not (over(id) or (stopped and under(by, id))):
          yield "done", id, a[3] if a[:2] == ("close", id) else CancelledError()
        case _ if stopped and get(a[1]) is not a:
          later.append(a)
        case _:
          w = lives(g, a)

  return lived


def boot(record=(), **outside):
  global say, act, drive, get, peek, transcript
  get_running_loop()
  made, known, done, held, log, alive, born, busy, left, waits = Counter(), {}, {}, {}, [], {}, {}, set(), [], {}
  first, driven, taken, holding = {e[1]: e for (e,) in record[::-1] if e[0] in ("started", "done")}, set(), set(), []
  due = [e for (e,) in record[::-1] if first.get(e[1]) is not e]
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
              if by in outside or by in kept or (on and not get(on)):
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
              for name in [x for x in holding if covers(a, x) and not over(x)]:
                holding.remove(name)
                taken.discard(name)
                offers(known[name])
    finally:
      for pile in log, left, due, first, kept:
        pile.clear()

  def say(kind, about, *words):
    a = (kind, about, site.get(), *words)
    if about in known and kind in ("started", "done"):
      taken.add(about)
      kind == "done" and done.setdefault(about, words[0])
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
    names = born.setdefault(kind, {})
    a = (kind, name := names.setdefault((by, made[speaker, kind]), kind + str(len(names) + 1)), by, on, *words)
    if known.setdefault(name, a) is a:
      holds(scope(on), [a])
      if kind == "chain":
        held[name, ""] = []
      log.append((a, heard := [*outside]))
      busy.add(id(a))
      try:
        for n, g in ears(a):
          if not (ear or n in heard or n == "journal" or n in busy or name in taken):
            hears(n, g, a)
            heard.append(n)
        if ear:
          heard.append(name)
          drive(ear(name), name)
        if kept.get(name, a) == a:
          if name not in taken and (f := first.get(name)):
            if f[0] == "done":
              with site.set("journal"):
                say("done", name, f[3])
            else:
              taken.add(name)
              holding.append(name)
          if kind in ("run", "chain") or not owed():
            offers(a)
        if name not in taken:
          with site.set(""):
            say("done", name, Refused("nothing takes " + kind))
      finally:
        busy.discard(id(a))
      dispatch()
    return Act(name)

  def offers(a):
    for n, g in outside.items():
      (a[1] in taken and a[0] != "chain") or hears(n, g, a)

  def ears(a):
    q = get(a[1]) or a[:1] * 4
    k = {a[0], *q[:4], scope(q[2]), scope(q[3])}
    return sorted(
      ((n, g) for n, g in alive.items() if k & (waits.get(n) or k) or a[0] in ("pause", "wake", "cancel", "close")),
      key=lambda pair: (pair[0] not in known, pair[0] in outside),
    )

  def holds(on, facts):
    if (on, "") in held:
      for x in facts:
        for k in {"", x[0].replace("wake", "pause"), x[2]}:
          held.setdefault((on, k), []).append(x)

  def dispatch():
    while not busy:
      if left:
        hears(*left.pop(0))
      elif log:
        a, heard = log.pop(0)
        left.extend((*x, a) for x in ears(a) if x[0] not in heard)
      else:
        if g := alive.get("journal"):
          hears("journal", g, None)
        if not log:
          return

  def hears(name, g, a):
    if name in busy or alive.get(name) is not g:
      return
    with site.set(name):
      busy.add(name)
      try:
        waits[name] = w = lives(g, a)
        w is False and alive.pop(name)
      finally:
        busy.discard(name)

  def drive(g, name):
    if name in alive:
      raise Refused(name + " hears")
    if acting():
      driven.add(name)
    alive[name] = g
    hears(name, g, None)
    dispatch()

  def get(about: str) -> Any:
    return known.get(about)

  def peek(at: str, waiting: object = None) -> object:
    return done.get(at, waiting)

  def transcript(on: str = "", of: str = "") -> list[tuple]:
    return [*held.get((scope(on or site.get()), of), [])]

  for who, hearer in [(OPERATOR, idle(OPERATOR)), *outside.items(), ("journal", journal())]:
    drive(hearer, who)
  root = Act(ROOT) if known else chain("root")
  stand()
  return root


doctrine = """You're furb, an AI harness that reads and speaks only Python and quotes, nothing else.

Your reply
- Your reply is one word: Python that calls the verbs of the engine, which the chain runs in its module.
- The chain runs your reply as it stands, so it holds Python and quotes and nothing else: no text before or after the
  code, no markdown fence, and no markup of a tool call. bash, read and write are verbs that your Python calls, and
  you have no other tool.
- Bind each name in the word itself, with def, class, import or =. The gate does not see a name that exec() binds,
  so it refuses a word that uses one.
- The gate reads each word before it runs. It refuses a word that is not Python or that fails its type check, and
  the prompt asks you again. Before you use a value that can be None or object, as peek and re.search give, narrow
  it with assert or isinstance: assert isinstance(got, Exit).
- Write a long text as a quote: <s:name> at the start of a line, then the text, then </s:name> at the end of a line.
  The quote binds the name to the text as a str, with no escapes.
- The transcript binds each value that it tells under a name, as bash2_command or prompt3_value: a string of more than
  one line as a quote, and any other value as a statement. Use these names as the values that they hold.
- The transcript binds the name of each act that it shows. Await an act for its value, as await bash2.
- read, write, cd, clock, chance, gate and stand give their value at once: await none of them. bash, prompt, wait,
  rung and grant give an act to await. chain gives a chain, which never settles: prompt on it, and never await it.
- A comment in the transcript is what the chain tells you. It binds nothing.

Your prompt
- The line "#rungN advance on promptM" names the prompt that you answer, and its binding shows its shape, as
  promptM: Act[str]. Work until its task is done, then close it with a value of its shape: your final report for a
  str, as close(report). A close ends the word that says it, but no act that the word started, as a rung or a bash.
  close checks the value with isinstance: close a prompt of float with a float, as 1.0, or it raises Refused.
- A word that runs to its end with no close ends its step, and the prompt asks you again with all that the word told,
  or with "#rungN told nothing". HIDDEN shows nothing: hide only what your word uses itself, never what you must see.
- The name of your prompt is the door of its ladder: read("promptM", HIDDEN) gives the program of the ladder, the
  words of its rungs in order, with the words that the gate refused among them. A write of the door edits that
  program, at any word and not only at the last. Write a door in a word of its own, with an old that only one word
  holds. The chain keeps the words before the first word that differs, with their acts. It cancels the old words from
  there, with all that they made, and runs the new text in their place. An act that is done keeps its result: in the
  new text, await it by its name, as await bash6, so it does not run again. Two examples:
  write(read("promptM", HIDDEN).replace(old, new)) fixes the refused word, which stands last, and .append(next)
  after the replace runs your next word in the same turn.
  write(Text("promptM", read("promptM", HIDDEN).content.partition(old)[0] + plan)) changes the plan of a word that
  waits on you, with an old from its start.
- A prompt is your one channel to speak, never debug. A prompt of str is a question: when you need a decision or
  the prompt is not clear, await prompt(str, question, to=OPERATOR) for its answer. A prompt of None is a
  notification: prompt(None, message, to=OPERATOR) tells the operator what your close cannot carry, as your report
  when the shape of your prompt is None. A question for the one that gave you your task goes on its chain:
  prompt(str, question, on=scope(get(promptM)[2])). to=OPERATOR is for the operator alone.
- prompt(None, message, on=promptM) adds a message to the thread promptM, which its model reads at its next step,
  or opens it again once it closed.

A word is a program
- A tool call stops to look at each result. A word does not: it is a program, so write the whole task in it when you
  can. Start acts, await them, branch on what they give, loop and retry. End a word with no close only when you must
  see a result before you can plan the rest.
- Where a step needs a judgment that code cannot make, await prompt(shape, question) in the word: the chain asks you
  on this chain, and the value that you close it with goes back into the word, which goes on. The transcript shows
  what the word told and none of the values that it bound, so put in the question the values that it judges. Your
  answer runs in the same module, so it can use the names of the word.
- Give work, or each independent part of a large task, to another model on a chain of its own when it is large or
  must not fill your window: prompt(str, brief, on=chain(label)). A new chain holds nothing of yours, so put in its
  brief all that it needs.
- This word runs the tests and asks you for each fix, five times at most:
  for _ in range(5):
    r = await bash("pytest -q", show=HIDDEN)
    if r.code == 0:
      close("The tests pass.")
    await prompt(None, "Fix these failures:\\n" + shown(r.stdout, TAIL))
  close("The tests still fail after five fixes.")
- Each prompt in a loop costs a reply. Give a loop a bound, or put a ceiling on the chain with grant(usd).
- A raise ends a word where it raises, and the acts that it started stay. Put a step that can fail in a try.

How to work
- Read before you write. To see a file, read it, with a show for a part of it: read(path, grep(pattern)) or
  read(path, span(lo, hi)). A read shows lines without their numbers, but for lines that are not one run, as a grep
  picks, it binds their numbers as read1_numbers. grep(pattern)(text.lines) gives the numbers of the lines that the
  pattern matches, which count from 1, so the line n is text.lines[n - 1].
- Change a file through the Text that read gave: replace, edit, insert, append or delete, then write. replace
  changes every place, as str does: give count=1 for one place. An edit of what the text does not hold raises
  Refused. A write tells its path, and the lines that landed otherwise than you asked.
- Keep notes and scratch files in scratch://, as write(Text("scratch://plan.md", plan)). Every chain of the record
  shares it, and a command finds it in $TMPDIR.
- Use bash to run commands and to find paths, as bash("grep -rln pattern src"). Let its show pick the lines of a long
  command, as bash(command, show=span(-40, -1)), since its stream flows while it runs. Give a long command a timeout
  that fits. bash runs the command in sh, so it takes no syntax of bash alone, as <(command).
- Start all independent acts first, then await each one, or await wait(seconds, *acts) for the first that ends.
- A report of another chain is a claim. Examine it before you give it as a fact, and give each result from what an
  act showed.
- To see a value, debug it: debug(t"{x=}") or debug(t"{x!r}") tells the text that the same f-string makes. When a
  word raises, the chain binds the exception as raised.
- Keep each word lean, with no comment and no code that the task does not need: the chain reads each token that
  you write again at each reply. A tell shows a long line cut short: read the whole of it from text.content.
- The advance binds your share of the window, as rungN_share = 0.25: the part of the window of your actor that your
  last answer read. When it passes 0.5, go on in a chain that keeps only the work you still need:
  chain(label, source=<your chain>, filter=take(<ids of old work>, inside=False)).
- Do what the prompt asks. Before an act that changes anything else, ask the operator.
"""
