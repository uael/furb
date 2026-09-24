import re
from asyncio import CancelledError, current_task, get_running_loop
from collections import Counter
from collections.abc import Callable, Generator
from contextvars import ContextVar
from string.templatelib import Template
from typing import Any

OPERATOR = "operator"
WORLD = "world"
site = ContextVar("site", default=OPERATOR)
modules, acts, asked, outcomes = {}, {}, {}, {}
raised = None
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


def take(*ids: str, inside: bool = True) -> Filter:
  return lambda facts: [a for a in facts if any(under(a[1], one) for one in ids) == inside]


def peek(at: str, on: str = "") -> object:
  return ask("peek", on, at)[1]


def turns(on: str = "") -> list[tuple]:
  return ask("turns", on)[1]


def clock(on: str = "") -> float:
  return ask("clock", on)[1]


def chance(on: str = "") -> float:
  return ask("chance", on)[1]


def gate(word: str, on: str = "") -> list[str]:
  return ask("gate", on, unquoted(word), ask("program", on)[1] or {})[1]


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
    case ("prompt", _, _, on, shape, *_) if not isinstance(value, BaseException):
      try:
        fits = isinstance(value, (s := eval(shape, modules[on])) or object)
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
    told(who, f"debugged {i.expression} = {i.value!r}")


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  return act("wait", on, started(ending(idle)), seconds)


def rung(word: str = "", retells: str = "", actor: str = "", on: str = "") -> Act:
  def ear(id):
    if word:
      told(id, "", word)
      yield "ready", id, word
    while True:
      match (yield):
        case ("answer", about, _, (_, said, _, _)) if about == id:
          yield "ready", id, said
        case ("wants", about, _, call) if about == id:
          while call not in outcomes:
            yield
          yield "sent", id, outcomes[call]
        case ("ran", about, _, value) if about == id:
          if retells and isinstance(value, CancelledError):
            value = None
          if isinstance(value, BaseException):
            told(id, "raised " + re.sub(r"^[\w.]*\.", "", repr(value)))
          yield "done", id, value
          return

  if not (word or actor):
    actor = modules.get(on or scope(site.get()), {}).get("actor")
  return act("rung", on, ending(pausing(ear)), word, retells, actor)


