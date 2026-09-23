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
WORLD = "world"
TIMEOUT = 600.0
site = ContextVar("site", default=OPERATOR)
modules, acts, asked, outcomes = {}, {}, {}, {}
type Show = Callable[[list[str]], list[int]]
type Filter = Callable[[list[tuple]], list[tuple]]


def send(kind, about, *words, by=""):
  raise RuntimeError("no life")


def ask(kind, on, *words):
  raise RuntimeError("no life")


def act(kind, on, ear, *words):
  raise RuntimeError("no life")


def drive(g, name):
  raise RuntimeError("no life")


def span(lo: int, hi: int) -> Show:
  def picks(lines):
    a, b = (x + len(lines) + 1 if x < 0 else x for x in (lo, hi))
    return list(range(max(a, 1), min(b, len(lines)) + 1))

  return picks


def grep(pattern: str) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if re.search(pattern, line)]


def differs(old: list[str]) -> Show:
  return lambda lines: [i for i, line in enumerate(lines, 1) if old[i - 1 : i] != [line]]


HEAD, TAIL, HIDDEN = span(1, 2000), span(-250, -1), span(0, 0)


def take(*ids: str, inside: bool = True) -> Filter:
  return lambda facts: [a for a in facts if any(under(a[1], one) for one in ids) == inside]


def read(path: str, show: Show = HEAD, on: str = "") -> Text:
  _, got = ask("read", on, path)
  if show is not HIDDEN:
    tell("read", path, *showing(got, show))
  return got


def write(text: Text, on: str = "") -> Text:
  _, got = ask("write", on, text)
  if not isinstance(got, Text) or got.lines != text.lines:
    tell("write", text.path, *showing(got, differs(text.lines)))
  return got


def peek(at: str, on: str = "") -> object:
  _, got = ask("peek", on, at)
  tell("peek", at)
  return got


def turns(on: str = "") -> list[tuple]:
  _, got = ask("turns", on)
  tell("turns", len(got or ()))
  return got


def clock(on: str = "") -> float:
  _, got = ask("clock", on)
  tell("clock", got)
  return got


def chance(on: str = "") -> float:
  _, got = ask("chance", on)
  tell("chance", got)
  return got


def gate(word: str, on: str = "") -> list[str]:
  _, got = ask("gate", on, unquoted(word), ask("program", on)[1] or {})
  tell("gate", "", *[commented(x) for x in got or ()])
  return got


def cd(path: str, on: str = "") -> str:
  _, got = ask("cd", on, path)
  tell("cd", path)
  return got


def cwd(on: str = "") -> str:
  _, got = ask("cwd", on)
  tell("cwd", got)
  return got


def get(about: str) -> tuple:
  return acts[about]


def pause(id: str) -> None:
  control("pause", "paused", id)


def wake(id: str) -> None:
  control("wake", "woke", id)


def cancel(id: str) -> None:
  control("cancel", "cancelled", id)


def close(value: object, id: str = "") -> None:
  who = acting()
  match acts.get(who):
    case ("rung", _, by, _, _, retells, *_):
      if retells:
        raise CancelledError()
      if not id and question(("prompt", by)):
        id = by
  id = id or who
  match acts.get(id):
    case ("prompt", _, _, on, shape, *_) if shape != "None" and not isinstance(value, BaseException):
      try:
        fits = isinstance(value, s := eval(shape, modules[on]))
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
  send("tell", who, [headed(who, f"debugged {i.expression} = {i.value!r}") for i in template.interpolations])


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  return act("wait", on, ending(started(idle)), seconds)


def rung(word: str = "", retells: str = "", actor: str = "", returns: str = "", on: str = "") -> Act:
  def ear(id):
    wants = None
    if word:
      told(id, "", word)
      yield "ready", id, word
    while True:
      match (yield):
        case ("answer", about, _, (_, said, _, _)) if about == id:
          yield "ready", id, said
        case ("wants", about, _, call) if about == id:
          wants = call
        case ("done", about, _, value) if about == wants:
          wants = None
          yield "sent", id, value
        case ("ran", about, _, value) if about == id:
          if retells and isinstance(value, CancelledError):
            value = None
          if isinstance(value, BaseException):
            told(id, "raised " + re.sub(r"^[\w.]*\.", "", repr(value)))
          yield "done", id, value
          return

  return act("rung", on, ending(pausing(ear)), word, retells, actor, returns)


