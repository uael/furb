import json

from furb.engine import HIDDEN, ROOT, Act, Refused, Text, act, program, question, read, scope, told, transcript, write

LORE = """Extend furb in the smallest of three ways that serves.
A def in a word lives with its chain: a chain with a source and a later life run it again.
Take it for the work of this chain.
extend(name, *rungs) lives with the project: it writes the words of those rungs as the word of the extension
name in .furb/extensions/name, and turns it on in .furb/config.json, so it plays on every chain, now and in
every later life. Take it for a helper that later work needs too, and give it only rungs that define.
extend(name, *rungs, tui=module) also writes module, TypeScript, as the part of the extension that the TUI loads
live. It imports no value and exports a default function of furb, which calls furb.panel(title, draw),
furb.row(kind, draw), furb.mark(draw), furb.key("ctrl+t", run) and furb.command(name, detail, run). draw(seen) gives
a list of lines, each a str or a list of (text, tone); a row gives {label, details}; a mark gives a list of {chain,
at, title, lines}, a rule in the feed of chain after the last act that began before at. seen.of(kind) gives the
acts of a kind.
An official extension lives in furb: a word, a contract, a suite and an ear of the World in the crate.
Take it for what every project needs, and ask the operator first.
enabled() gives the extensions that the life runs.
bound_by() gives each name that a word of the chain bound, with the rung of that word."""


def enabled() -> list[str]:
  return list(dict.fromkeys(str(x[3]) for x in transcript(ROOT) if x[0] == "enable"))


def bound_by(on: str = "") -> dict[str, str]:
  found = {}
  for by, text in program(on).items():
    for line in text.splitlines():
      left, eq, right = line.partition("=")
      name = left.split(":")[0].strip() if eq and not right.startswith("=") else ""
      if line.startswith("<s:"):
        name = line[3:].split(">")[0]
      elif line.removeprefix("async ").startswith(("def ", "class ")):
        name = line.removeprefix("async ").split(None, 2)[1].split("(")[0].split(":")[0]
      if line[:1].strip() and name.isidentifier():
        found[name] = by
  return found


def extend(name: str, *rungs: str, tui: str = "") -> Text:
  words, folder = program(), f".furb/extensions/{name}"
  try:
    held = read(f"{folder}/{name}.py", HIDDEN).content
  except Refused:
    held = ""
  if tui:
    write(Text(f"{folder}/tui.ts", tui.strip("\n") + "\n"))
  else:
    try:
      tui = read(f"{folder}/tui.ts", HIDDEN).content
    except Refused:
      tui = ""
  try:
    skill = read(f"{folder}/SKILL.md", HIDDEN).content
  except Refused:
    skill = f"---\nname: {name}\ndescription: The extension {name} of this project.\n---\nThe extension {name} of this project.\n"
  head, _, body = skill[4:].partition("\n---\n") if skill.startswith("---\n") else ("", "", skill)
  kept = [x for x in head.splitlines() if not x.strip().startswith(("furb-word:", "furb-view:"))]
  kept = kept if "metadata:" in kept else [*kept, "metadata:"]
  at = kept.index("metadata:") + 1
  kept[at:at] = [f"  furb-word: {name}.py", *(["  furb-view: tui.ts"] if tui else [])]
  write(Text(f"{folder}/SKILL.md", "---\n" + "\n".join(kept) + "\n---\n" + body))
  landed = write(
    Text(f"{folder}/{name}.py", "\n\n".join([held.strip("\n"), *(words[one] for one in rungs)]).strip("\n") + "\n")
  )
  try:
    config = json.loads(read(".furb/config.json", HIDDEN).content)
  except Refused:
    config = {}
  config.setdefault("extensions", {})[name] = f"extensions/{name}"
  write(Text(".furb/config.json", json.dumps(config, indent=2) + "\n"))
  return landed


def ouroboros(on: str = "") -> Act:
  def ear(id):
    if not any(x[0] == "tell" and question(("ouroboros", x[1])) for x in transcript(scope(id))):
      yield told(id, "", lore=LORE)
    yield "done", id, None

  return act("ouroboros", on, ear)
