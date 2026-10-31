import json
import re

from furb.engine import (
  HIDDEN,
  ROOT,
  Refused,
  Text,
  bash,
  cancel,
  chain,
  clock,
  cwd,
  debug,
  get,
  grant,
  module,
  over,
  paused,
  peek,
  prompt,
  read,
  rung,
  transcript,
  wait,
  write,
)

CONVERGING = """converge(target) runs one loop toward the words of the owner, and nothing else: no spec, no design, no plan.
A target is a dict: its name; words, the numbers of the issues of the owner in the private repository of the decisions, whose bodies and comments are his ruling as they stand; paths, the files whose change it judges; capture(target, pairs, out), the command that captures it at each (part, sha) of pairs in one run, on the same records, as artifacts that the renderer of the target drew, in out/part; differ(target, out, was, whole, labels), the command and the stdin that turn the artifacts of out/head and of was into out/shots.json, the captures that changed with the text and the images that a judge reads; legend and look, the words that tell a judge how to read a capture; busy, a pattern of the faults that mean a busy tree and no fault of the tool; checks, the commands that must pass at the last sha; owners, the keys of the items of the round whose workers own its parts; and thread, the thread of its items.
A capture changed when the differ of the target says that it changed: a capture of the same code in another run did not change. A judge reads a capture that changed as the text that the differ gives it, and gets an image of a capture only when the differ gives one.
Each pass captures the target at the head of main, and each judge in JUDGES, blind on a chain of its own, reads against his words each capture that changed since the last pass, or each capture when his words changed or no pass judged them, and closes with the gaps that it sees. A capture that did not change keeps the gaps of the last pass. A gap counts only when it names a capture that the judge read and quotes his words exactly, and a failed check of the head is a gap of its own. A step of the capture that fails is a fault of the tool and no gap: converge rebuilds the tree once and captures again, and when the step fails again it tells the operator and judges nothing, so a PR passes and a pass of main waits for a change of main under its paths or of his words.
A pass with gaps is one step: the gaps go to each worker of an owner that works, or to one item in the queue of the round, which fixes the largest gap in one PR. The next pass comes once that work ended and its PR landed.
The target converges after CLEAN passes in a row with no gap, and stalls after STALL passes with no fewer gaps than its best before them: each tells the operator through the round, and the loop then waits for a change of his words or of main under paths.
The owner rules, on 10-29: "TUI PR merge as soon as ready, defects from the converge loop is then run only once and the defects are addressed as follow up." So converge gives the conductor no veto: a PR merges on its checks alone. converge_veto still judges a PR when a word asks it, as main and main with the PR merged in it, in one run, with trains that split in halves. converge(target, 1) runs one pass of main after the merges, and its gaps go to follow-up items. converge puts no label on a PR.
A grant holds each chain of a judge under EACH dollars. A judge whose chain waits on a usage limit is neither cut nor counted, and its BOUND starts again when the limit ends. A judge that fails, or that the round cuts at BOUND or by a pause, gives no answer: its pass is inconclusive. A pass whose judges the round cut all counts as no try, and a head whose judges failed to answer twice is stuck, which tells the operator and keeps the veto until a new head.
A target keeps CAPTURES trees, <tree>-1 to <tree>-N, so its captures run at once, each in a tree where no other capture runs, at most as many as the free slots of the cap of chains of the round, and one at least. A pass of main runs in the first tree alone, so its frames and those of the pass before it read the same paths.
A gap is held once. The ledger of a target keeps each open gap, keyed by its capture and the words of the owner that it quotes, for the PR or the pass of main that showed it first. A head that shows a gap that the ledger holds for another live PR, or that a pass of main shows, has no such gap: converge tells the one worker or item that holds it, once for each PR, orders no second one, and judges the head on its own gaps alone. A gap leaves the ledger when its holder shows it no more, and the gap of a PR that ended leaves it once a pass of main no longer shows it.
The ledger of a target, .furb/converge/<name>.json, keeps each pass, the open gaps, the frames and the records of the last pass, each judged head and the step that runs, so a later life goes on from it. A run ends when it spent JOBS captures or USD dollars, and a later run starts with the whole budget."""
JUDGES = ("claude-cli:sonnet/high",)
CLEAN = 2
STALL = 3
JOBS = 40
USD = 150.0
EACH = 3.0
BOUND = 1800.0
LIMIT = 12
BATCH = 16
PROMPT = 200000
CUT = 100000
EVERY = 300.0
BOOKS = ".furb/converge"
HOME = ROOT
CAPTURES = 3

LOOK = "each part of it"

LEGEND = """Each capture stands in a capture tag, under its name, and its change says why it is here. Its text is what the target made of the change. A capture whose change is the same as the change of another capture names that capture. An image shows a capture as the target draws it, when the capture comes with one."""

