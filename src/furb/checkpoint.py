"""The checkpoint of a life, which stands outside the engine: gathered gives the data of a checkpoint at the tip of a
life, and seed gives the entries that a later life boots on from that data, so the build that opens a checkpoint, and
not the one that wrote it, says how its own engine makes each chain again.

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

  # The journal makes an act again only on a chain or an act that it knows when the entry comes, and each act with a
  # value comes before each open prompt. home gives each act that it makes again the chain that the act is on, a
  # chain being on itself and a chain that jumped on the chain that it went on as, and the facts of each other act, as
  # one with a value on an open prompt, stay in the prefix.
  jumped = data.get("jumped", [])
  went = {c["id"]: c["to"] for c in jumped}
  home = {c["id"]: c["id"] for c in data["chains"]}
  kinds = {c["id"]: "chain" for c in data["chains"]}
  for i in went:
    j = i
    while j in went:
      j = went[j]
    j in kinds and home.update({i: j})
  for x in sorted(data["acts"], key=lambda a: "value" not in a):
    if x["on"] in home:
      home[x["id"]], kinds[x["id"]] = home[x["on"]], x["kind"]
  asking = {home.get(a["id"]) for a in data["acts"] if "value" not in a}
  # The maker of each chain and each act that the data names stands in the fact carried of the chain that the act is
  # on, or of the first chain, since the seed makes each again as the operator or the journal.
  made = {c["id"]: {} for c in data["chains"]}
  for x in [*data["chains"], *data["acts"]]:
    "by" in x and made[home.get(x["id"], data["chains"][0]["id"])].update({x["id"]: x["by"]})
  # Each chain is made before any chain holds its prefix, so a chain with a source takes no turn of its source again.
  # A chain that jumped is made too, in the order of the numbers, and goes on as the chain that it jumped to.
  for c in sorted([*data["chains"], *jumped], key=lambda c: int(c["id"][len("chain") :])):
    put("chain", c["id"], operator, c["on"], c["label"], c["source"])
  entries += [(("jumped", c["id"], "journal", c["to"]),) for c in jumped]
  for c in data["chains"]:
    i = c["id"]
    definitions = [d] if isinstance(d := c["definitions"], str) else d
    kept = [
      t for t in c["turns"] if t[1] not in kinds or home[t[1]] != i or t[0] not in ("started", "done", kinds[t[1]])
    ]
    lost = [*c["lost"], *c["values"]]
    carried = ("carried", i, "journal", c["turns"], c["values"], definitions, made[i], lost, c["cut"])
    entries += [(carried,), (("prefix", i, "journal", kept),)]
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
    i = c["id"]
    word = (
      f"if not get(acting())[5]:\n for __part in {c.get('lives', [])!r}:\n"
      "  try: await rung(__part)\n  except Exception as __raised: debug(t'{__raised!r}')\n"
      " del __part"
    )
    c.get("lives") and put("rung", f"rung{numbers['rung'] + 1}", operator, i, word, "", "")
  return entries + [(("wake", y["id"], operator),) for y in data["chains"] if y["id"] in asking and not y["paused"]]


def birth(names):
  module, transcript, get = (names[x] for x in ("module", "transcript", "get"))
  notes = {}
  while True:
    match (yield {"carried", "module", "prefix", "jumped"}):
      case ("jumped", about, _, to):
        names["jumped"][about] = to
      case ("carried", about, _, _, values, definitions, _, lost, cut):
        bind(module(about), about, values, definitions)
        lost = [n for n in lost if n not in module(about)]
        note = f"The life goes on from a checkpoint. It lost {lost or 'no name'}, and it cut {cut or 'no act'}."
        notes[about] = (lost or cut) and note
      case ("prefix", about, *_) if notes.get(about):
        yield "tell", about, [notes.pop(about)]
      case ("module", about, _, held):
        # The program of a born chain starts with its birth, so each module that a chain makes again, as a fork of a
        # born chain makes its own, takes the bind of the last fact carried of that chain, or of the chain it forks.
        born = about
        while not transcript(born, "carried") and get(born)[5]:
          born = get(born)[5]
        for x in transcript(born, "carried")[-1:]:
          x[5:] and bind(held, about, x[4], x[5])


def bind(module, about, values, definitions):
  # The module of a chain at the tip, made again as data: each value from its text, each definition, then each value
  # that a definition bound again, while a value whose object the module still holds stays that object, which a
  # definition took. A value or a definition that raises loses only itself.
  bound = {}
  for word in [None, *definitions, None]:
    for k, text in values.items() if word is None else ():
      if k not in bound or module.get(k) is not bound[k]:
        try:
          module[k] = bound[k] = eval(text, module)  # noqa: S307
        except Exception:  # noqa: S110
          pass
    try:
      word and eval(compile(word, about, "exec"), module)  # noqa: S307
    except Exception:  # noqa: S110
      pass


def gathered(names, define, floor):
  get, peek, over, scope, question, prompted, lineage, paused, root = (
    names[x] for x in ("get", "peek", "over", "scope", "question", "prompted", "lineage", "paused", "ROOT")
  )
  transcript, module, program, standing, listening, refused = (
    names[x] for x in ("transcript", "module", "program", "standing", "listening", "Refused")
  )

  def literal(v, into=None):
    isinstance(v, str) and get(v) and into is not None and into.add(v)
    if isinstance(v, names["Act"]):
      return f"Act({str(v)!r})" * (into is not None)
    if isinstance(v, BaseException):
      return literal(v.args) and repr(v)
    if v is None or isinstance(v, (int, float, str, names["Text"], names["Exit"])):
      return repr(v) * (not (isinstance(v, float) and v - v))
    if not isinstance(v, (dict, list, tuple, set)):
      return ""
    pairs = v.items() if isinstance(v, dict) else [(y,) for y in v]
    s = [": ".join(made) if all(made := [literal(y, into) for y in pair]) else "" for pair in pairs]
    ends = "[]" if isinstance(v, list) else "()" if isinstance(v, tuple) else "{}"
    whole = (
      "set()"
      if isinstance(v, set) and not v
      else ends[0] + ", ".join(s) + "," * (ends == "()" and len(s) == 1) + ends[1]
    )
    return whole if all(s) else ""

  def number(called):
    return int(re.sub(r"^\D+", "", called) or 0)

  went = names.get("jumped", {})
  chains = [x for x in listening() if question(("chain", x)) and x not in went]
  made = {k: v for c in chains for x in transcript(c, "carried") for k, v in (x[6:] or [{}])[0].items()}

  def maker(called):
    return made.get(called, get(called)[2])

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
          values[c][key] = text
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
    d = {"kind": a[0], "id": one, "on": a[3], "by": maker(one)}
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
      for x in transcript(c)
      if x[0] not in ("module", "carried", "made")
      and literal(x, set())
      and (
        not question(x)
        or (x[0] == "reply" and x[1] in ended)
        or x[1] in borne
        or (x[0] == "run" and x[4][-5:] == "_told")
      )
    ]
    held = {*chains, *borne, *(x[1] for x in saved if question(x))}
    out.append(
      {
        "id": c,
        "by": maker(c),
        "on": a[3] * (a[3] in chains),
        "label": a[4],
        "source": a[5],
        "paused": paused(c),
        "turns": [x for x in saved if x[0] not in ("started", "done") or x[1] in held],
        "definitions": [
          *filter(None, [define(d) for x in transcript(c, "carried")[-1:] for d in (x[5:] or [()])[0]]),
          *filter(None, [define(w) for r, w in program(c).items() if get(r) and get(r)[2] != "extensions"]),
        ],
        "values": values[c],
        "lost": lost[c],
        "cut": [x for x in cut if scope(x) == c],
        "lives": [
          *dict.fromkeys(
            w
            for x in listening()
            if scope(x) == c
            for r in lineage(x)
            if question(("rung", r))
            for w in words
            # The ear of the extensions runs the word of an extension and its life word in one rung on a chain that
            # lacks the extension.
            if w and (get(r)[4] == w or get(r)[4].endswith("\n\n" + w))
          )
        ],
      }
    )
  jumped = [
    {"id": c, "by": maker(c), "on": get(c)[3], "label": get(c)[4], "source": get(c)[5], "to": went[c]}
    for c in sorted(went, key=number)
    if get(c)
  ]
  return {"taken": taken, "chains": out, "acts": acts, "jumped": jumped}
