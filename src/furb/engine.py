import re
from asyncio import CancelledError, get_running_loop
from collections import Counter
from collections.abc import Callable, Generator
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template
from typing import Any, Literal, Never

WINDOW = 200000
OPERATOR = "operator"
TIMEOUT = 600.0
LARGEST = 524288
CAP = 40
BUDGET = 4096
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
          any(type(a) is type(value) and a == value for a in x.__args__)
          if (origin := getattr(x, "__origin__", x)) is Literal
          else isinstance(value, origin)
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
  named = (
    shape if isinstance(shape, str) else re.sub(r"""<class '|'>|('[^']*'|"[^"]*")|[\w.:/]*\.""", r"\1", repr(shape))
  )
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
      kin = set()
      for one in (filter or list)([x for x in theirs if question(x)]):
        name = one[1]
        while name not in kin and (b := get(name)):
          kin.add(name)
          name = b[2]
      yield (
        "prefix",
        id,
        [it for it in theirs if it[1] == source or (get(it[1])[2] if question(("reply", it[1])) else it[1]) in kin],
      )
    yield "started", id
    yield told(id, "", bound(id), label=label, source=source)
    yield from replay()
    if last and not source:
      yield takes()
    while True:
      match a := (yield {id, "stand", "reply"}):
        case ("done", about, *_) if question(("stand", about)):
          if standing() != last:
            if standing()[2:] != last[2:]:
              stood = module()["actor"] = standing()[2]
            last = standing()
            yield takes()
          if id == ROOT:
            act("clock", id, None)
          seen = {}
          for fact in transcript(id)[::-1]:
            if (
              fact[0] in ("read", "write") and (path := getattr(was := peek(fact[1]), "path", str(fact[4]))) not in seen
            ):
              seen[path] = 0
              if isinstance(was, Text) and "://" not in path and not get(path.split("/")[0]):
                try:
                  now = read(path, HIDDEN, id).lines
                  d = {i: now[i - 1] for i in differs(was.lines)(now)} | dict.fromkeys(
                    range(len(now) + 1, len(was.lines) + 1)
                  )
                  seen[path] = (len(d) > CAP and len(d)) or d
                except Refused:
                  seen[path] = "gone"
          c = [v for x in transcript(ROOT, "clock") if isinstance(v := peek(x[1]), float)][-2:] or [0]
          c = (c[-1] - c[0]) / 3600
          seen = {k: v for k, v in seen.items() if v}
          while (
            len((note := headed(id, "caught up", hours=(c > 1 and round(c, 1)) or None, changed=seen or None)).encode())
            > BUDGET
          ):
            seen = [*seen][: (isinstance(seen, list) and -1) or None]
          if "\n" in note:
            yield "tell", id, [note]
        case ("done", about, _, value) if running.get(maker := (get(about) or a)[2]) is False and about not in [
          x[4] for x in transcript(id, "wants")
        ]:
          if scope(about) != id:
            yield "tell", maker, [headed(about, "done", value=value)]
          if not isinstance(value, CancelledError):
            unseen = about + " done", maker
        case ("done", about, by, Refused() as why) if (
          question(("reply", about))
          and scope(about) != id
          and (paused(scope(about)) or not by)
          and (maker := (get(p := get(get(about)[2])[2]) or a)[2]) in running
        ):
          yield "tell", maker, [headed(p, "paused", why=why)]
          if not running[maker]:
            unseen = p + " paused", maker
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
          yield (
            "done",
            about,
            getattr(
              peek(command),
              door[2],
              Text(path, "".join(x[3] for x in transcript(command, "out") if x[1] == command and x[4] == door[2])),
            ),
          )
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
          if taught in [offered(last[0], x[4], 3) for x in transcript(id, "reply")]:
            taught = ""
          yield told(
            asking,
            f"advance on {maker}",
            *([f"<s:doctrine>\n{taught}</s:doctrine>"] if taught else []),
            share=filled(reply) or None,
          )
          with site.set(asking):
            reply = act("reply", id, controlled(idle), to)
          unseen = ""
      if unseen and not waiting and (r[2] in running or over(r[2]) or a[0] == "tell"):
        prompt(None, unseen[0], on=ours if question(("prompt", ours := get(unseen[1])[2])) else "")
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
  before = None

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()

  def edit(self, lo: int, hi: int, text: str) -> Text:
    now = (self.content + "\n").splitlines(True)[: len(self.lines)]
    if not 1 <= lo <= hi + 1 <= len(now) + 1:
      raise Refused(f"{self.path} no lines {lo}:{hi}")
    now[lo - 1 : hi] = [text and text.removesuffix("\n") + "\n"]
    (made := Text(self.path, "".join(now))).before = self
    return made

  def replace(self, old: str, new: str, count: int = -1) -> Text:
    if old not in self.content:
      raise Refused(f"{self.path} no {old!r}")
    (made := Text(self.path, self.content.replace(old, new, count))).before = self
    return made

  def undo(self, n: int = 1) -> Text:
    made = self
    for _ in range(n):
      made = made.before or made
    return made

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
      while not over(self):
        yield
      got = peek(self)
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

  def drift(name):
    return Drift(
      f"{name} drifts: the life made {get(name)!r:.300}, the record holds {kept.get(name) or first.get(name)!r:.300}"
    )

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
                raise drift(name)
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
              raise drift(about)
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