JUDGE = """You judge {name} against the words of its owner, quoted below as he wrote them, and against nothing else: no spec, no plan and no taste of your own. Every single detail counts.
Read his words whole, then look at each capture whole: {look}, and each thing that his words ask for and that the capture does not show.
{legend} before is the last pass that judged the capture, and now is the head of main.
Close with the list of the gaps that you see, each a tuple (capture, gap, quote): the name of the capture that shows it, as its tag gives it; what the capture shows against his words, in one or two sentences that a worker can act on; and the words of the owner that it fails, copied exactly from below, twelve characters at least. A gap that names no capture below, or whose quote is not in his words, does not count. Close with [] only when each capture serves his words.

<words>
{words}
</words>

The captures of {name} at {sha} that changed since the last pass, or each capture when no pass judged them against these words:

{captures}"""

SCREEN = """You judge the PR #{n} of {name} against the words of its owner, quoted below as he wrote them, and against nothing else: no spec, no plan and no taste of your own. Every single detail counts.
Each capture below is one that the PR changes: main, and main with the PR merged in it (the head {sha} of the PR), were captured in the same run, and a capture that the PR does not change is not below. Judge only what the PR changes: a gap counts when the head shows it against his words, and a gap that main shows the same way is not a gap of this PR. Look at each change whole: {look}, and each thing that his words ask for and that the head does not show.
{legend}
Close with the list of the gaps that you see, each a tuple (capture, gap, quote): the name of the capture that shows it, as its tag gives it; what the head shows against his words, in one or two sentences that a worker can act on; and the words of the owner that it fails, copied exactly from below, twelve characters at least. A gap that names no capture below, or whose quote is not in his words, does not count. Close with [] only when each change serves his words.

<words>
{words}
</words>

The captures that the PR changes:

{captures}"""

STEP = """The words of the owner on {name}, quoted below as he wrote them, are the only ruling of this work.

<words>
{words}
</words>

Blind judges read the captures of {name} at {sha} against his words, and found the gaps below. Fix the largest gap that falls in your work, the one that fails his words most, in one PR, with each other gap that the same change fixes. {capturing}
converge judges the head of your PR against his words before the conductor merges it, and tells you its gaps. Once your PR lands, converge captures {name} again, so leave each other gap to a later pass.

The gaps:

{gaps}"""

def conductor() -> dict:
  return module()


def tree_of(target: dict) -> str:
  """Each tree of a target holds the slot of its capture, so the captures of one target run in CAPTURES trees at once."""
  base = str(target.get("tree") or f"{cwd()}-converge-{target['name']}")
  return f"{base}-{int(target.get('slot', 1))}"


def book_of(target: dict) -> dict:
  try:
    got = json.loads(read(f"{BOOKS}/{target['name']}.json", HIDDEN).content)
  except (Refused, ValueError):
    got = {}
  book = got if isinstance(got, dict) else {}
  for key, empty in (
    ("passes", []),
    ("heads", {}),
    ("shots", {}),
    ("clean", 0),
    ("spent", 0.0),
    ("jobs", 0),
    ("state", ""),
    ("since", ""),
    ("words", ""),
    ("faulted", ""),
    ("ledger", {}),
    ("step", None),
    ("frames", ""),
    ("records", {}),
  ):
    book.setdefault(key, empty)
  return book


def booked(target: dict, book: dict) -> None:
  conductor()["quietly"](write, Text(f"{BOOKS}/{target['name']}.json", json.dumps(book, indent=1) + "\n"))


def tell_home(message: str) -> None:
  conductor()["quietly"](prompt, None, "converge: " + message, on=HOME)


async def ruling(target: dict) -> str:
  shq = conductor()["shq"]
  repo = conductor()["ops"]()[0]
  jq = r'"## #\(.number) \(.title)\n\n\(.body)" + ([.comments[] | "\n\n-- \(.author.login), \(.createdAt[0:10]):\n\n\(.body)"] | join(""))'
  views = [f"gh issue view {n} --repo {shq(repo)} --json number,title,body,comments --jq {shq(jq)}" for n in target["words"]]
  got = await converge_sh(" && echo && ".join(views), show=HIDDEN, timeout=300)
  if got.code != 0 or not got.stdout.content.strip():
    raise Refused(f"the words of the owner on {target['name']} did not load: {got.stderr.content[-300:]}")
  return got.stdout.content.strip()


async def main_head() -> str:
  got = await converge_sh("git fetch -q origin main && git rev-parse origin/main", show=HIDDEN, timeout=300)
  return got.stdout.content.strip() if got.code == 0 else ""


def named(target: dict, gaps: list[list[str]]) -> list[str]:
  found = {m for g in gaps for m in re.findall(r"[\w.-]+(?:/[\w.-]+)+", f"{g[0]} {g[1]}") if m.startswith(tuple(target["paths"]))}
  return sorted(found)


async def moved(target: dict, since: str, head: str, only: list[str] | None = None) -> bool:
  if not since or not head:
    return bool(head)
  if since == head or only == []:
    return False
  shq = conductor()["shq"]
  paths = " ".join(shq(p) for p in (target["paths"] if only is None else only))
  got = await converge_sh(f"git diff --quiet {shq(since)} {shq(head)} -- {paths}", show=HIDDEN, timeout=120)
  return got.code != 0


def does(target: dict, key: str):
  # A target names its function, and the module of the chain is asked at each call, so a reload takes effect also
  # in a loop that holds the dict of the target from before it.
  return globals()[target[key]]


