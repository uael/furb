from dataclasses import dataclass

from furb.engine import HEAD, Refused, Show, Text, ask, asked, commented, outcomes, read, scope, site, tell


def skills(on: str = "") -> list[Skill]:
  here = on or scope(site.get())
  was = next((outcomes[q[1]] for q in reversed(list(asked.values())) if q[0] == "skills" and q[3] == here), [])
  now = ask("skills", here)[1]
  found = (
    [Skill(x["name"], x["description"], x["path"]) for x in now if isinstance(x, dict)] if isinstance(now, list) else []
  )
  before = {x["name"]: x["description"] for x in was if isinstance(x, dict)} if isinstance(was, list) else {}
  lines = [f"{x.name}: {x.description}" for x in found if before.get(x.name) != x.description]
  lines += [f"{name} is gone" for name in before if name not in {x.name for x in found}]
  if lines:
    tell("skills", "", commented("\n".join(lines)))
  return found


def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  for x in skills(on):
    if x.name == name:
      return read(x.path, show, on)
  raise Refused(f"no skill {name}")


@dataclass
class Skill:
  name: str
  description: str
  path: str
