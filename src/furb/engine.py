import re
from asyncio import CancelledError, current_task, get_running_loop
from collections import Counter
from collections.abc import Callable
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template

WINDOW = 200000
OPERATOR = "operator"
WORLD = "world"
TIMEOUT = 600.0
site = ContextVar("site", default=OPERATOR)
modules, acts, asked, outcomes = {}, {}, {}, {}
type Show = Callable[[list[str]], list[int]]
type Filter = Callable[[list[tuple]], list[tuple]]


def send(kind, about, *words, by=""):
  raise RuntimeError("no life is living")


def ask(kind, on, *words):
  raise RuntimeError("no life is living")


def act(kind, on, life, *words):
  raise RuntimeError("no life is living")


def drive(g, name):
  raise RuntimeError("no life is living")


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


def gate(word: str, returns: str = "", on: str = "") -> list[str]:
  _, got = ask("gate", on, word, returns)
  tell("gate", ("returns", returns), body="\n".join(got or ()))
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
    case ("rung", _, by, _, "", *_) if id == who:
      id = by
  control("close", "closed", id, value)
  if under(who, id):
    raise CancelledError()


def debug(template: Template) -> None:
  if not (who := acting()):
    raise Refused("debug tells of an act, and there is none outside one")
  send("tell", who, [("debugged", [("id", who), (i.expression, i.value)], None) for i in template.interpolations])


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  return act("wait", on, ending(started(idle)), seconds)


def rung(word: str = "", retells: str = "", actor: str = "", returns: str = "", on: str = "") -> Act:
  def life(id):
    wants = None
    if word:
      told("opened", id, body=word)
      yield "ready", id, word, returns
    while True:
      match (yield):
        case ("answer", about, _, (_, content, _, _)) if about == id:
          yield "ready", id, "\n".join(x for x in content if isinstance(x, str)), returns
        case ("wants", about, _, call) if about == id:
          wants = call
        case ("done", about, _, value) if about == wants:
          wants = None
          yield "sent", id, value
        case ("ran", about, _, value) if about == id:
          if isinstance(value, BaseException):
            told("raised", id, ("type", type(value).__name__), ("message", str(value)))
          told("closed", id)
          yield "done", id, value
          return

  return act("rung", on, ending(pausing(life)), word, retells, actor, returns)


