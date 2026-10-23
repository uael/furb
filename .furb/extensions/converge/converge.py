import json
import re

CONVERGING = """converge(target) runs one loop toward the words of the owner, and nothing else: no spec, no design, no plan.
A target is a dict: its name; words, the numbers of the issues of the owner in the private repository of the decisions, whose bodies and comments are his ruling as they stand; paths, the files whose change it judges; capture(target, sha, out), the command that writes its captures at a sha as PNG files in out; checks, the commands that must pass at that sha; owners, the keys of the items of the round whose workers own its parts; and thread, the thread of its items.
Each pass captures the target at the head of main, and the judges in JUDGES, each blind on a chain of its own, read each capture against his words and close with the gaps that they see. A gap counts only when it names a capture of the pass and quotes his words exactly, and a failed capture or check is a gap of its own.
A pass with gaps is one step: the gaps go to each worker of an owner that works, or to one item in the queue of the round, which fixes the largest gap in one PR. The next pass comes once that work ended and its PR landed.
The target converges after CLEAN passes in a row with no gap, and stalls after STALL passes with no fewer gaps than its best before them: each tells the operator through the round, and the loop then waits for a change of his words or of main under paths.
Before the conductor merges a PR that changes paths, it asks converge through its vetoes: once the checks of the PR are green, converge judges each capture that the PR changes against main, and says no until a head shows no gap. The gaps go to its worker and to a comment, and a new head is judged again. converge puts no label on a PR.
The ledger of a target, .furb/converge/<name>.json, keeps each pass, each judged head and the step that runs, so a later life goes on from it. The loop ends when it spent JOBS captures or USD dollars."""
JUDGES = ("claude-cli:fable/high", "claude-cli:sonnet/high")
CLEAN = 2
STALL = 3
JOBS = 40
USD = 150.0
EACH = 15.0
LIMIT = 12
BATCH = 16
EVERY = 300.0
RECORDS = 3
HEAVIEST = 160_000_000
BOOKS = ".furb/converge"
IMAGES = ""
CAPTURER = ""
HOME = ROOT

JUDGE = """You judge {name} against the words of its owner, quoted below as he wrote them, and against nothing else: no spec, no plan and no taste of your own. Every single detail counts.
Read his words whole, then look at each capture whole: each line, each color, each cut, each overlap, each empty place, and each thing that his words ask for and that the capture does not show.
Close with the list of the gaps that you see, each a tuple (capture, gap, quote): the name of the capture that shows it, as it stands above its image; what the capture shows against his words, in one or two sentences that a worker can act on; and the words of the owner that it fails, copied exactly from below, twelve characters at least. A gap that names no capture below, or whose quote is not in his words, does not count. Close with [] only when each capture serves his words.

<words>
{words}
</words>

The captures of {name} at {sha}:

{captures}"""

SCREEN = """You judge the PR #{n} of {name} against the words of its owner, quoted below as he wrote them, and against nothing else: no spec, no plan and no taste of your own. Every single detail counts.
Each capture below that the PR changes comes as a pair: first as main shows it, then at the head of the PR. Judge only what the PR changes: a gap counts when the head shows it against his words, and a gap that main shows the same way is not a gap of this PR.
Close with the list of the gaps that you see, each a tuple (capture, gap, quote): the name of the capture that shows it, as it stands above its images; what the head shows against his words, in one or two sentences that a worker can act on; and the words of the owner that it fails, copied exactly from below, twelve characters at least. A gap that names no capture below, or whose quote is not in his words, does not count. Close with [] only when each change serves his words.

<words>
{words}
</words>

The captures that the PR changes, main then its head {sha}:

{captures}"""

STEP = """The words of the owner on {name}, quoted below as he wrote them, are the only ruling of this work.

<words>
{words}
</words>

Blind judges read the captures of {name} at {sha} against his words, and found the gaps below. Fix the largest gap that falls in your work, the one that fails his words most, in one PR, with each other gap that the same change fixes. {capturing}
converge judges the head of your PR against his words before the conductor merges it, and tells you its gaps. Once your PR lands, converge captures {name} again, so leave each other gap to a later pass.

The gaps:

{gaps}"""