def capturing(target: dict, pairs: list[tuple[str, str]], out: str, pins: dict, fresh: bool = False) -> str:
  shq = conductor()["shq"]
  checks = [
    f"( cd {shq(tree_of(target))} && {check} ) >{shq(f'{out}/check{i}.log')} 2>&1"
    f" || printf 'head\\t%s\\t%s\\n' {shq('check: ' + check)} {shq(f'{out}/check{i}.log')} >>{shq(out + '/fails.txt')}"
    for i, check in enumerate(target["checks"])
  ]
  pinned = f"printf '%s' {shq(json.dumps(pins))} >{shq(out + '/pins.json')}"
  return "\n".join([f"rm -rf {shq(out)} && mkdir -p {shq(out)}", *(["fresh=1"] if fresh else []), pinned, does(target, "capture")(target, pairs, out), *checks])


async def shots(target: dict, out: str, was: str, whole: bool, labels: tuple[str, str]) -> dict:
  command, stdin = does(target, "differ")(target, out, was, whole, labels)
  got = await converge_sh(command, stdin=stdin, show=HIDDEN, timeout=900)
  try:
    seen = json.loads(read(f"{out}/shots.json", HIDDEN).content) if got.code == 0 else None
  except (Refused, ValueError):
    seen = None
  if not isinstance(seen, dict):
    seen = {"faults": [["capture", f"the list of the captures failed: {got.stderr.content[-600:]}", ""]]}
  for key, empty in (("shots", {}), ("failed", []), ("faults", []), ("changed", []), ("kept", []), ("records", {})):
    seen.setdefault(key, empty)
  return seen


def trimmed(text: str) -> str:
  if len(text) <= CUT:
    return text
  head = text[:CUT]
  head = head[: head.rfind("\n")] if "\n" in head else head
  return head + f"\n     ... cut: the capture holds {len(text)} characters, and only its first part comes here"


def batches(captures: list[str], many: int, space: int) -> list[list[str]]:
  made: list[list[str]] = []
  used = 0
  for one in captures:
    if not made or len(made[-1]) >= max(1, many) or used + len(one) > space:
      made.append([])
      used = 0
    made[-1].append(one)
    used += len(one) + 2
  return made


def sections(changed: list[dict]) -> list[str]:
  told = []
  for one in changed:
    name = str(one["name"])
    images = [f"![{label} {name}](furb-image://{png})" for label, png in one["images"]]
    text = [trimmed(str(one["text"]))] if one["text"] else []
    told.append("\n".join([f'<capture name="{name}" change="{one["kind"]}">', *text, *images, "</capture>"]))
  return told


def judges(target: dict, brief: str, captures: list[str], label: str, batch: int) -> list[str]:
  quietly = conductor()["quietly"]
  asked = []
  groups = batches(captures, batch, PROMPT - len(brief))
  for actor in JUDGES:
    for group in groups:
      where = str(quietly(chain, f"converge/{target['name']}/{label}"))
      quietly(rung, f"actor = {actor!r}", on=where)
      quietly(grant, EACH, on=where)
      text = safe(brief.replace("{captures}", "\n\n".join(group)))
      asked.append(str(quietly(prompt, list[tuple[str, str, str]], text, to=actor, on=where)))
  return asked


def safe(text: str) -> str:
  return re.sub(r"\n\n(?=#\S)", "\n\n ", text)


def plain(text: str) -> str:
  for mark in ">*_`\"'“”":
    text = text.replace(mark, " ")
  return " ".join(text.split()).lower()


def gaps_of(asked: list[str], words: str, names: set[str]) -> tuple[list[list[str]], list[str]]:
  said, found, seen, broken = plain(words), [], set(), []
  for one in asked:
    got = peek(one)
    if not isinstance(got, list):
      broken.append(f"{one}: {got!r:.300}")
      continue
    for x in got:
      if not (isinstance(x, (tuple, list)) and len(x) == 3 and all(isinstance(y, str) for y in x)):
        continue
      shot, gap, quote = (y.strip() for y in x)
      key = (shot, plain(gap))
      if shot in names and gap and len(plain(quote)) >= 12 and plain(quote) in said and key not in seen:
        seen.add(key)
        found.append([shot, gap, quote])
  return found, broken


def cost_of(asked: list[str]) -> float:
  total = 0.0
  for where in {str(get(one)[3]) for one in asked if get(one)}:
    for x in transcript(where, "reply"):
      match peek(x[1]):
        case (_, _, (*_, (int() | float()) as cost), _):
          total += cost
  return total


def limited(where: str) -> bool:
  waiting = False
  for x in transcript(where):
    if x[0] == "tell" and len(x) > 3 and isinstance(x[3], list) and any("waits on" in str(n) and "usage limit" in str(n) for n in x[3]):
      waiting = True
    elif x[0] == "done" and (get(x[1]) or ("",))[0] == "reply":
      waiting = False
  return waiting


def stalled(counts: list[int]) -> bool:
  run = counts[len(counts) - counts[::-1].index(0) :] if 0 in counts else counts
  return len(run) > STALL and min(run[-STALL:]) >= min(run[:-STALL])