def prompt[T](shape: type[T] | object, message: str = "", to: str = "", on: str = "") -> Act[T]:
  named = shape if isinstance(shape, str) else re.sub(r"<class '|'>|[\w.:/]*\.", "", repr(shape))

  def ear(id):
    if to == OPERATOR:
      told(id, message, bound(id, named))
      yield from idle(id)
    while True:
      asking = rung(actor=to, returns=named)
      while (yield)[:2] != ("done", asking):
        pass

  return act("prompt", on, pausing(ending(started(ear, to))), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act:
  if scope(source) != source:
    raise Refused(f"no chain {source}")

  def ear(id):
    transcript, rungs, program, refused, own, paused, mine = [], {}, {}, {}, set(), {}, {id: None}
    waiting, asking, running, to_run, unseen = {}, "", "", [], ""

    def stands(name):
      return any(covers(x, name) for x in paused.values())

    def answers():
      match a:
        case ("program", *_):
          return {**program}
        case ("read", _, by, _, path) | ("write", _, by, _, Text(path)) if path in mine and question(("prompt", path)):
          if a[0] == "write":
            replay(path, a[4].content, by)
            return a[4]
          return Text(
            path, "\n".join(said for one, said in rungs.items() if one != by and under(acts[one][5] or one, path))
          )
        case ("transcript", *_, at) if at in mine:
          return transcript[: mine[at]]
        case ("stand", *_):
          return standing
        case ("cd", *_):
          return a[4]
        case ("cwd", *_):
          return next((x[4] for x in reversed(transcript) if x[0] == "cd"), standing[1])
        case ("turns", *_):
          return turns_of(transcript)

    def replay(of="", words="", by=""):
      for whose, said in rungs.items():
        donor = acts[whose][5] or whose
        if under(donor, of):
          if whose == by:
            continue
          if not (words + "\n").startswith(said + "\n"):
            break
          words = words[len(said) + 1 :]
        own.add(rung(said, donor))
      rungs.clear()
      program.clear()
      to_run.clear()
      modules[id] = {**globals(), "__name__": id, "actor": standing[2], "raised": None}
      if words:
        token = site.set(of)
        rung(words)
        site.reset(token)

    if source:
      _, theirs = ask("transcript", source, source)
      picked = (filter or (lambda x: x))([x for x in theirs if question(x)])
      transcript += [
        it for it in theirs if it[0] == "holds" or it[1] == source or any(under(one[1], it[1]) for one in picked)
      ]
      rungs.update(ask("program", source)[1])
    cut = len(transcript)
    transcript.append(told(id, f"{label} from {source}".strip() if source else label, bound(id)))
    _, standing = ask("stand", source or id)
    if not source:
      transcript.append(told(id, f"stands {standing!r}"))
    replay()
    while True:
      a = yield
      if (a[3] if question(a) else scope(a[1])) != id:
        continue
      if question(a):
        if (about := a[1]) not in own:
          mine[about] = len(transcript) + 1
        if about in acts and stands(about):
          yield "pause", about, []
        if (got := answers()) is not None:
          yield "done", about, got
      if a[1] in mine and not (a[0] == "tell" and a[2] == id):
        transcript.append(a)
      match a:
        case ("rung", rid, _, _, "", *_):
          waiting[rid] = a
        case ("ready", rid, _, word):
          waiting.pop(rid, None)
          rungs[rid] = word
          if rid in own:
            found = acts[rid][5] in refused
          elif found := ask("gate", id, unquoted(word), {**program})[1]:
            transcript.append(told(rid, "refused", commented("\n".join(found))))
          if found:
            refused[rid] = found
            close(Refused(), rid)
          else:
            program[rid] = unquoted(word)
            to_run.append(rid)
        case ("done", about, _, value):
          waiting.pop(about, None)
          if isinstance(value, Exception) and about in program:
            modules[id]["raised"] = value
          if about == running:
            running = ""
          if not isinstance(value, CancelledError) and about in acts and acts[about][2] in rungs:
            unseen = about
        case ("wants", *_):
          running = ""
        case ("pause", about, *_) if a[2] != id:
          paused[about] = a
        case ("wake", about, *_):
          paused.pop(about, None)
      if asking not in waiting and (asking := next((x for x in waiting if not stands(x)), "")):
        prompted = question(("prompt", maker := acts[asking][2]))
        if prompted and ("tell", maker) not in [x[:2] for x in transcript]:
          transcript.append(told(maker, acts[maker][5], bound(maker, acts[maker][4])))
        if offered(standing, to := acts[asking][6] or modules[id]["actor"]) is None:
          close(Refused(f"{to} no actor"), maker if prompted else asking)
        else:
          q, held = ask("holds", id, asking)
          transcript += [told(asking, f"advance on {maker}" if prompted else "advance"), q]
          if binds := "\n".join(
            x for it in transcript[cut:] if it[0] == "tell" for x in it[3] if str(x).startswith(it[1] + ": Act[")
          ):
            own.add(rung(binds))
          cut, unseen = len(transcript), ""
          if not held:
            yield "ask", asking, id, to, turns_of(transcript)
      if to_run and not running:
        yield "run", (running := to_run.pop(0)), id, program[running], acts[running][5]
      if unseen and not running and all(x in outcomes for x in mine if question(("prompt", x))):
        prompt(None, f"{unseen} done", on=id)
        unseen = ""

  return act("chain", on, ear, label, source)


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  def ear(id):
    here = scope(id)
    if not (usd or share) or (usd or 0) < 0 or not 0 <= (share or 0) <= 1:
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    _, transcript = ask("transcript", here, here)
    if not isinstance(transcript, list):
      yield "done", id, transcript
      return
    _, standing = ask("stand", here)
    spent = 0.0
    for old in transcript:
      match old:
        case ("grant", gid, *_) if gid not in outcomes:
          close(None, gid)
    told(id, f"usd={usd} share={share}", bound(id, "None"))
    while True:
      match (yield):
        case ("answer", about, _, (_, _, (seen, *_, dollars), _)) if scope(about) == here:
          spent += dollars
          filled = seen / (offered(standing, acts[about][6] or modules[here]["actor"]) or WINDOW)
          told(about, f"ledger spent={spent} filled={filled}")
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
    out, err = f"{id}/stdout", f"{id}/stderr"
    streams, mute = {out: Text(out), err: Text(err)}, "" if fed else "not fed"
    told(id, "" if show is HIDDEN else command, bound(id, "Exit"))
    while True:
      match (yield):
        case ("merged", qid, _, _, about) if about == id:
          yield "done", qid, show_err is None
        case ("read", qid, _, _, path) if path in streams:
          yield "done", qid, streams[path]
        case ("write", qid, _, _, Text(path, text) as took) if path == f"{id}/stdin":
          if takes := not mute:
            yield "feed", id, text or None
            mute = "" if text else "closed"
          yield "done", qid, took if takes else Refused(mute)
        case _ if mute == "ended":
          continue
        case ("peek", qid, _, _, at) if at == id:
          yield "done", qid, Exit(None, *streams.values())
        case ("out", about, _, text, stream) if about == id:
          into = f"{id}/{stream}" if show_err is not None else out
          streams[into] = streams[into].grow(text)
        case ("exited", about, _, code) if about == id:
          mute = "ended"
          if show is not HIDDEN:
            told(
              id,
              f"exited {code}",
              (streams[out], show),
              *[(streams[err], x) for x in [show_err] if x not in (None, HIDDEN)],
            )
          yield "done", id, Exit(code, *streams.values())
        case ("cancel" | "close", *_) as a if covers(a, id):
          mute = "ended"
          yield "done", id, ended(a, id)

  return act("bash", on, pausing(started(ear)), command, fed, timeout)


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
    if not 1 <= lo <= hi + 1 <= len(self.lines) + 1:
      raise Refused(f"{self.path} no lines {lo}:{hi}")
    now = self.content.splitlines(True)
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
      while self not in outcomes:
        yield
      (f.set_exception if isinstance(got := outcomes[self], BaseException) else f.set_result)(got)
      f.exception()

    drive(waits(), f"{self} waits {id(current_task())}")
    return (yield from f.__await__())


class Refused(Exception): ...


class Drift(Exception): ...


def under(name, of):
  while name != of and (a := acts.get(name) or asked.get(name)):
    name = a[2]
  return bool(of) and name == of


def acting():
  return site.get() if site.get() in acts else ""


def question(a):
  return bool(re.fullmatch(rf"{a[0]}(\d+|@.+)", a[1]))


def scope(name):
  match acts.get(name) or asked.get(name):
    case (kind, _, _, on, *_):
      return name if kind == "chain" else on
  return ""


def tell(name, text="", *notes):
  if who := acting():
    send("tell", who, [headed(name, text), *notes])


def told(id, text="", *notes):
  return send("tell", id, [headed(id, text), *notes])


def control(kind, name, id, *words):
  return send(kind, id, *words, [headed(id, " ".join([name, *[repr(x) for x in words]]))])


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


def turns_of(heard):
  seen, folded, said, cut = {}, [], [], 0
  for it in heard:
    match it:
      case ("holds", _, by, on, _) if by == on:
        cut = len(said)
      case ("answer", _, _, turn):
        folded += [("user", "\n\n".join(said[:cut]), None, None), turn]
        del said[:cut]
      case (kind, *_, notes) if kind in ("tell", "pause", "wake", "cancel", "close") and notes:
        said.append("\n".join([shown(x, seen) for x in notes]))
  return [*folded, ("user", "\n\n".join(said), None, None)]


def unquoted(word):
  while m := re.search(r"(?ms)^<(S\d+)>\n?(.*)</\1>$", word):
    word = word[: m.start()] + f"{m[1]} = {m[2]!r}" + "\n" * m[0].count("\n") + word[m.end() :]
  return word


def offered(standing, to):
  return next((w for name, efforts, w in standing[0] if to in (name, *[f"{name}/{x}" for x in efforts])), None)


def covers(a, id):
  return (under(id, a[1]) or scope(id) == a[1]) and (a[0] != "close" or id == a[1] or question(("rung", id)))


def ended(a, id):
  return a[3] if a[:2] == ("close", id) else CancelledError()


def idle(id):
  while True:
    yield


def lives(g, a):
  try:
    while (a := g.send(a)) is not None:
      a = send(*a)
  except StopIteration:
    return False
  return True


def pausing(ear):
  def lived(id):
    g, held, paused = ear(id), [None], False
    while True:
      while held and not paused:
        if not lives(g, held.pop(0)):
          return
      match a := (yield):
        case ("pause" | "wake", *_) if covers(a, id):
          paused = a[0] == "pause"
        case ("cancel" | "close", _, by, *_) if not under(by, id):
          if not lives(g, a):
            return
        case _ if not (paused and asked.get(a[1]) is a):
          held.append(a)

  return lived


def ending(ear):
  def lived(id):
    g, a = ear(id), None
    while lives(g, a):
      match a := (yield):
        case ("done", about, *_) if about == id:
          return
        case ("cancel" | "close", *_) if covers(a, id):
          yield "done", id, ended(a, id)
          return

  return lived


def started(ear, to=OPERATOR):
  def lived(id):
    if to == OPERATOR and not ask("holds", "", id)[1]:
      yield "start", id
    yield from ear(id)

  return lived


def boot(record=(), **outside):
  get_running_loop()
  kept = list(record)
  for table in (modules, acts, asked, outcomes):
    table.clear()
  log, alive, made, born, busy, heard = [], {}, Counter(), {}, set(), 0

  def door():
    answers = {e[1][1]: e[2] if e[2:] else e[1][3] for e in record if e[2:] or e[1][0] == "done"}
    while True:
      match a := (yield):
        case ("holds", qid, _, _, about):
          yield "done", qid, [e[1] for e in record if e[1][1] == about]
        case ("peek", qid, _, _, at):
          yield "done", qid, outcomes.get(at)
        case (_, qid, *_) if question(a) and qid in answers:
          yield "done", qid, answers[qid]

  def journal():
    facts = {e[1][1]: e[1] for e in kept if question(e[1])}
    said, cursor, copies, after, held = set(), 0, set(), "", set(facts)

    def keep(entry):
      kept.append(entry)
      send("keep", "", entry)

    while True:
      while cursor < len(record):
        match kept[cursor]:
          case (_, (kind, _, "operator", on, *words) as then) if question(then) and (not on or on in said):
            token = site.set(OPERATOR)
            globals()[kind](*words, on=on)
            site.reset(token)
          case (before, (_, about, *_)) if not (about in said and before in said):
            break
          case (_, (kind, about, by, *words) as then) if not question(then) and kind != "done":
            copies.add(id(send(kind, about, *words, by=by)))
        cursor += 1
      a = yield
      match a:
        case (_, qid, by, *words) if question(a):
          ours = qid in acts
          if ours and list(words[1:]) != list(facts.get(qid, a)[4:]):
            raise Drift(f"{qid} drifts")
          if ours or question(("rung", by)):
            said.add(qid)
            facts.setdefault(qid, a)
            if by == OPERATOR and ours and qid not in held:
              held.add(qid)
              keep((after, a))
          if ours:
            after = qid
        case (kind, about, by, *words) if by in (WORLD, OPERATOR) and about in facts and id(a) not in copies:
          answer = [words[0]] if kind == "done" and about in asked else []
          if about not in held:
            held.add(about)
            keep((after, facts[about], *answer))
          if not answer:
            keep((after, a))

  def named(kind, on, *words):
    made[speaker := site.get()] += 1
    by = (question(("rung", speaker)) and acts[speaker][5]) or speaker
    return (kind, f"{kind}@{by}.{made[speaker]}", by, on or scope(speaker), *words)

  def says(kind, about, *words, by=""):
    a = (kind, about, by or site.get(), *words)
    if kind == "done" and (about in acts or about in asked):
      outcomes.setdefault(about, words[0])
    log.append(a)
    dispatch()
    return a

  def asks(kind, on, *words):
    a = named(kind, on, *words)
    asked[a[1]] = a
    for name, g in ears():
      if name not in busy and a[1] not in outcomes:
        hears(name, g, a)
    dispatch()
    if isinstance(got := outcomes.get(a[1]), Refused) and kind != "peek":
      raise got
    return a, got

  def makes(kind, on, ear, *words):
    a = named(kind, on, *words)
    if not a[3] and kind != "chain":
      raise Refused(f"no chain for {kind}")
    names = born.setdefault(kind, {})
    a = (kind, names.setdefault(a[1], f"{kind}{len(names) + 1}"), *a[2:])
    if acts.setdefault(a[1], a) is a:
      log.append(a)
      live(ear(a[1]), a[1])
    return Act(a[1])

  def ears():
    return sorted(alive.items(), key=lambda pair: (pair[0] not in acts, pair[0] in outside))

  def dispatch():
    nonlocal heard
    while not busy and heard < len(log):
      for name, g in ears():
        if alive.get(name) is g:
          hears(name, g, log[heard])
      heard += 1

  def hears(name, g, a):
    token, living = site.set(name), False
    busy.add(name)
    try:
      living = lives(g, a)
    finally:
      site.reset(token)
      busy.discard(name)
      if not living:
        alive.pop(name, None)

  def live(g, name):
    if name in alive or (name in outside and name in (OPERATOR, "record", "journal")):
      raise Refused(f"{name} hears")
    alive[name] = g
    hears(name, g, None)
    dispatch()

  globals().update(send=says, ask=asks, act=makes, drive=live)
  for who, hearer in [(OPERATOR, idle(OPERATOR)), ("record", door()), *outside.items(), ("journal", journal())]:
    live(hearer, who)
  return Act("chain1") if "chain1" in acts else chain("root")