TUI_CAPTURE = r"""repo={repo}; tree={tree}; out={out}; sha={sha}
fail() { printf '%s\t%s\n' "$1" "$2" >>"$out/fails.txt"; }
[ -d "$tree" ] || "$repo/script/worktree.sh" "$tree" converge/{name} >"$out/tree.log" 2>&1 || { fail tree "$out/tree.log"; exit 0; }
cd "$tree" || exit 0
{ git fetch -q origin main && { git cat-file -e "$sha^{commit}" 2>/dev/null || git fetch -q origin "$sha"; } && git checkout -q --detach "$sha" && git clean -fdq; } >"$out/checkout.log" 2>&1 || { fail checkout "$out/checkout.log"; exit 0; }
built=$(cat target/converge-built 2>/dev/null)
if [ -z "$built" ] || ! git diff --quiet "$built" "$sha" -- src bind build.rs Cargo.toml Cargo.lock package.json bun.lock; then
  build=; [ -f "$TMPDIR/build.sh" ] && build="sh $TMPDIR/build.sh"
  { bun install --frozen-lockfile && $build bun run build; } >"$out/build.log" 2>&1 || { fail build "$out/build.log"; exit 0; }
  mkdir -p target && echo "$sha" >target/converge-built
fi
FURB_GALLERY_OUT="$out/demo" bun run screenshots >"$out/demo.log" 2>&1 || fail demo "$out/demo.log"
mkdir -p "$out/records"
python3 -I - "$repo/.furb/sessions" "$out/records" {records} {heaviest} >"$out/records.log" 2>&1 <<'EOF' || fail records "$out/records.log"
import os, sys
src, dst, many, most = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
names = [f for f in os.listdir(src) if f.endswith(".jsonl") and not f.endswith(".changes.jsonl")]
taken = 0
for f in sorted(names, key=lambda f: -os.path.getmtime(os.path.join(src, f))):
  size = os.path.getsize(os.path.join(src, f))
  if taken < many and 0 < size <= most:
    with open(os.path.join(src, f), "rb") as r:
      data = r.read(size)
    with open(os.path.join(dst, f), "wb") as w:
      w.write(data[: data.rfind(b"\n") + 1])
    taken += 1
EOF
cp {capturer} tui/script/converge-capture.ts && FURB_CONVERGE_OUT="$out/life" FURB_CONVERGE_RECORDS="$(ls "$out"/records/*.jsonl 2>/dev/null)" bun run tui/script/converge-capture.ts >"$out/life.log" 2>&1 || fail life "$out/life.log"
rm -f tui/script/converge-capture.ts"""

SHOTS = r"""import hashlib, os, shutil, sys
out, images, repo = sys.argv[1], sys.argv[2], sys.argv[3]
if not images:
  sessions = os.path.join(repo, ".furb", "sessions")
  records = [os.path.join(sessions, f) for f in os.listdir(sessions) if f.endswith(".jsonl") and not f.endswith(".changes.jsonl")]
  images = max(records, key=os.path.getmtime) + ".images"
os.makedirs(images, exist_ok=True)
for part in ("demo", "life"):
  folder = os.path.join(out, part)
  for f in sorted(os.listdir(folder)) if os.path.isdir(folder) else []:
    if f.endswith(".png"):
      with open(os.path.join(folder, f), "rb") as r:
        name = hashlib.sha256(r.read()).hexdigest() + ".png"
      if not os.path.exists(os.path.join(images, name)):
        shutil.copyfile(os.path.join(folder, f), os.path.join(images, name))
      print("S\t" + part + "/" + f[:-4] + "\t" + name)
fails = os.path.join(out, "fails.txt")
for line in open(fails, encoding="utf-8").read().splitlines() if os.path.exists(fails) else []:
  name, _, log = line.partition("\t")
  tail = open(log, encoding="utf-8", errors="replace").read()[-800:] if os.path.exists(log) else ""
  print("F\t" + name + "\t" + " ".join(tail.split()))
"""


def conductor() -> dict:
  return module(ROOT)


def tree_of(target: dict) -> str:
  return str(target.get("tree") or f"{cwd()}-converge-{target['name']}")