def itemized(gaps: list[list[str]], taken: dict[str, str]) -> str:
  lines = []
  for shot, gap, quote in gaps[:LIMIT]:
    lines.append(f"- {shot}: {gap}" + (f"\n  His words: {quote}" if quote else ""))
    if shot in taken:
      lines.append(f"  ![{shot}](furb-image://{taken[shot]})")
  if len(gaps) > LIMIT:
    lines.append(f"- and {len(gaps) - LIMIT} more gaps, which a later pass shows again")
  return "\n".join(lines)


def stepped(target: dict, book: dict, gaps: list[list[str]], taken: dict[str, str], sha: str, words: str) -> dict:
  c = conductor()
  text = STEP
  for key, value in (
    ("{name}", str(target["name"])),
    ("{words}", words),
    ("{sha}", sha),
    ("{capturing}", str(target.get("capturing", ""))),
    ("{gaps}", itemized(gaps, taken)),
  ):
    text = text.replace(key, value)
  text = safe(text)
  owners = [w for w, key in c["working"].items() if key in target["owners"] and not over(w)]
  if owners:
    for w in owners:
      c["quietly"](prompt, None, text, on=c["working_at"](w))
    return {"routed": owners, "sha": sha}
  key = f"converge-{target['name']}-{len(book['passes'])}"
  c["queue"].append((key, text, str(target["thread"])))
  return {"key": key, "sha": sha, "worker": "", "pr": 0}


def keyed(shot: str, quote: str) -> str:
  return f"{shot} | {plain(quote)}"


def live(c: dict, holder: str) -> bool:
  return holder == "main" or (holder.isdigit() and int(holder) in c["prs"])


def claim(book: dict, c: dict, holder: str, gaps: list[list[str]], main: bool = False) -> tuple[list[list[str]], dict]:
  """The ledger of the target holds each open gap once, for the PR or the pass of main that showed it first.

  A gap that another live holder holds is no gap of this holder: it comes back in the second value, by holder.
  A holder that shows a gap no more lets it go, and a pass of main lets go of the gaps of a PR that ended.
  """
  ledger, mine, others, seen = book["ledger"], [], {}, set()
  for one in gaps:
    shot, text, quote = one
    if not quote:
      mine.append(one)
      continue
    key = keyed(shot, quote)
    if key in seen:
      continue
    seen.add(key)
    was = ledger.get(key)
    if was and was["holder"] != holder and live(c, was["holder"]):
      others.setdefault(was["holder"], []).append((one, key))
      continue
    also = was["also"] if was and was["holder"] == holder else []
    ledger[key] = {"shot": shot, "gap": text, "quote": quote, "holder": holder, "also": also}
    mine.append(one)
  for key, was in [*ledger.items()]:
    if key not in seen and (was["holder"] == holder or (main and not live(c, was["holder"]))):
      del ledger[key]
  return mine, others


def deliver(c: dict, book: dict, holder: str, text: str) -> None:
  """Tell the one worker or item that holds a gap."""
  if holder == "main":
    step = book["step"] or {}
    owners = [w for w in step.get("routed", []) if not over(w)]
    for w in owners:
      c["quietly"](prompt, None, text, on=c["working_at"](w))
    if owners:
      return
    key = step.get("key", "")
  else:
    held = c["prs"].get(int(holder))
    own = c["owner_of"](held) if held else ""
    if own:
      c["quietly"](prompt, None, text, on=c["working_at"](own))
      return
    start = f"converge-pr{holder}-"
    key = next((k for k, *_ in c["queue"] if k.startswith(start)), "") or next(
      (k for k in c["working"].values() if k.startswith(start)), ""
    )
  worker = c["taken"](key) if key else ""
  if worker == "queued":
    c["queue"][:] = [(k, t + "\n\n" + text if k == key else t, th) for k, t, th in c["queue"]]
  elif worker:
    c["quietly"](prompt, None, text, on=c["working_at"](worker))


def told_holders(c: dict, book: dict, others: dict, who: str, sha: str, taken: dict[str, str]) -> None:
  """A gap that a holder has already goes to that holder, once for each PR or pass that shows it again."""
  for holder, items in others.items():
    fresh = [(one, key) for one, key in items if who not in book["ledger"][key]["also"]]
    if not fresh:
      continue
    for _, key in fresh:
      book["ledger"][key]["also"].append(who)
    shown = "the pass of main" if who == "main" else f"the PR #{who}"
    text = safe(
      f"converge found {len(fresh)} gap(s) that your work holds again in {shown} at {sha[:7]}. It orders no second"
      f" worker or item for them, and it judges {shown} on its own gaps alone. Fix them in your work, and each one"
      f" is served:\n\n{itemized([one for one, _ in fresh], taken)}"
    )
    deliver(c, book, holder, text)


async def settled(target: dict, step: dict, head: str) -> bool:
  c = conductor()
  if step.get("held"):
    gone = all(not (h.isdigit() and int(h) in c["prs"]) for h in step["held"])
    return gone or await moved(target, step["sha"], head)
  if step.get("routed"):
    return all(over(w) for w in step["routed"]) or await moved(target, step["sha"], head)
  own = c["taken"](step["key"])
  if own == "queued":
    return False
  step["worker"] = step["worker"] or own
  if not step["worker"]:
    return True
  if step["worker"] in c["working"]:
    return False
  got = peek(step["worker"])
  step["pr"] = got[0] if isinstance(got, tuple) and got and isinstance(got[0], int) else 0
  return not step["pr"] or step["pr"] in c["merged"] or step["pr"] not in c["prs"]