def prompt[T](shape: type[T] | object, message: str = "", to: str = "", on: str = "") -> Act[T]:
  named = shape if isinstance(shape, str) else re.sub(r"<class '|'>|[\w.:/]*\.", "", repr(shape))
  actor = to or modules.get(on or scope(site.get()), {}).get("actor")

  def ear(id):
    told(id, message, bound(id, named))
    while actor != OPERATOR:
      asking = rung(actor=actor)
      while asking not in outcomes:
        yield
    yield from idle(id)

  return act("prompt", on, started(pausing(ending(ear)), actor), named, message, to)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act:
  if scope(source) != source:
    raise Refused(f"no chain {source}")

  def ear(id):
    transcript, rungs, program, refused, own, mine, controls = [], {}, {}, set(), set(), {id: (None, None)}, []
    waiting, asking, running, to_run, unseen, pending = {}, "", "", [], "", None

    def hold(*paragraph):
      transcript.append(told(*paragraph))

    def paused(name):
      return [x[0] for x in controls if covers(x, name)][-1:] == ["pause"]

    def takes():
      modules[id]["actor"] = standing[2]
      hold(id, f"roster {standing[0]!r}", headed(id, f"cwd {standing[1]}"), headed(id, f"actor {standing[2]}"))

    def answers():
      match a:
        case ("ladder", _, by, _, path, *word) if path in mine and question(("prompt", path)):
          if word:
            replay(path, word[0], by)
            return word[0]
          return "\n".join(said for one, said in rungs.items() if one != by and under(acts[one][5] or one, path))
        case ("transcript", *_, at) if at in mine:
          return transcript[: mine[at][0]]
      match a[0]:
        case "program":
          return {**program}
        case "stand":
          return mine.get(a[-1], (0, standing))[1] or standing
        case "turns":
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
      modules[id] = {**globals(), "__name__": id, "actor": standing[2]}
      if words:
        with site.set(of):
          rung(words)

    if source:
      _, theirs = ask("transcript", source, source)
      picked = (filter or (lambda x: x))([x for x in theirs if question(x)])
      transcript += [
        it for it in theirs if it[0] == "holds" or it[1] == source or any(under(one[1], it[1]) for one in picked)
      ]
      rungs.update(ask("program", source)[1])
    hold(id, f"{label} from {source}".strip() if source else label, bound(id))
    standing = ask("stand", source or id)[1]
    replay()
    source or takes()
    while True:
      a = yield
      if (a[3] if question(a) else scope(a[1])) != id:
        continue
      if question(a):
        if (about := a[1]) not in own:
          mine[about] = len(transcript) + 1, standing
        if about in acts and paused(about):
          yield "pause", about, []
        if (got := answers()) is not None:
          yield "done", about, got
      if a[1] in mine and not (a[0] in ("tell", "done") and a[2] == id):
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
            hold(rid, "refused", commented("\n".join(found)))
          if found:
            refused.add(rid)
            close(Refused(), rid)
          else:
            program[rid] = unquoted(word)
            to_run.append(rid)
        case ("done", about, _, value):
          waiting.pop(about, None)
          if isinstance(value, Exception) and about in program:
            modules[id]["raised"] = value
          if not isinstance(value, CancelledError) and about in acts and acts[about][2] in rungs:
            unseen = about
        case ("stood", _, _, standing):
          takes()
        case ("wants" | "ran", *_):
          running = ""
        case ("pause" | "wake", *_):
          controls.append(a)
          if a[0] == "wake" and pending and a[2] != "record" and covers(a, asking):
            pending = []
      if asking not in waiting and (asking := next((x for x in waiting if not paused(x)), "")):
        maker, pending = acts[asking][2], None
        if offered(standing, to := acts[asking][6]) is None:
          close(Refused(f"{to} no actor"), maker if question(("prompt", maker)) else asking)
        else:
          q, pending = ask("holds", id, asking)
          hold(asking, f"advance on {maker}")
          transcript.append(q)
          if binds := "\n".join(re.findall(r"(?m)^\w+: Act\[.*\] = Act\('\w+'\)$", turns_of(transcript)[-1][1])):
            own.add(rung(binds))
          unseen = ""
      if pending == []:
        pending = None
        yield "ask", asking, id, to, turns_of(transcript)
      if to_run and not running:
        yield "run", (running := to_run.pop(0)), id, program[running], acts[running][5]
      if unseen and not running and all(x in outcomes for x in mine if question(("prompt", x))):
        prompt(None, f"{unseen} done", on=id)
        unseen = ""

  return act("chain", on, ear, label, source)


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
      f.set_result(outcomes[self])

    drive(g := waits(), f"{self} waits {id(current_task())}")
    try:
      got = yield from f
    finally:
      g.close()
    if isinstance(got, BaseException):
      raise got
    return got


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
  return any(x not in outcomes and covers((kind, id), x) for x in acts) and send(
    kind, id, *words, [headed(id, " ".join([name, *[repr(x) for x in words]]))]
  )


def headed(name, text=""):
  return "#" + commented(f"{name} {text}".rstrip())[2:]


def commented(text):
  return "\n".join(f"# {x}" if x else "#" for x in str(text).split("\n"))


def bound(id, of="object"):
  return f"{id}: Act[{of}] = Act({id!r})"


def shown(note, seen):
  match note:
    case (str(path), str(content), show):
      old, lines = seen.setdefault(path, {}), content.splitlines()
      picked = show(lines)
      new = {i: line for i in picked if old.get(i) != (line := lines[i - 1])}
      old.update(new)
      return commented(f"{path}, {len(picked) - len(new)} known" + "".join(f"\n{i} {line}" for i, line in new.items()))
  return note