def tui_capture(target: dict, sha: str, out: str) -> str:
  shq = conductor()["shq"]
  given = {
    "{repo}": shq(cwd()),
    "{tree}": shq(tree_of(target)),
    "{out}": shq(out),
    "{sha}": shq(sha),
    "{name}": str(target["name"]),
    "{records}": str(RECORDS),
    "{heaviest}": str(HEAVIEST),
    "{capturer}": shq(CAPTURER or f"{cwd()}/.furb/extensions/converge/capture.ts"),
  }
  command = TUI_CAPTURE
  for key, value in given.items():
    command = command.replace(key, value)
  return command


TUI = {
  "name": "tui",
  "words": (14, 15, 16, 17, 28),
  "paths": ("tui/", "bind/typescript/"),
  "capture": tui_capture,
  "checks": (),
  "owners": ("tui-design", "tui-compact"),
  "thread": "tui",
  "tree": "",
  "capturing": "Capture each screen that you change with the real renderer and no window, as"
  " FURB_GALLERY_ONLY=<pattern> FURB_GALLERY_OUT=<folder> bun run screenshots does, and read each capture against his"
  " words before you push. Never open a window on this machine.",
}


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
    ("step", None),
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
  got = await bash(" && echo && ".join(views), show=HIDDEN, timeout=300)
  if got.code != 0 or not got.stdout.content.strip():
    raise Refused(f"the words of the owner on {target['name']} did not load: {got.stderr.content[-300:]}")
  return got.stdout.content.strip()


async def main_head() -> str:
  got = await bash("git fetch -q origin main && git rev-parse origin/main", show=HIDDEN, timeout=300)
  return got.stdout.content.strip() if got.code == 0 else ""


async def moved(target: dict, since: str, head: str) -> bool:
  if not since or not head:
    return bool(head)
  if since == head:
    return False
  shq = conductor()["shq"]
  paths = " ".join(shq(p) for p in target["paths"])
  got = await bash(f"git diff --quiet {shq(since)} {shq(head)} -- {paths}", show=HIDDEN, timeout=120)
  return got.code != 0


def capturing(target: dict, sha: str, out: str) -> str:
  shq = conductor()["shq"]
  checks = [
    f"( cd {shq(tree_of(target))} && {check} ) >{shq(f'{out}/check{i}.log')} 2>&1"
    f" || printf '%s\\t%s\\n' {shq('check: ' + check)} {shq(f'{out}/check{i}.log')} >>{shq(out + '/fails.txt')}"
    for i, check in enumerate(target["checks"])
  ]
  return "\n".join([f"rm -rf {shq(out)} && mkdir -p {shq(out)}", target["capture"](target, sha, out), *checks])


async def shots(out: str) -> tuple[dict[str, str], list[list[str]]]:
  shq = conductor()["shq"]
  got = await bash(f"python3 -I - {shq(out)} {shq(IMAGES)} {shq(cwd())}", stdin=SHOTS, show=HIDDEN, timeout=600)
  taken: dict[str, str] = {}
  failed: list[list[str]] = []
  for line in got.stdout.lines:
    kind, _, rest = line.partition("\t")
    name, _, value = rest.partition("\t")
    if kind == "S":
      taken[name] = value
    elif kind == "F":
      failed.append([name, f"the step {name} of the capture failed: {value}", ""])
  if got.code != 0:
    failed.append(["capture", f"the list of the captures failed: {got.stderr.content[-600:]}", ""])
  return taken, failed


def judges(target: dict, brief: str, lines: list[str], label: str) -> list[str]:
  quietly = conductor()["quietly"]
  asked = []
  for actor in JUDGES:
    for at in range(0, len(lines), BATCH):
      where = str(quietly(chain, f"converge/{target['name']}/{label}"))
      quietly(rung, f"actor = {actor!r}", on=where)
      quietly(grant, EACH, on=where)
      text = safe(brief.replace("{captures}", "\n\n".join(lines[at : at + BATCH])))
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
      broken.append(f"{one}: {str(got)[:300]}")
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