TARGETS: list[dict] = globals().setdefault("TARGETS", [])
LIVE: dict[str, dict] = {}
OPEN: dict[str, dict] = {}
MAIN: dict[str, dict] = {}


def holding(target: dict) -> set[int]:
  """The slot of each tree of a target where a capture holds an open command."""
  jobs = [*OPEN.values(), *MAIN.values()]
  return {
    int(j.get("slot", 1))
    for j in jobs
    if j.get("target") == target["name"] and j.get("act") and not j["asked"] and not over(j["act"])
  }


def vacant(target: dict, slot: int = 0) -> int:
  """A tree of a target with no open capture, or 0: the slot alone when it names one, and the captures that run stay
  under CAPTURES and under the free slots of the cap of chains of the round, since each capture asks its judges on
  chains of their own, but one capture runs always."""
  held, c = holding(target), conductor()
  most = CAPTURES
  if callable(c.get("cap")) and callable(c.get("open_chains")):
    most = min(CAPTURES, max(1, int(c["cap"]()) - len(c["open_chains"]())))
  if len(held) >= most:
    return 0
  return next((one for one in ([slot] if slot else range(1, CAPTURES + 1)) if one not in held), 0)


def launched(target: dict, book: dict, job: dict, fresh: bool = False) -> str:
  """The command of a capture ends the old command of the job first, and a cancel of it kills its process group."""
  old = job.get("act", "none")
  if old != "none" and not over(old):
    cancel(old)
  where = {**target, "slot": int(job.get("slot", 1))}
  command = capturing(where, job["pairs"], job["out"], book["records"], fresh)
  return str(conductor()["quietly"](bash, command, show=HIDDEN, timeout=7200))


def shared(target: dict) -> dict:
  if target["name"] not in LIVE:
    LIVE[target["name"]] = book_of(target)
  return LIVE[target["name"]]


async def classified(target: dict, book: dict, pr: dict) -> str:
  key = f"{pr['n']} {pr['sha']}"
  if key not in book["heads"]:
    got = await converge_sh(f"gh pr view {pr['n']} --json files --jq '.files[].path'", show=HIDDEN, timeout=120)
    if got.code != 0:
      return "held"
    if not any(f.startswith(tuple(target["paths"])) for f in got.stdout.lines):
      book["heads"][key] = {"state": "outside"}
      return "outside"
    # A head that only merged main into its branch makes the same change, so it keeps the verdict of that change.
    same = await converge_sh(
      f"git fetch -q origin main refs/pull/{pr['n']}/head && git diff origin/main...{pr['sha']} | git patch-id --stable",
      show=HIDDEN,
      timeout=120,
    )
    patch = (same.stdout.content.split() or [""])[0] if same.code == 0 else ""
    was = next(
      (v for k, v in book["heads"].items() if patch and k.split()[0] == str(pr["n"]) and v.get("patch") == patch and v.get("state") in ("clean", "gaps")),
      None,
    )
    book["heads"][key] = {**was, "carried": True} if was else {"state": "held", **({"patch": patch} if patch else {})}
  return book["heads"][key]["state"]


async def started(target: dict, book: dict, sha: str, n: int = 0) -> dict:
  # A pass of main runs in the first tree alone, so its frames and those of the pass before it share their paths.
  want = 0 if n else 1
  if not vacant(target, want):
    return {}
  shq = conductor()["shq"]
  pairs = [("head", sha)]
  if n:
    label = f"pr{n}-{sha[:12]}"
    # The head is main with the PR merged in it, so a change of main shows in neither part of the pair.
    make = "t=$(git merge-tree --write-tree origin/main " + shq(sha) + ") && git rev-parse origin/main && git commit-tree \"$t\" -p origin/main -m converge"
    got = await converge_sh(f"git fetch -q origin main refs/pull/{n}/head && {make}", show=HIDDEN, timeout=300)
    made = got.stdout.lines
    if got.code != 0 or len(made) != 2:
      return {}
    pairs = [("before", made[0].strip()), ("head", made[1].strip())]
  else:
    label = f"main{len(book['passes'])}-{sha[:12]}"
  slot = vacant(target, want)
  if not slot:
    return {}
  out = f"{cwd()}/{BOOKS}/{target['name']}/{label}"
  where = {**target, "slot": slot}
  act = str(conductor()["quietly"](bash, capturing(where, pairs, out, book["records"]), show=HIDDEN, timeout=7200))
  return {"kind": "pr" if n else "main", "sha": sha, "n": n, "out": out, "label": label, "act": act, "pairs": pairs, "asked": [], "target": target["name"], "slot": slot}


def dropped(live: set[str]) -> None:
  for key in [k for k in OPEN if k not in live]:
    job = OPEN.pop(key)
    if any(k in OPEN for k in job.get("batch", [])):
      continue
    for one in [job["act"], *job["asked"]]:
      if one != "none" and not over(one):
        cancel(one)


