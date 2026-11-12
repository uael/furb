"""The checkpoint of a life, which stands outside the engine: gathered gives the data of a checkpoint at the tip of a
life, and seed gives the entries that a later life boots on from that data, so the engine that opens a checkpoint, and
not the one that wrote it, says how its own journal makes each chain again.

This interpreter imports it, and the sandbox of the crate loads it in a module of its own, as it loads the Kernel. Each
call is given the names of its engine, which it reads at each call and never holds, since boot binds the bus into the
engine. No model reads it, so it costs no token of the system prompt, and its contract, checkpoint.pyi, holds its types,
as the contract of the engine holds those of the engine.
"""

import re
from collections import Counter


def seed(names, data):
  root, operator = names["ROOT"], names["OPERATOR"]
  entries, numbers = [], Counter()

  def put(sort, called, *words):
    last = int(called[len(sort) :])
    entries.extend(((sort, f"{sort}{n}", root, ""),) for n in range(numbers[sort] + 1, last))
    numbers[sort] = max(numbers[sort], last - 1) + bool(words)
    words and entries.append(((sort, called, *words),))

  asking = {a["on"] for a in data["acts"] if "value" not in a}
  parts = {}
  for c in data["chains"]:
    put("chain", i := c["id"], operator, c["on"], c["label"], "")
    v = "".join(
      f"try:\n if {k!r} not in __bound or {k} is not __bound[{k!r}]: {k} = __bound[{k!r}] = "
      f"eval(__t) if isinstance(__t := __carried[4][{k!r}], str) else (__q.clear(), [__q.append(eval(p)) for p in __t], __q[-1])[2]"
      "\nexcept Exception: pass\n"
      for k in c["values"]
    )
    parts[i] = [*filter(None, [v, *([d] if isinstance(d := c["definitions"], str) else d), v, *c.get("lives", ())])]
    entries += [(("carried", i, "journal", c["turns"], c["values"], parts[i]),), (("prefix", i, i, c["turns"]),)]
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
    word = (
      "for __carried in [] if transcript(of='cut') else transcript(of='carried')[-1:]:\n"
      " __bound, __part, __q, __t = {}, None, [], None\n"
      " for __part in __carried[5]:\n  try: await rung(__part)\n  except Exception as __raised: debug(t'{__raised!r}')\n"
      " say('cut', scope(acting()), "
      "[x for x in reading() if x[0] != 'tell' or not under(x[1], acting()) or x[3][0].startswith(f'#{acting()} debugged')])\n"
      f' (__lost := [n for n in {c["lost"]} + [*__carried[4]] if n not in globals()]) + {cut} and debug(t"The life goes on from a checkpoint. '
      f"It lost {{__lost or 'no name'}}, and it cut {cut or 'no act'}.\")\n del __carried, __bound, __part, __lost, __q, __t"
    )
    seat = f"rung{numbers['rung'] + 1}"
    for maker, said in [(operator, word), *((seat, one) for one in parts[i])]:
      put("rung", f"rung{numbers['rung'] + 1}", maker, i, said, "", "")
      put("gate", gated := f"gate{numbers['gate'] + 1}", i, i, said)
      entries.append((("done", gated, "gate", []),))
  return entries + [(("wake", y["id"], operator),) for y in data["chains"] if y["id"] in asking and not y["paused"]]


def pieces(value, text, literal):
  # One eval of the sandbox takes at most 65536 constants and 65536 items of a display, so a value with more leaves
  # stands as the texts of its parts, each after the parts that it names as __q[i], and the last makes the value.
  most = 4096

  def leaves(v):
    held = [*v, *v.values()] if isinstance(v, dict) else v if isinstance(v, list | tuple | set) else ()
    return 1 + sum(map(leaves, held))

  if leaves(value) <= most:
    return text
  out, seen = [], set()

  def shape(v, j):
    if isinstance(v, dict | set):
      return f"{{{j}}}" if j or isinstance(v, dict) else "set()"
    return f"[{j}]" if isinstance(v, list) else f"({j},)" if j else "()"

  def put(v):
    if not isinstance(v, list | tuple | set | dict):
      return literal(v, seen), 1
    kids = (
      [(literal(k, seen) + ": ", *put(y)) for k, y in v.items()] if isinstance(v, dict) else [("", *put(y)) for y in v]
    )
    groups, held, total = [[]], 0, 1
    for head, part, n in kids:
      total += n
      if groups[-1] and held + n > most:
        groups.append([])
        held = 0
      groups[-1].append(head + part)
      held += n
    if total <= most:
      return shape(v, ", ".join(groups[0])), total
    refs = []
    for group in groups:
      out.append(shape(v, ", ".join(group)))
      refs.append(f"__q[{len(out) - 1}]")
    mark = "**" if isinstance(v, dict) else "*" if isinstance(v, set) else ""
    out.append("{" + ", ".join(mark + r for r in refs) + "}" if mark else " + ".join(refs))
    return f"__q[{len(out) - 1}]", 1

  return [*out, put(value)[0]]


def gathered(names, define, floor):
  get, peek, over, scope, question, prompted, lineage, paused, root = (
    names[x] for x in ("get", "peek", "over", "scope", "question", "prompted", "lineage", "paused", "ROOT")
  )
  transcript, reading, module, program, literal, standing, listening, refused = (
    names[x] for x in ("transcript", "reading", "module", "program", "literal", "standing", "listening", "Refused")
  )

  def number(called):
    return int(re.sub(r"^\D+", "", called) or 0)

  chains = [x for x in listening() if question(("chain", x))]
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
          values[c][key] = pieces(value, text, literal)
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
    d = {"kind": a[0], "id": one, "on": a[3]}
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
      and (not question(x) or (x[0] == "reply" and x[1] in ended) or x[1] in borne)
    ]
    held = {*chains, *borne, *(x[1] for x in saved if question(x))}
    out.append(
      {
        "id": c,
        "on": a[3] * (a[3] in chains),
        "label": a[4],
        "source": a[5],
        "paused": paused(c),
        "turns": [x for x in saved if x[0] not in ("started", "done") or x[1] in held],
        "definitions": [
          *filter(None, [define(w) for r, w in program(c).items() if get(r) and get(r)[2] != "extensions"])
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
