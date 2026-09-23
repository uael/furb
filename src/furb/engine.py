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
    tell("read", ("path", path), body=showing(got, show))
  return got


def write(text: Text, on: str = "") -> Text:
  _, got = ask("write", on, text)
  tell("write", ("path", text.path), body=showing(got, differs(text.lines)))
  return got


def peek(at: str, on: str = "") -> object:
  _, got = ask("peek", on, at)
  tell("peek", ("at", at))
  return got


def turns(on: str = "") -> list[tuple]:
  _, got = ask("turns", on)
  tell("turns", ("turns", len(got or ())))
  return got


def clock(on: str = "") -> float:
  _, got = ask("clock", on)
  tell("clock", ("at", got))
  return got


def chance(on: str = "") -> float:
  _, got = ask("chance", on)
  tell("chance", ("drew", got))
  return got


def gate(word: str, on: str = "") -> list[str]:
  _, got = ask("gate", on, word, ask("program", on)[1] or {})
  tell("gate", body="\n".join(got or ()))
  return got


def cd(path: str, on: str = "") -> str:
  _, got = ask("cd", on, path)
  tell("cd", ("path", path))
  return got


def cwd(on: str = "") -> str:
  _, got = ask("cwd", on)
  tell("cwd", ("path", got))
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
  id = id or who
  match acts.get(who):
    case ("rung", _, by, _, _, retells, *_) if id == who:
      if retells:
        raise CancelledError()
      if by.startswith("prompt://"):
        id = by
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
  send("tell", who, [("debugged", [("id", who), (i.expression, i.value)], None) for i in template.interpolations])


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  return act("wait", on, ending(started(idle)), seconds)


def rung(word: str = "", retells: str = "", actor: str = "", returns: str = "", on: str = "") -> Act:
  def ear(id):
    wants = None
    if word:
      told("opened", id, body=word)
      yield "ready", id, word
    while True:
      match (yield):
        case ("answer", about, _, (_, content, _, _)) if about == id:
          yield "ready", id, "\n".join(x for x in content if isinstance(x, str))
        case ("wants", about, _, call) if about == id:
          wants = call
        case ("done", about, _, value) if about == wants:
          wants = None
          yield "sent", id, value
        case ("ran", about, _, value) if about == id:
          if retells and isinstance(value, CancelledError):
            value = None
          if isinstance(value, BaseException):
            told("raised", id, ("type", type(value).__name__), ("message", str(value)))
          told("closed", id)
          yield "done", id, value
          return

  return act("rung", on, ending(pausing(ear)), word, retells, actor, returns)