TRAIN = 6


def solo(book: dict, job: dict) -> dict:
  """A train that met a fault or no answer sends each of its PRs to be judged alone."""
  for key in job["batch"]:
    book["heads"][key] = {**book["heads"].get(key, {}), "state": "held", "solo": True}
    OPEN.pop(key, None)
  return {}


def halve(book: dict, job: dict) -> dict:
  """A train that found a gap splits in two halves, each a train of its own, down to a PR alone."""
  keys = job["batch"]
  middle = (len(keys) + 1) // 2
  for part in (keys[:middle], keys[middle:]):
    for key in part:
      head = {k: v for k, v in book["heads"].get(key, {}).items() if k not in ("half", "solo")}
      book["heads"][key] = {**head, "state": "held", **({"half": part} if len(part) > 1 else {"solo": True})}
      OPEN.pop(key, None)
  return {}


def cleared(book: dict, job: dict) -> dict:
  for key in job["batch"]:
    book["heads"][key] = {**book["heads"].get(key, {}), "state": "clean", "gaps": [], "train": job["label"]}
    OPEN.pop(key, None)
  return {}


def released(job: dict) -> dict:
  for key in job["batch"]:
    OPEN.pop(key, None)
  return {}


async def trained(target: dict, book: dict, pr: dict) -> dict:
  """The green PRs that wait for the judges ride one capture: main, and main with each of them merged in it."""
  c = conductor()
  keys: list[str] = []
  for one in sorted(c["prs"].values(), key=lambda p: p["n"]):
    key = f"{one['n']} {one['sha']}"
    calm = one.get("verdict") == "pass" and one.get("base", "main") == "main" and not one.get("draft")
    if not calm or {"hold", "paused", "owner-merge"} & set(one.get("labels", [])) or key in OPEN:
      continue
    if await classified(target, book, one) == "held" and not book["heads"][key].get("solo"):
      keys.append(key)
  mine = f"{pr['n']} {pr['sha']}"
  half = book["heads"][mine].get("half")
  keys = [k for k in keys if k in half] if half else [k for k in keys if not book["heads"][k].get("half")][:TRAIN]
  if mine not in keys or len(keys) < 2:
    return {}
  shq = c["shq"]
  steps = "".join(
    f" && {{ t=$(git merge-tree --write-tree $cur {shq(k.split()[1])} 2>/dev/null)"
    f" && cur=$(git commit-tree $t -p $cur -p {shq(k.split()[1])} -m converge) && echo in {k.split()[0]}"
    f" || echo out {k.split()[0]}; }}"
    for k in keys
  )
  pulls = " ".join(f"refs/pull/{k.split()[0]}/head" for k in keys)
  got = await converge_sh(
    f"git fetch -q origin main {pulls} && cur=$(git rev-parse origin/main) && echo $cur{steps} && echo $cur",
    show=HIDDEN,
    timeout=300,
  )
  lines = [x.strip() for x in got.stdout.lines if x.strip()]
  ins = {x.split()[1] for x in lines if x.startswith("in ")}
  members = [k for k in keys if k.split()[0] in ins]
  slot = vacant(target)
  if got.code != 0 or len(lines) < 2 or mine not in members or len(members) < 2 or not slot:
    return {}
  base, sha = lines[0], lines[-1]
  nums = [k.split()[0] for k in members]
  label = f"train{'-'.join(nums)}-{sha[:12]}"
  out = f"{cwd()}/{BOOKS}/{target['name']}/{label}"
  pairs = [("before", base), ("head", sha)]
  act = str(c["quietly"](bash, capturing({**target, "slot": slot}, pairs, out, book["records"]), show=HIDDEN, timeout=7200))
  return {
    "kind": "pr", "sha": sha, "n": ", #".join(nums), "out": out, "label": label, "act": act, "pairs": pairs,
    "asked": [], "target": target["name"], "batch": members, "slot": slot,
  }


async def converge_veto(pr: dict) -> bool:
  key = f"{pr['n']} {pr['sha']}"
  dropped({key, *[f"{n} {one['sha']}" for n, one in conductor()["prs"].items()]})
  for target in TARGETS:
    book = shared(target)
    state = await classified(target, book, pr)
    if state == "held" and key in book["heads"]:
      job = OPEN.get(key)
      if not job:
        if not vacant(target):
          return True
        job = await trained(target, book, pr) or await started(target, book, str(pr["sha"]), int(pr["n"]))
        if not job:
          return True
        job["words"] = await ruling(target)
      job = await advanced(target, book, job, job["words"], str(pr["sha"]))
      for one in (job or {}).get("batch", [key]):
        if job:
          OPEN[one] = job
        else:
          OPEN.pop(one, None)
      booked(target, book)
      state = book["heads"][key]["state"]
    if state not in ("outside", "clean", "unjudged"):
      return True
  return False


def counted(book: dict) -> list[int]:
  return [len(p["gaps"]) for p in book["passes"] if "gaps" in p and not p.get("n")]


