import json

from furb.engine import HIDDEN, ROOT, Refused, Text, program, read, say, transcript, write


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


def extend(name: str, *rungs: str, tui: str = "", session: bool = False) -> Text:
  words, folder = program(), f".furb/extensions/{name}"
  if session:
    held = next((str(x[4]) for x in reversed(transcript(ROOT)) if x[0] == "enable" and x[3] == name), "")
    word = "\n\n".join([held.strip("\n"), *(words[one] for one in rungs)]).strip("\n") + "\n"
    say("enable", ROOT, name, word, "", "", "")
    return Text(f"session://{name}/{name}.py", word)
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