def turns_of(heard):
  seen, folded, user, cut = {}, [], [], 0
  for it in heard:
    match it:
      case ("holds", _, by, on, _) if by == on:
        cut = len(user)
      case ("answer", _, _, turn):
        folded += [("user", "\n\n".join(user[:cut]), None, None), turn]
        del user[:cut]
      case (kind, *_, notes) if kind in ("tell", "pause", "wake", "cancel", "close") and notes:
        user.append("\n".join([shown(x, seen) for x in notes]))
  return [*folded, ("user", "\n\n".join(user), None, None)]


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
        case ("cancel" | "close", _, by, *_) if covers(a, id) and not under(by, id):
          if not lives(g, a):
            return
        case _ if not (paused and asked.get(a[1]) is a):
          held.append(a)

  return lived


def ending(ear):
  def lived(id):
    g, a = ear(id), None
    while lives(g, a):
      a = yield
      if id in outcomes:
        return
      if a[0] in ("cancel", "close") and covers(a, id):
        yield "done", id, ended(a, id)
        return

  return lived


def started(ear, to=OPERATOR):
  def lived(id):
    if (pending := to == OPERATOR and ask("holds", "", id)[1]) == []:
      yield "start", id
    g, a = ear(id), None
    while lives(g, a):
      a = yield
      if pending and a[0] == "wake" and a[2] != "record" and covers(a, id) and id not in outcomes:
        pending = None
        yield "start", id

  return lived


def boot(record=(), **outside):
  get_running_loop()
  for table in (modules, acts, asked, outcomes):
    table.clear()
  log, alive, made, born, busy, left = [], {}, Counter(), {}, set(), []

  def given():
    answers = {q[1]: got for q, *got in record if got}
    while True:
      match (yield):
        case ("holds", qid, _, _, about):
          yield "done", qid, [q for q, *_ in record if q[1] == about]
        case ("peek", qid, _, _, at):
          yield "done", qid, outcomes.get(at)
        case (_, qid, *_) if qid in answers:
          yield "done", qid, *answers.pop(qid)

  def journal():
    kept, due = {q[1]: q for q, *_ in record if question(q)}, [None, *reversed(record)]

    def keep(name, *got):
      if name not in kept:
        send("keep", "", (kept.setdefault(name, acts.get(name) or asked[name]), *got))

    while True:
      while due and not log:
        match due.pop():
          case None if modules:
            now = ask("stand", "")[1]
            for on in modules:
              now == ask("stand", on)[1] or send("stood", on, now)
          case ((kind, about, _, *words) as fact,) if not question(fact):
            scope(about) and send(kind, about, *words, by="record")
          case ((kind, qid, by, on, *words),) if qid not in acts:
            if by in kept or scope(on) != on:
              born.setdefault(kind, {})[qid] = qid
              continue
            if not callable(verb := modules.get(on, globals()).get(kind)) or re.fullmatch(r"\w+?(\d+|@.+)", by):
              raise Drift(f"{qid} drifts")
            with site.set(by):
              verb(*words, on=on)
      match a := (yield):
        case (_, qid, by, _, *words) if a is acts.get(qid):
          if words != [*kept.get(qid, a)[4:]]:
            raise Drift(f"{qid} drifts")
          if by not in acts:
            keep(qid)
        case ("start" | "ask", about, *_):
          keep(about)
        case ("done", about, by, value) if by == WORLD and about in asked and asked[about][2] in acts:
          keep(about, value)
        case (_, about, by, *_) if by in (WORLD, OPERATOR, "journal") and about in acts:
          keep(about)
          send("keep", "", (a,))

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
      a[1] in outcomes or hears(name, g, a)
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
    while not busy:
      if not left:
        if not log and (g := alive.get("journal")):
          hears("journal", g, None)
        if not log:
          return
        left.extend(ears()[::-1])
      name, g = left.pop()
      hears(name, g, log[0] if left else log.pop(0))

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
    if name in alive or (name in outside and name in (OPERATOR, "record", "journal")):
      raise Refused(f"{name} hears")
    alive[name] = g
    hears(name, g, None)
    dispatch()

  globals().update(send=says, ask=asks, act=makes, drive=live)
  for who, hearer in [(OPERATOR, idle(OPERATOR)), ("record", given()), *outside.items(), ("journal", journal())]:
    live(hearer, who)
  return Act("chain1") if acts else chain("root")