def faulted(target: dict, book: dict, job: dict, faults: list[list[str]], words: str, head: str) -> dict:
  said = " ".join(why for _, why, _ in faults)[:800]
  name, sha = target["name"], job["sha"][:7]
  if job["kind"] == "pr":
    debug(t"{name} PR #{job['n']} at {sha}: faulted, no judge read it")
    book["heads"][f"{job['n']} {job['sha']}"] = {"state": "unjudged"}
    tell_home(f"the capture of {name} for the PR #{job['n']} at {sha} failed again after converge rebuilt its tree. That is a fault of the tool and no gap, so no judge read the PR and the conductor does not hold it: {said}")
  else:
    book.update(state="faulted", since=head, faulted=words)
    tell_home(f"the capture of {name} at {sha} failed again after converge rebuilt its tree. That is a fault of the tool and no gap, so no judge read it, and the pass waits for a change of main under its paths or of his words: {said}")
  return {}


async def advanced(target: dict, book: dict, job: dict, words: str, head: str) -> dict:
  pr = job["kind"] == "pr"
  if not job["asked"]:
    if not over(job["act"]):
      return job
    if not job.get("resumed") and isinstance(peek(job["act"]), Refused):
      job.update(resumed=True, act=launched(target, book, job))
      return job
    was = f"{job['out']}/before" if pr else book["frames"] if book["words"] == words else ""
    seen = await shots(target, job["out"], was, not pr, ("main", "head") if pr else ("before", "now"))
    if seen["faults"] and target.get("busy") and re.search(target["busy"], " ".join(why for _, why, _ in seen["faults"]), re.I):
      # A busy tree or lock is no fault of the tool: the job ends, and its turn comes again.
      if job.get("batch"):
        return released(job)
      if pr:
        book["heads"][f"{job['n']} {job['sha']}"] = {"state": "held"}
      return {}
    if seen["faults"]:
      if not job.get("again"):
        job.update(again=True, act=launched(target, book, job, True))
        return job
      return solo(book, job) if job.get("batch") else faulted(target, book, job, seen["faults"], words, head)
    book["jobs"] += 1
    job.update(taken=seen["shots"], failed=seen["failed"], kept=seen["kept"], records=seen["records"])
    job["shown"] = [str(one["name"]) for one in seen["changed"]]
    brief = (SCREEN if pr else JUDGE).replace("{look}", str(target.get("look") or LOOK))
    brief = brief.replace("{legend}", str(target.get("legend") or LEGEND).replace("{was}", "main" if pr else "before"))
    brief = brief.replace("{name}", str(target["name"])).replace("{sha}", job["sha"]).replace("{n}", str(job["n"]))
    captures = sections(seen["changed"])
    job["asked"] = judges(target, brief.replace("{words}", words), captures, job["label"], BATCH)
    job["asked"] = job["asked"] or ["none"]
    job["at"] = clock()
  asked = [a for a in job["asked"] if a != "none"]
  held = [one for one in asked if not over(one) and limited(str(get(one)[3]))]
  if held:
    job["at"] = clock()
  late = clock() - float(job.get("at", 0.0)) >= BOUND
  cut = job.setdefault("cut", [])
  for one in asked:
    if not over(one) and one not in held and (late or paused(str(get(one)[3]))):
      cancel(one)
      cut.append(one)
  if not all(over(a) for a in asked):
    return job
  found, broken = gaps_of(asked, words, set(job["shown"]))
  book["spent"] += cost_of(asked)
  if broken and job.get("batch"):
    return solo(book, job)
  if broken:
    book["passes"].append({"sha": job["sha"], "n": job["n"], "inconclusive": broken})
    if pr:
      key = f"{job['n']} {job['sha']}"
      tries = book["heads"].get(key, {}).get("tries", 0) + (0 if all(one in cut for one in asked) else 1)
      book["heads"][key] = {"state": "held" if tries < 2 else "stuck", "tries": tries}
      if tries >= 2:
        tell_home(
          f"the judges of the PR #{job['n']} at {job['sha'][:7]} gave no answer twice, so the conductor does not"
          f" merge it until a new head: {'; '.join(broken)[:600]}"
        )
    return {}
  if job.get("batch"):
    return halve(book, job) if [*job["failed"], *found] else cleared(book, job)
  if pr:
    await screened(target, book, job, [*job["failed"], *found], words)
    return {}
  last = next((p for p in reversed(book["passes"]) if "gaps" in p and not p.get("n")), None)
  carried = [g for g in (last["gaps"] if last else []) if g[0] in job["kept"]]
  gaps = [*job["failed"], *carried, *found]
  book["passes"].append({"sha": job["sha"], "gaps": gaps, "judged": len(job["shown"])})
  if job["taken"]:
    book.update(shots=job["taken"], frames=f"{job['out']}/head")
  if job["records"]:
    book["records"] = job["records"]
  book["words"] = words
  counts = counted(book)
  label, best = target["name"], min(counts)
  debug(t"{label} pass {len(counts)} at {job['sha'][:7]}: {len(gaps)} gaps, best {best}, PR none")
  mine, others = claim(book, conductor(), "main", gaps, main=True)
  told_holders(conductor(), book, others, "main", job["sha"], job["taken"])
  if gaps:
    book["clean"] = 0
    book["step"] = (
      stepped(target, book, mine, job["taken"], job["sha"], words)
      if mine
      else {"held": sorted(others), "sha": job["sha"]}
    )
    if stalled(counts):
      book.update(state="stalled", since=head, named=named(target, gaps))
      if book.get("toldbest") != min(counts):
        book["toldbest"] = min(counts)
        tell_home(
          f"{target['name']} stalled: its last {STALL} passes found {counts[-STALL:]} gaps against the words of the"
          f" owner, no fewer than its best before them. His words or the measure must change, and only he changes his"
          f" words. Its ledger is {BOOKS}/{target['name']}.json, and its last gaps are:\n{itemized(gaps, {})[:3000]}"
        )
  else:
    book["clean"] += 1
    if book["clean"] >= CLEAN:
      book.update(state="converged", since=head)
      tell_home(
        f"{target['name']} converged: {CLEAN} passes in a row at {job['sha'][:7]} found no gap against the words of"
        f" the owner. Its ledger is {BOOKS}/{target['name']}.json."
      )
  return {}