def stalled(counts: list[int]) -> bool:
  run = counts[len(counts) - counts[::-1].index(0) :] if 0 in counts else counts
  return len(run) > STALL and min(run[-STALL:]) >= min(run[:-STALL])


def listed(gaps: list[list[str]], taken: dict[str, str]) -> str:
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
    ("{gaps}", listed(gaps, taken)),
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


async def settled(target: dict, step: dict, head: str) -> bool:
  c = conductor()
  if step.get("routed"):
    return all(over(w) for w in step["routed"]) or await moved(target, step["sha"], head)
  if any(item[0] == step["key"] for item in c["queue"]):
    return False
  step["worker"] = step["worker"] or next((w for w, key in c["working"].items() if key == step["key"]), "")
  if not step["worker"]:
    return True
  if step["worker"] in c["working"]:
    return False
  got = peek(step["worker"])
  step["pr"] = got[0] if isinstance(got, tuple) and got and isinstance(got[0], int) else 0
  return not step["pr"] or step["pr"] in c["merged"] or step["pr"] not in c["prs"]


TARGETS = [TUI]
LIVE: dict[str, dict] = {}
OPEN: dict[str, dict] = {}


def shared(target: dict) -> dict:
  if target["name"] not in LIVE:
    LIVE[target["name"]] = book_of(target)
  return LIVE[target["name"]]


async def classified(target: dict, book: dict, pr: dict) -> str:
  key = f"{pr['n']} {pr['sha']}"
  if key not in book["heads"]:
    got = await bash(f"gh pr view {pr['n']} --json files --jq '.files[].path'", show=HIDDEN, timeout=120)
    if got.code != 0:
      return "held"
    book["heads"][key] = {"state": "held" if any(f.startswith(tuple(target["paths"])) for f in got.stdout.lines) else "outside"}
  return book["heads"][key]["state"]


async def converge_veto(pr: dict) -> bool:
  key = f"{pr['n']} {pr['sha']}"
  for target in TARGETS:
    book = shared(target)
    state = await classified(target, book, pr)
    if state == "held" and key in book["heads"]:
      job = OPEN.get(key)
      if not job:
        job = started(target, "pr", str(pr["sha"]), int(pr["n"]))
        job["words"] = await ruling(target)
      job = await advanced(target, book, job, job["words"], str(pr["sha"]))
      if job:
        OPEN[key] = job
      else:
        OPEN.pop(key, None)
      booked(target, book)
      state = book["heads"][key]["state"]
    if state not in ("outside", "clean"):
      return True
  return False


def started(target: dict, kind: str, sha: str, n: int = 0) -> dict:
  label = f"pr{n}-{sha[:12]}" if n else f"main-{sha[:12]}"
  out = f"{cwd()}/{BOOKS}/{target['name']}/{label}"
  act = str(bash(capturing(target, sha, out), show=HIDDEN, timeout=7200))
  return {"kind": kind, "sha": sha, "n": n, "out": out, "label": label, "act": act, "asked": []}


