import json

from furb.engine import HIDDEN, ROOT, Act, Refused, Text, act, program, question, read, scope, told, transcript, write

LORE = """Extend furb in the smallest of three ways that serves.
A def in a word lives with its chain: a chain with a source and a later life run it again.
Take it for the work of this chain.
extend(name, *rungs) lives with the project: it writes the words of those rungs as the word of the extension
name in .furb/extensions/name, and turns it on in .furb/config.json, so it plays on every chain, now and in
every later life. Take it for a helper that later work needs too, and give it only rungs that define.
An official extension lives in furb: a word, a contract, a suite and an ear of the World in the crate.
Take it for what every project needs, and ask the operator first.
enabled() gives the extensions that the life runs."""


def enabled() -> list[str]:
  return list(dict.fromkeys(str(x[3]) for x in transcript(ROOT) if x[0] == "enable"))


def extend(name: str, *rungs: str) -> Text:
  words, folder = program(), f".furb/extensions/{name}"
  try:
    held = read(f"{folder}/{name}.py", HIDDEN).content
  except Refused:
    held = ""
  write(Text(f"{folder}/furb.json", json.dumps({"name": name, "word": f"{name}.py"}) + "\n"))
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