async def screened(target: dict, book: dict, job: dict, gaps: list[list[str]], words: str) -> None:
  c, n, sha = conductor(), job["n"], job["sha"]
  gaps, others = claim(book, c, str(n), gaps)
  label = target["name"]
  debug(t"{label} PR #{n} at {sha[:7]}: {len(gaps)} gaps")
  told_holders(c, book, others, str(n), sha, job["taken"])
  book["heads"][f"{n} {sha}"] = {**book["heads"].get(f"{n} {sha}", {}), "state": "gaps" if gaps else "clean", "gaps": gaps}
  if not gaps:
    return
  body = (
    f"🤖 converge judged the head {sha[:7]} of this PR against the words of the owner on {target['name']}, and"
    " the conductor does not merge it until a head shows none of these gaps:\n\n"
    + "\n".join(f"- {shot}: {gap}" + (f" His words: {quote}" if quote else "") for shot, gap, quote in gaps[:LIMIT])
  )
  await converge_sh(f"gh pr comment {n} --body-file -", stdin=body, show=HIDDEN, timeout=120)
  told = (
    f"converge judged the head {sha[:7]} of your PR #{n} against the words of the owner, which follow, and the"
    f" conductor does not merge it until a head shows none of the gaps below. Fix them and push."
    f"\n\n<words>\n{words}\n</words>\n\nThe gaps:\n\n{itemized(gaps, job['taken'])}"
  )
  told = safe(told)
  pr = c["prs"].get(n)
  own = c["owner_of"](pr) if pr else ""
  if own:
    c["quietly"](prompt, None, told, on=c["working_at"](own))
  else:
    c["queue"].append((f"converge-pr{n}-{sha[:7]}", f"The open PR #{n}. " + told, str(target["thread"])))


async def converge(target: dict, ticks: int = 0) -> dict:
  book, job, tick, spoke = shared(target), {}, 0, ""
  # The budget counts the captures and the dollars of this run, so a run that the coordinator starts again has it whole.
  if book["state"] == "spent":
    book["state"] = ""
  jobs, spent = book["jobs"], book["spent"]
  while not ticks or tick < ticks:
    tick += 1
    before = len(counted(book))
    head = await main_head()
    words = await ruling(target)
    said = book["faulted"] if book["state"] == "faulted" else book["words"]
    if book["state"] in ("converged", "stalled", "faulted") and (
      words != said or await moved(target, book["since"], head, book.get("named") if book["state"] == "stalled" else None)
    ):
      book.update(state="", clean=0)
    if book["step"] and await settled(target, book["step"], head):
      book["step"] = None
    if not job and book["state"] != "spent" and (book["jobs"] - jobs >= JOBS or book["spent"] - spent >= USD):
      book["state"] = "spent"
      tell_home(f"{target['name']} spent its budget: {book['jobs'] - jobs} captures and {book['spent'] - spent:.2f} dollars.")
    if not job and book["state"] != "spent" and head:
      if not book["state"] and not book["step"]:
        job = await started(target, book, head)
        MAIN[target["name"]] = job
    while job:
      was = job
      job = await advanced(target, book, job, words, head)
      if job is was:
        break
    if job:
      MAIN[target["name"]] = job
    else:
      MAIN.pop(target["name"], None)
    booked(target, book)
    state = book["state"] or ("waits" if job else "idle")
    if len(counted(book)) == before and (job or state != spoke):
      counts = counted(book)
      last = f"{counts[-1]} gaps, best {min(counts)}" if counts else "no pass yet"
      debug(t"{target['name']} tick {tick} at {head[:7]}: {state}, {last}")
    spoke = state
    if book["state"] == "spent" and not job:
      return book
    if not job:
      await wait(EVERY)
    else:
      await wait(EVERY, *[a for a in ([job["act"]] if not job["asked"] else job["asked"]) if a != "none" and not over(a)])
  return book


def converge_sh(command, **named):
  """The commands of this word read their own codes, so each is run with every code accepted."""
  return bash(command, ok=True, **named)