async def advanced(target: dict, book: dict, job: dict, words: str, head: str) -> dict:
  if not job["asked"]:
    if not over(job["act"]):
      return job
    taken, failed = await shots(job["out"])
    book["jobs"] += 1
    job.update(taken=taken, failed=failed)
    brief = (JUDGE if job["kind"] == "main" else SCREEN).replace("{name}", str(target["name"]))
    brief = brief.replace("{words}", words).replace("{sha}", job["sha"]).replace("{n}", str(job["n"]))
    if job["kind"] == "main":
      lines = [f"{name}\n![{name}](furb-image://{digest})" for name, digest in taken.items()]
    else:
      lines = [
        (f"{name}, main:\n![main {name}](furb-image://{book['shots'][name]})\n" if name in book["shots"] else "")
        + f"{name}, head:\n![head {name}](furb-image://{digest})"
        for name, digest in taken.items()
        if book["shots"].get(name) != digest
      ]
    job["asked"] = judges(target, brief, lines, job["label"]) or ["none"]
  asked = [a for a in job["asked"] if a != "none"]
  if not all(over(a) for a in asked):
    return job
  found, broken = gaps_of(asked, words, set(job["taken"]))
  gaps = [*job["failed"], *found]
  book["spent"] += cost_of(asked)
  if broken:
    book["passes"].append({"sha": job["sha"], "n": job["n"], "inconclusive": broken})
    if job["kind"] == "pr":
      key = f"{job['n']} {job['sha']}"
      tries = book["heads"].get(key, {}).get("tries", 0) + 1
      book["heads"][key] = {"state": "held" if tries < 2 else "stuck", "tries": tries}
      if tries >= 2:
        tell_home(f"the judges of the PR #{job['n']} at {job['sha'][:7]} gave no answer twice, so the conductor does not merge it.")
    return {}
  if job["kind"] == "pr":
    await screened(target, book, job, gaps, words)
    return {}
  book["passes"].append({"sha": job["sha"], "gaps": gaps})
  if job["taken"]:
    book["shots"] = job["taken"]
  book["words"] = words
  counts = [len(p["gaps"]) for p in book["passes"] if "gaps" in p and not p.get("n")]
  if gaps:
    book["clean"] = 0
    book["step"] = stepped(target, book, gaps, job["taken"], job["sha"], words)
    if stalled(counts):
      book.update(state="stalled", since=head)
      tell_home(
        f"{target['name']} stalled: its last {STALL} passes found {counts[-STALL:]} gaps against the words of the"
        f" owner, no fewer than its best before them. His words or the measure must change, and only he changes his"
        f" words. Its ledger is {BOOKS}/{target['name']}.json, and its last gaps are:\n{listed(gaps, {})[:3000]}"
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
  book["heads"][f"{n} {sha}"] = {"state": "gaps" if gaps else "clean", "gaps": gaps}
  if not gaps:
    return
  body = (
    f"🤖 converge judged the head {sha[:7]} of this PR against the words of the owner on {target['name']}, and"
    " the conductor does not merge it until a head shows none of these gaps:\n\n"
    + "\n".join(f"- {shot}: {gap}" + (f" His words: {quote}" if quote else "") for shot, gap, quote in gaps[:LIMIT])
  )
  await bash(f"gh pr comment {n} --body-file -", stdin=body, show=HIDDEN, timeout=120)
  told = (
    f"converge judged the head {sha[:7]} of your PR #{n} against the words of the owner, which follow, and the"
    f" conductor does not merge it until a head shows none of the gaps below. Fix them and push."
    f"\n\n<words>\n{words}\n</words>\n\nThe gaps:\n\n{listed(gaps, job['taken'])}"
  )
  told = safe(told)
  pr = c["prs"].get(n)
  own = c["owner_of"](pr) if pr else ""
  if own:
    c["quietly"](prompt, None, told, on=c["working_at"](own))
  else:
    c["queue"].append((f"converge-pr{n}-{sha[:7]}", f"The open PR #{n}. " + told, str(target["thread"])))


async def converge(target: dict, ticks: int = 0) -> dict:
  book, job, tick = shared(target), {}, 0
  while not ticks or tick < ticks:
    tick += 1
    head = await main_head()
    words = await ruling(target)
    if book["state"] in ("converged", "stalled") and (words != book["words"] or await moved(target, book["since"], head)):
      book.update(state="", clean=0)
    if book["step"] and await settled(target, book["step"], head):
      book["step"] = None
    if not job and book["state"] != "spent" and (book["jobs"] >= JOBS or book["spent"] >= USD):
      book["state"] = "spent"
      tell_home(f"{target['name']} spent its budget: {book['jobs']} captures and {book['spent']:.2f} dollars.")
    if not job and book["state"] != "spent" and head:
      if not book["state"] and not book["step"]:
        job = started(target, "main", head)
    while job:
      was = job
      job = await advanced(target, book, job, words, head)
      if job is was:
        break
    booked(target, book)
    if book["state"] == "spent" and not job:
      return book
    if not job:
      await wait(EVERY)
    else:
      await wait(EVERY, *[a for a in ([job["act"]] if not job["asked"] else job["asked"]) if a != "none" and not over(a)])
  return book

if scope(acting()) == ROOT:
  conductor().get("vetoes", {})["converge"] = converge_veto