def prompt[T](shape: type[T] | None, message: str = "", to: str = "", on: str = "") -> Act[T]:
  named = shape if isinstance(shape, str) else shape.__name__ if isinstance(shape, type) else repr(shape)

  def life(id):
    asking = None
    told("opened", id, ("shape", named), ("message", message), ("to", to))
    while True:
      if to != OPERATOR and asking is None:
        asking = rung(actor=to, returns=named)
      match (yield):
        case ("done", about, *_) if about == asking:
          asking = None
        case ("write", qid, _, _, Text(path=path, content=word)) if path == id:
          rung(word)
          yield "done", qid, Text(path, word)

  return act("prompt", on, pausing(ending(started(life, to))), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act:
  if source and acts.get(source, ("",))[0] != "chain":
    raise Refused(f"{source} names no chain")

  def life(id):
    transcript, rungs, retold, paused, mine = [], {}, set(), set(), {id}
    asking, running, to_run, unseen = None, "", [], ""

    def hold(fact):
      if fact[1] not in retold:
        transcript.append(fact)
      return fact

    def each(*kinds):
      return [x for x in transcript if x[0] in kinds and x[1] in mine]

    def stands(name):
      return any(under(name, x) or x == id for x in paused)

    if source:
      _, theirs = ask("transcript", source, source)
      picked = (filter or (lambda x: x))([x for x in theirs if question(x)])
      transcript += [it for it in theirs if it[1] == source or any(under(one[1], it[1]) for one in picked)]
    hold(told("opened", id, ("label", label), ("source", source)))
    if source:
      _, standing = ask("stand", source)
      _, program = ask("program", source, source)
      for whose, said in program:
        retold.add(rung(said, whose, on=id))
    else:
      _, standing = ask("stand", id)
      hold(told("opened", id, *zip(("roster", "directory", "actor"), standing, strict=True)))
    _, directory, actor = standing
    bound = modules[id] = {**globals(), "__name__": id, "actor": actor}
    while True:
      a = yield
      if (a[3] if question(a) else scope(a[1])) != id:
        continue
      if question(a):
        mine.add(a[1])
      if not (a[0] == "tell" and a[2] == id) and not (a[0] == "done" and a[1] not in mine):
        hold(a)
      if question(a) and a[1] in acts and stands(a[1]):
        yield "pause", a[1], []
      if a[0] in ("ready", "done") and a[1] == asking:
        asking = None
      match a:
        case ("rung", rid, _, _, "", _, who, returns):
          unseen = ""
          hold(told("opened", rid, ("actor", who or bound["actor"]), ("says", f"close({returns})")))
        case ("ready", rid, _, word, returns):
          _, refusals = ask("gate", id, word, returns)
          if refusals:
            hold(told("refused", rid, body="\n".join(refusals)))
            close(Refused(), rid)
          else:
            rungs[rid] = word
            to_run.append(rid)
        case ("prompt", pid, _, _, _, _, to) if to and offered(standing, to) is None:
          close(Refused(f"{to} is no actor of the roster"), pid)
        case ("program", qid, _, _, about) if about in mine:
          yield "done", qid, [(one, rungs[one]) for one in rungs if about == id or under(one, about)]
        case ("read", qid, _, _, path) if path in mine:
          yield "done", qid, Text(path, "\n".join(rungs[one] for one in rungs if path == id or under(one, path)))
        case ("transcript", qid, _, _, about) if about in mine:
          cut = next((i + 1 for i, x in enumerate(transcript) if question(x) and x[1] == about), len(transcript))
          yield "done", qid, transcript[:cut]
        case ("write", qid, _, _, Text(path=path, content=word)) if path == id:
          rung(word, on=id)
          yield "done", qid, Text(path, "\n".join((*rungs.values(), word)))
        case ("stand", qid, *_):
          yield "done", qid, standing
        case ("cd", qid, _, _, path):
          yield "done", qid, path
        case ("cwd", qid, *_):
          yield "done", qid, next((x[4] for x in reversed(transcript) if x[0] == "cd"), directory)
        case ("turns", qid, *_):
          yield "done", qid, turns_of(transcript)
        case ("done", about, _, value):
          if isinstance(value, Exception) and about in rungs:
            bound["raised"] = value
          if about == running:
            running = ""
          if not isinstance(value, CancelledError) and about in acts and acts[about][2] in rungs:
            unseen = about
        case ("wants", *_):
          running = ""
        case ("pause", about, by, _) if by != id:
          paused.add(about)
        case ("wake", about, *_):
          paused.discard(about)
      spoken = {x[1] for x in each("done", "ask", "ready")}
      mute = [x for x in each("rung") if not x[4] and x[1] not in spoken and not stands(x[1])]
      if not asking and mute:
        _, asking, _, _, _, _, who, _ = mute[0]
        q, held = ask("holds", id, asking)
        hold(q)
        if not held:
          yield "ask", asking, id, who or bound["actor"], turns_of(transcript)
      if to_run and not running:
        yield "run", (running := to_run.pop(0)), id, rungs[running]
      if unseen and not running and all(x[1] in spoken for x in each("prompt")):
        prompt(None, f"{unseen} is done", on=id)
        unseen = ""

  return act("chain", on, life, label, source)


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  def life(id):
    here = scope(id)
    if not (usd or share) or (usd or 0) < 0 or not 0 <= (share or 0) <= 1:
      yield "done", id, Refused(f"{usd} dollars and {share} of the window is no ceiling")
      return
    _, transcript = ask("transcript", here, here)
    if not isinstance(transcript, list):
      yield "done", id, transcript
      return
    _, standing = ask("stand", here)
    actor, spent, filled = "", 0.0, 0.0
    for old in transcript:
      match old:
        case ("grant", gid, *_) if gid != id and gid not in outcomes:
          close(None, gid)
        case ("ask", _, _, _, who, _):
          actor = who
    told("opened", id, ("usd", usd), ("share", share))
    while True:
      match (yield):
        case ("ask", _, _, whose, who, _) if whose == here:
          actor = who
        case ("answer", about, _, (_, _, (seen, _, _, _, dollars), _)) if scope(about) == here:
          spent, filled = spent + dollars, seen / (offered(standing, actor) or WINDOW)
          told("ledger", about, ("spent", spent), ("filled", filled))
          if (usd is not None and spent >= usd) or (share is not None and filled >= share):
            pause(here)

  return act("grant", on, ending(life), usd, share)


def bash(
  command: str,
  fed: bool = False,
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  def life(id):
    out, err, stdin = (f"{id}/{one}" for one in ("stdout", "stderr", "stdin"))
    streams, gone, mute = {out: Text(out), err: Text(err)}, "", "" if fed else "the command is not fed"
    if show is not HIDDEN:
      told("opened", id, ("command", command))
    while True:
      match (yield):
        case ("merged", qid, _, _, about) if about == id:
          yield "done", qid, show_err is None
        case ("read", qid, _, _, path) if path in streams:
          yield "done", qid, streams[path]
        case ("write", qid, _, _, Text(path=path, content=text)) if path == stdin:
          if takes := not (gone or mute):
            yield "feed", id, text or None
            mute = "" if text else "its stdin is closed"
          yield "done", qid, Text(path, text) if takes else Refused(f"{stdin} takes no word, since {gone or mute}")
        case _ if gone:
          continue
        case ("peek", qid, _, _, at) if at == id:
          yield "done", qid, Exit(None, *streams.values())
        case ("out", about, _, text, stream) if about == id:
          into = err if stream == "stderr" and show_err is not None else out
          streams[into] = streams[into].grow(text)
        case ("exited", about, _, code) if about == id:
          gone = "the command ended"
          if show is not HIDDEN:
            shows = [(streams[one], what) for one, what in ((out, show), (err, show_err)) if what not in (None, HIDDEN)]
            told("closed", id, ("code", code), body=shows)
          yield "done", id, Exit(code, *streams.values())
        case ("cancel" | "close", *_) as a if covers(a, id):
          gone = "the command ended"
          yield "done", id, ended(a, id)

  return act("bash", on, pausing(started(life)), command, fed, timeout)


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
      raise Refused(f"{self.path} has no lines {lo} to {hi}")
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
  def __await__(self):
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
  return bool(of) and (lineage(name) + ".").startswith(lineage(of) + ".")


def acting():
  return site.get() if "://" in site.get() else ""


def question(a):
  return a[1].startswith(a[0] + "://")


def scope(name):
  match acts.get(name) or asked.get(name):
    case ("chain", id, *_):
      return id
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
  text, show = pair
  old, lines = seen.setdefault(text.path, {}), text.lines
  picked = show(lines)
  new = {i: line for i in picked if old.get(i) != (line := lines[i - 1])}
  old.update(new)
  return ("shown", [("path", text.path), ("known", len(picked) - len(new))], "\n".join(f"{i} {new[i]}" for i in new))


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
  who, effort = to.partition("/")[::2]
  roster, _, _ = standing
  return next((w for name, efforts, w in roster if name == who and (not effort or effort in efforts)), None)


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


def pausing(life):
  def lived(id):
    g, held, paused = life(id), [], False
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


def ending(life):
  def lived(id):
    g, a = life(id), None
    while lives(g, a):
      match a := (yield):
        case ("done", about, *_) if about == id:
          return
        case ("cancel" | "close", *_) if covers(a, id):
          yield "done", id, ended(a, id)
          return

  return lived


def started(life, to=OPERATOR):
  def lived(id):
    if to == OPERATOR and not ask("holds", "", id)[1]:
      yield "start", id
    yield from life(id)

  return lived


def boot(record=(), **outside):
  get_running_loop()
  kept, past = list(record), len(record)
  for table in (modules, acts, asked, outcomes):
    table.clear()
  log, alive, made, busy, hearing, heard = [], {}, Counter(), set(), set(), 0

  def door():
    answers = {e[1][1]: e[2] if len(e) == 3 else e[1][3] for e in kept if len(e) == 3 or e[1][0] == "done"}
    while True:
      match a := (yield):
        case ("holds", qid, _, _, about):
          yield "done", qid, [e[1] for e in kept[:past] if e[1][1] == about]
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
      while cursor < past:
        match kept[cursor]:
          case (_, q, _):
            said.add(q[1])
          case (_, (kind, _, "operator", on, *words) as then) if question(then) and (not on or on in said):
            token = site.set(OPERATOR)
            try:
              globals()[kind](*words, on=on)
            finally:
              site.reset(token)
          case (before, (_, about, *_)) if not (about in said and before in said):
            break
          case (_, (kind, about, by, *words) as then) if not question(then) and kind != "done":
            copies.add(id(send(kind, about, *words, by=by)))
        cursor += 1
      a = yield
      match a:
        case (kind, qid, by, *words) if question(a):
          if qid in acts and qid in facts and list(words[1:]) != list(facts[qid][4:]):
            raise Drift(f"{kind} {words[1:]} is not the {qid} the record holds")
          if qid in acts or by.startswith("rung://"):
            said.add(qid)
            facts.setdefault(qid, a)
            if by == OPERATOR and qid in acts and qid not in held:
              held.add(qid)
              keep((after, a))
          if qid in acts:
            after = qid
        case (kind, about, by, *words) if by in (WORLD, OPERATOR) and about in facts and id(a) not in copies:
          answer = [words[0]] if kind == "done" and about in asked else []
          if about not in held:
            held.add(about)
            keep((after, facts[about], *answer))
          if not answer:
            keep((after, a))

  def named(kind, by):
    stem = by
    match acts.get(by):
      case ("rung", _, _, _, _, retells, *_) if retells:
        stem = retells
    made[by] += 1
    return f"{kind}://{lineage(stem)}.{made[by]}"

  def says(kind, about, *words, by=""):
    a = (kind, about, by or site.get(), *words)
    if kind == "done" and about not in outcomes and (about in acts or about in asked):
      outcomes[about] = words[0]
    log.append(a)
    dispatch()
    return a

  def asks(kind, on, *words):
    by = site.get()
    a = (kind, named(kind, by), by, on or scope(by), *words)
    asked[a[1]] = a
    if isinstance(got := put(a), Refused) and kind != "peek":
      raise got
    return a, got

  def makes(kind, on, life, *words):
    by = site.get()
    a = (kind, named(kind, by), by, on or scope(by), *words)
    if not a[3] and kind != "chain":
      raise Refused(f"{a[1]} names no chain")
    if acts.setdefault(a[1], a) is not a:
      return Act(a[1])
    log.append(a)
    live(life(a[1]), a[1])
    dispatch()
    return Act(a[1])

  def ears():
    return sorted(alive.items(), key=lambda ear: (ear[0] not in acts, ear[0] in outside))

  def put(a):
    for name, g in ears():
      if name not in busy and a[1] not in outcomes:
        hears(name, g, a)
    dispatch()
    return outcomes.get(a[1])

  def dispatch():
    nonlocal heard
    while not busy and heard < len(log):
      for name, g in ears():
        if alive.get(name) is g and name not in hearing:
          hearing.add(name)
          hears(name, g, log[heard])
      heard += 1
      hearing.clear()

  def hears(name, g, a):
    token = site.set(name)
    busy.add(name)
    try:
      if not lives(g, a):
        alive.pop(name, None)
    except BaseException:
      alive.pop(name, None)
      raise
    finally:
      site.reset(token)
      busy.discard(name)

  def live(g, name):
    if name in alive:
      raise Refused(f"{name} hears already")
    alive[name] = g
    hears(name, g, None)
    dispatch()

  for who in outside.keys() & {OPERATOR, "record", "journal"}:
    raise Refused(f"{who} hears already")
  globals().update(send=says, ask=asks, act=makes, drive=live)
  for who, hearer in {OPERATOR: idle(OPERATOR), "record": door(), **outside, "journal": journal()}.items():
    live(hearer, who)
  return Act(root) if (root := f"chain://{OPERATOR}.1") in acts else chain("root")