def prompt[T](shape: type[T] | object, message: str = "", to: str = "", on: str = "") -> Act[T]:
  named = (
    shape
    if isinstance(shape, str)
    else shape.__name__
    if isinstance(shape, type)
    else re.sub(r"[\w.:/]*\.", "", repr(shape))
  )

  def ear(id):
    asking = None
    told("opened", id, ("shape", named), ("message", message), ("to", to))
    while True:
      if to != OPERATOR and not asking:
        asking = rung(actor=to, returns=named)
      match (yield):
        case ("done", about, *_) if about == asking:
          asking = None

  return act("prompt", on, pausing(ending(started(ear, to))), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act:
  if source and scope(source) != source:
    raise Refused(f"no chain {source}")

  def ear(id):
    transcript, rungs, refused, retold, paused, mine = [], {}, {}, set(), set(), {id}
    waiting, asking, running, to_run, unseen = [], None, "", [], ""

    def stands(name):
      return any(under(name, x) or x == id for x in paused)

    def program():
      return {one: rungs[one] for one in rungs if one not in refused}

    def answers():
      match a:
        case ("program", *_):
          return program()
        case ("read", _, by, _, path) | ("write", _, by, _, Text(path)) if path in mine and path.startswith(
          "prompt://"
        ):
          if a[0] == "write":
            replay(path, a[4].content, by)
            return a[4]
          return Text(
            path, "\n".join(said for one, said in rungs.items() if one != by and under(acts[one][5] or one, path))
          )
        case ("transcript", _, _, _, at) if at in mine:
          return transcript[
            : transcript.index(where) + 1 if (where := acts.get(at) or asked.get(at)) in transcript else None
          ]
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
        if of and under(donor, of):
          if whose == by:
            continue
          if not (words + "\n").startswith(said + "\n"):
            break
          words = words[len(said) + 1 :]
        retold.add(rung(said, donor))
      rungs.clear()
      to_run.clear()
      modules[id] = {**globals(), "__name__": id, "actor": standing[2], "raised": None}
      if words:
        with site.set(of):
          rung(words)

    if source:
      _, theirs = ask("transcript", source, source)
      picked = (filter or (lambda x: x))([x for x in theirs if question(x)])
      transcript += [it for it in theirs if it[1] == source or any(under(one[1], it[1]) for one in picked)]
      rungs.update(ask("program", source)[1])
    start = len(transcript)
    transcript.append(told("opened", id, ("label", label), ("source", source)))
    _, standing = ask("stand", source or id)
    if not source:
      transcript.append(told("opened", id, *zip(("roster", "directory", "actor"), standing, strict=True)))
    replay()
    while True:
      a = yield
      if (a[3] if question(a) else scope(a[1])) != id:
        continue
      if question(a):
        mine.add(about := a[1])
        if about in acts and stands(about):
          yield "pause", about, []
        if (got := answers()) is not None:
          yield "done", about, got
      if a[1] in mine and a[1] not in retold and not (a[0] == "tell" and a[2] == id):
        transcript.append(a)
      if a[0] in ("ready", "done") and a[1] == asking:
        asking = None
      match a:
        case ("rung", rid, _, _, "", _, who, returns):
          unseen = ""
          waiting.append(rid)
          transcript.append(told("opened", rid, ("actor", who or modules[id]["actor"]), ("says", f"close({returns})")))
        case ("ready", rid, _, word):
          retells = acts[rid][5]
          found = refused.get(retells, []) if retells else ask("gate", id, word, program())[1]
          rungs[rid] = word
          if found:
            refused[rid] = found
            if not retells:
              transcript.append(told("refused", rid, body="\n".join(found)))
            close(Refused(), rid)
          else:
            to_run.append(rid)
        case ("prompt", pid, _, _, _, _, to) if to and offered(standing, to) is None:
          close(Refused(f"{to} no actor"), pid)
        case ("done", about, _, value):
          if about in waiting:
            waiting.remove(about)
          if isinstance(value, Exception) and about in program():
            modules[id]["raised"] = value
          if about == running:
            running = ""
          if not isinstance(value, CancelledError) and about in acts and acts[about][2] in rungs:
            unseen = about
        case ("wants", *_):
          running = ""
        case ("pause", about, *_) if a[2] != id:
          paused.add(about)
        case ("wake", about, *_):
          paused.discard(about)
      if not asking and (mute := [x for x in waiting if not stands(x)]):
        waiting.remove(asking := mute[0])
        q, held = ask("holds", id, asking)
        transcript.append(q)
        if not held:
          yield "ask", asking, id, acts[asking][6] or modules[id]["actor"], turns_of(transcript)
      if to_run and not running:
        yield "run", (running := to_run.pop(0)), id, rungs[running], acts[running][5]
      if unseen and not running and all(x[1] in outcomes for x in transcript[start:] if x[0] == "prompt"):
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
    spent, filled = 0.0, 0.0
    for old in transcript:
      match old:
        case ("grant", gid, *_) if gid != id and gid not in outcomes:
          close(None, gid)
    told("opened", id, ("usd", usd), ("share", share))
    while True:
      match (yield):
        case ("answer", about, _, (_, _, (seen, _, _, _, dollars), _)) if scope(about) == here:
          spent, filled = (
            spent + dollars,
            seen / (offered(standing, acts[about][6] or modules[here]["actor"]) or WINDOW),
          )
          told("ledger", about, ("spent", spent), ("filled", filled))
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
    out, err, stdin = f"{id}/stdout", f"{id}/stderr", f"{id}/stdin"
    streams, gone, mute = {out: Text(out), err: Text(err)}, "", "" if fed else "not fed"
    if show is not HIDDEN:
      told("opened", id, ("command", command))
    while True:
      match (yield):
        case ("merged", qid, _, _, about) if about == id:
          yield "done", qid, show_err is None
        case ("read", qid, _, _, path) if path in streams:
          yield "done", qid, streams[path]
        case ("write", qid, _, _, Text(path, text) as took) if path == stdin:
          if takes := not (gone or mute):
            yield "feed", id, text or None
            mute = "" if text else "closed"
          yield "done", qid, took if takes else Refused(gone or mute)
        case _ if gone:
          continue
        case ("peek", qid, _, _, at) if at == id:
          yield "done", qid, Exit(None, *streams.values())
        case ("out", about, _, text, stream) if about == id:
          into = err if stream == "stderr" and show_err is not None else out
          streams[into] = streams[into].grow(text)
        case ("exited", about, _, code) if about == id:
          gone = "ended"
          if show is not HIDDEN:
            shows = [(streams[one], what) for one, what in ((out, show), (err, show_err)) if what not in (None, HIDDEN)]
            told("closed", id, ("code", code), body=shows)
          yield "done", id, Exit(code, *streams.values())
        case ("cancel" | "close", *_) as a if covers(a, id):
          gone = "ended"
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
      if self.startswith("chain://"):
        raise Refused(f"{self} never settles")
      return (yield self)
    f = get_running_loop().create_future()

    def waits():
      while self not in outcomes:
        yield
      got = outcomes[self]
      f.set_exception(got) if isinstance(got, BaseException) else f.set_result(got)
      f.exception()

    drive(waits(), f"waits://{lineage(self)}.{id(current_task())}")
    return (yield from f.__await__())


class Refused(Exception): ...


class Drift(Exception): ...


def lineage(name):
  return name.rpartition("://")[2]


def under(name, of):
  return bool(of) and f"{lineage(name)}.".startswith(f"{lineage(of)}.")


def acting():
  return site.get() if "://" in site.get() else ""


def question(a):
  return a[1].startswith(f"{a[0]}://")


def scope(name):
  match acts.get(name) or asked.get(name):
    case ("chain", *_):
      return name
    case (_, _, _, on, *_):
      return on
  return ""


def tell(name, *attrs, body=None):
  if who := acting():
    send("tell", who, [(name, [*attrs], body)])


def told(name, id, *attrs, body=None):
  return send("tell", id, [(name, [("id", id), *attrs], body)])


def control(kind, name, id, *words):
  return send(kind, id, *words, [(name, [("over", id)], repr(words[0]) if words else None)])


def showing(got, show):
  return [(got, show)] if isinstance(got, Text) else repr(got)


def shown(pair, seen):
  match pair:
    case (Text() as text, show):
      old, lines = seen.setdefault(text.path, {}), text.lines
      picked = show(lines)
      new = {i: line for i in picked if old.get(i) != (line := lines[i - 1])}
      old.update(new)
      return (
        "shown",
        [("path", text.path), ("known", len(picked) - len(new))],
        "\n".join(f"{i} {line}" for i, line in new.items()),
      )
  return pair


def turns_of(heard):
  seen, folded, tags, cut = {}, [], [], 0
  for it in heard:
    match it:
      case ("holds", _, by, on, _) if by == on:
        cut = len(tags)
      case ("answer", _, _, turn):
        folded += [("user", tags[:cut], None, None), turn]
        del tags[:cut]
      case ("tell" | "pause" | "wake" | "cancel", _, _, says) | ("close", _, _, _, says):
        tags += [
          (n, attrs, [shown(one, seen) for one in body] if isinstance(body, list) else body) for n, attrs, body in says
        ]
  return [*folded, ("user", tags, None, None)]


def offered(standing, to):
  return next((w for name, efforts, w in standing[0] if to in (name, *(f"{name}/{e}" for e in efforts))), None)


def covers(a, id):
  about = a[1]
  reaches = under(id, about) or scope(id) == about
  return reaches and (id == about or id.startswith("rung://")) if a[0] == "close" else reaches


def ended(a, id):
  return a[3] if a[0] == "close" and a[1] == id else CancelledError()


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
    g, held, paused = ear(id), [], False
    lives(g, None)
    while True:
      match a := (yield):
        case ("pause" | "wake", *_) if covers(a, id):
          paused = a[0] == "pause"
        case ("cancel" | "close", _, by, *_) if not under(by, id):
          if not lives(g, a):
            return
        case _ if not (paused and asked.get(a[1]) is a):
          held.append(a)
      while held and not paused:
        if not lives(g, held.pop(0)):
          return

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
  kept, past = list(record), len(record)
  for table in (modules, acts, asked, outcomes):
    table.clear()
  log, alive, made, busy, heard = [], {}, Counter(), set(), 0

  def door():
    answers = {e[1][1]: e[2] for e in kept if len(e) == 3}
    while True:
      match a := (yield):
        case ("holds", qid, _, _, about):
          yield "done", qid, [e[1] for e in kept[:past] if e[1][1] == about]
        case ("peek", qid, _, _, at):
          yield "done", qid, outcomes.get(at)
        case (_, qid, *_) if question(a) and qid in answers:
          yield "done", qid, answers[qid]

  def journal():
    facts = {one[1]: one for _, one, *_ in kept if question(one)}
    said, cursor, after, held = set(), 0, "", set(facts)

    def keep(entry):
      kept.append(entry)
      send("keep", "", entry)

    while True:
      while cursor < past and heard == len(log):
        match kept[cursor]:
          case (_, q, _):
            said.add(q[1])
          case (_, (kind, _, "operator", on, *words) as then) if question(then) and (not on or on in said):
            with site.set(OPERATOR):
              made[OPERATOR] = int(then[1].rpartition(".")[2]) - 1
              globals()[kind](*words, on=on)
          case (before, (_, about, *_)) if not (about in said and before in said):
            break
          case (_, (kind, about, by, *words) as then) if not question(then):
            held.add(id(send(kind, about, *words, by="record" if kind == "done" else by)))
        cursor += 1
      a = yield
      match a:
        case (_, qid, by, *words) if question(a):
          ours = qid in acts
          if ours and qid in facts and words[1:] != [*facts[qid][4:]]:
            raise Drift(f"{qid} drifts")
          if ours or by.startswith("rung://"):
            said.add(qid)
            facts.setdefault(qid, a)
            if by == OPERATOR and ours and qid not in held:
              held.add(qid)
              keep((after, a))
          if ours:
            after = qid
        case (kind, about, by, *words) if by in (WORLD, OPERATOR) and about in facts and id(a) not in held:
          answer = [words[0]] if kind == "done" and about in asked else []
          if about not in held:
            held.add(about)
            keep((after, facts[about], *answer))
          if not answer:
            keep((after, a))

  def named(kind, on, *words):
    stem = by = site.get()
    if (a := acts.get(by)) and a[0] == "rung" and a[5]:
      stem = a[5]
    made[by] += 1
    return (kind, f"{kind}://{lineage(stem)}.{made[by]}", by, on or scope(by), *words)

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
    if isinstance(got := put(a), Refused) and kind != "peek":
      raise got
    return a, got

  def makes(kind, on, ear, *words):
    a = named(kind, on, *words)
    id = a[1]
    if not a[3] and kind != "chain":
      raise Refused(f"no chain {id}")
    if acts.setdefault(id, a) is a:
      log.append(a)
      live(ear(id), id)
      dispatch()
    return Act(id)

  def ears():
    return sorted(alive.items(), key=lambda pair: (pair[0] not in acts, pair[0] in outside))

  def put(a):
    for name, g in ears():
      if name not in busy and a[1] not in outcomes:
        hears(name, g, a)
    dispatch()
    return outcomes.get(a[1])

  def dispatch():
    nonlocal heard
    while not busy:
      if heard == len(log):
        if g := alive.get("journal"):
          hears("journal", g, None)
        if heard == len(log):
          break
      for name, g in ears():
        if alive.get(name) is g:
          hears(name, g, log[heard])
      heard += 1

  def hears(name, g, a):
    with site.set(name):
      living = False
      busy.add(name)
      try:
        living = lives(g, a)
      finally:
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
  return Act(root) if (root := f"chain://{OPERATOR}.1") in acts else chain("root")
