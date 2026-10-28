import json
import re

CONVERGING = """converge(target) runs one loop toward the words of the owner, and nothing else: no spec, no design, no plan.
A target is a dict: its name; words, the numbers of the issues of the owner in the private repository of the decisions, whose bodies and comments are his ruling as they stand; paths, the files whose change it judges; capture(target, pairs, out), the command that captures it at each (part, sha) of pairs in one run, on the same records, as PNG files and the cells that the renderer drew, in out/part; checks, the commands that must pass at the last sha; owners, the keys of the items of the round whose workers own its parts; and thread, the thread of its items.
Each capture holds the clock of the view still at STILL, so that no spinner turns and no time passes in a frame, and a capture changed when its cells changed, with each time that the view measures and each frame of its spinner read as one mark: a capture of the same code in another run did not change. A judge reads a capture that changed as the text of its frame, with a mark on each row that changed, the row that it replaced and the colors of each changed row. A judge gets an image of a capture only when its change is in its colors alone, when the capture is new, and in a pass of main.
Each pass captures the target at the head of main, and each judge in JUDGES, blind on a chain of its own, reads against his words each capture that changed since the last pass, or each capture when his words changed or no pass judged them, and closes with the gaps that it sees. A capture that did not change keeps the gaps of the last pass. A gap counts only when it names a capture that the judge read and quotes his words exactly, and a failed check of the head is a gap of its own. A step of the capture that fails is a fault of the tool and no gap: converge rebuilds the tree once and captures again, and when the step fails again it tells the operator and judges nothing, so a PR passes and a pass of main waits for a change of main under its paths or of his words.
A pass with gaps is one step: the gaps go to each worker of an owner that works, or to one item in the queue of the round, which fixes the largest gap in one PR. The next pass comes once that work ended and its PR landed.
The target converges after CLEAN passes in a row with no gap, and stalls after STALL passes with no fewer gaps than its best before them: each tells the operator through the round, and the loop then waits for a change of his words or of main under paths.
Before the conductor merges a PR that changes paths, it asks converge through its vetoes: once the checks of the PR are green, converge captures main and main with the PR merged in it in one run, so that no change of main shows as a change of the PR, sends each judge his words once in each prompt, with the captures that the PR changes in prompts of BATCH captures and PROMPT characters at most, and cuts a capture over CUT characters with a note, and says no until a head shows no gap. A PR that changes no capture sends nothing to a judge. The gaps go to its worker and to a comment, and a new head is judged again. converge puts no label on a PR.
A grant holds each chain of a judge under EACH dollars. A judge that fails, pauses or gives no answer in BOUND seconds gives no answer: its pass is inconclusive, and a head whose judges gave no answer twice is stuck, which tells the operator and keeps the veto until a new head.
The ledger of a target, .furb/converge/<name>.json, keeps each pass, the frames and the records of the last pass, each judged head and the step that runs, so a later life goes on from it. The loop ends when it spent JOBS captures or USD dollars."""
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
RECORDS = 3
HEAVIEST = 160_000_000
STILL = 1_767_225_600_000
BOOKS = ".furb/converge"
IMAGES = ""
CAPTURER = ""
HOME = ROOT

LEGEND = """Each capture stands in a capture tag, under its name, and its change says why it is here. Its text is the frame that the renderer drew, one numbered line for each row of the screen, less the runs of rows that did not change, which it names. A row marked + changed: under it, the line {was} gives what {was} showed in the columns that changed, and colors gives the colors of those columns now. A row marked ~ changed in its colors alone, which it gives for both. A row marked - is a row of {was} that is gone, and a row marked + with no line {was} under it is new, with the colors of its whole row. A color is a key and its columns, and styles names each key: its foreground, its background and its attributes. A capture whose change is the same as the change of another capture names that capture. An image shows a capture as a terminal draws it, when the capture comes with one."""

JUDGE = """You judge {name} against the words of its owner, quoted below as he wrote them, and against nothing else: no spec, no plan and no taste of your own. Every single detail counts.
Read his words whole, then look at each capture whole: each line, each color, each cut, each overlap, each empty place, and each thing that his words ask for and that the capture does not show.
{legend} before is the last pass that judged the capture, and now is the head of main.
Close with the list of the gaps that you see, each a tuple (capture, gap, quote): the name of the capture that shows it, as its tag gives it; what the capture shows against his words, in one or two sentences that a worker can act on; and the words of the owner that it fails, copied exactly from below, twelve characters at least. A gap that names no capture below, or whose quote is not in his words, does not count. Close with [] only when each capture serves his words.

<words>
{words}
</words>

The captures of {name} at {sha} that changed since the last pass, or each capture when no pass judged them against these words:

{captures}"""

SCREEN = """You judge the PR #{n} of {name} against the words of its owner, quoted below as he wrote them, and against nothing else: no spec, no plan and no taste of your own. Every single detail counts.
Each capture below is one that the PR changes: main, and main with the PR merged in it (the head {sha} of the PR), were captured in the same run, and a capture that the PR does not change is not below. Judge only what the PR changes: a gap counts when the head shows it against his words, and a gap that main shows the same way is not a gap of this PR. Look at each change whole: each line, each color, each cut, each overlap, each empty place, and each thing that his words ask for and that the head does not show.
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

TUI_CAPTURE = r"""repo={repo}; tree={tree}; out={out}
fail() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >>"$out/fails.txt"; }
[ -d "$tree" ] || "$repo/script/worktree.sh" "$tree" converge/{name} >"$out/tree.log" 2>&1 || { fail head tree "$out/tree.log"; exit 0; }
[ -z "$fresh" ] || rm -f "$tree/target/converge-built"
mkdir -p "$out/records"
python3 -I - "$repo/.furb/sessions" "$out" {records} {heaviest} >"$out/records.log" 2>&1 <<'EOF' || fail head records "$out/records.log"
import json, os, sys
src, out, many, most = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
try:
  with open(os.path.join(out, "pins.json"), encoding="utf-8") as r:
    pinned = json.load(r)
except (OSError, ValueError):
  pinned = {}
names = [f for f in os.listdir(src) if f.endswith(".jsonl") and not f.endswith(".changes.jsonl")]
size = {f: os.path.getsize(os.path.join(src, f)) for f in names}
chosen = {f: n for f, n in pinned.items() if f in size and isinstance(n, int) and 0 < n <= size[f]}
for f in sorted(names, key=lambda f: -os.path.getmtime(os.path.join(src, f))):
  if len(chosen) < many and f not in chosen and 0 < size[f] <= most:
    chosen[f] = size[f]
taken = {}
for f, n in chosen.items():
  with open(os.path.join(src, f), "rb") as r:
    data = r.read(n)
  data = data[: data.rfind(b"\n") + 1]
  with open(os.path.join(out, "records", f), "wb") as w:
    w.write(data)
  taken[f] = len(data)
with open(os.path.join(out, "records.json"), "w", encoding="utf-8") as w:
  json.dump(taken, w)
EOF
for pair in {pairs}; do
  part=${pair%%:*}; sha=${pair#*:}; at="$out/$part"; mkdir -p "$at"
  (
    cd "$tree" || exit 0
    { git fetch -q origin main && { git cat-file -e "$sha^{commit}" 2>/dev/null || git fetch -q origin "$sha"; } && git checkout -q --detach "$sha" && git clean -fdq; } >"$at/checkout.log" 2>&1 || { fail "$part" checkout "$at/checkout.log"; exit 0; }
    built=$(cat target/converge-built 2>/dev/null)
    if [ -z "$built" ] || ! git diff --quiet "$built" "$sha" -- src bind build.rs Cargo.toml Cargo.lock package.json bun.lock; then
      build=; [ -f "$TMPDIR/build.sh" ] && build="sh $TMPDIR/build.sh"
      { bun install --frozen-lockfile && $build bun run build; } >"$at/build.log" 2>&1 || { fail "$part" build "$at/build.log"; exit 0; }
      mkdir -p target && echo "$sha" >target/converge-built
    fi
    FURB_STILL_CLOCK={still} FURB_GALLERY_OUT="$at/demo" FURB_GALLERY_CELLS="$at/demo" bun run screenshots >"$at/demo.log" 2>&1 || fail "$part" demo "$at/demo.log"
    rm -rf "$out/open" && cp -R "$out/records" "$out/open" && cp {capturer} tui/script/converge-capture.ts \
      && FURB_STILL_CLOCK={still} FURB_CONVERGE_OUT="$at/life" FURB_CONVERGE_RECORDS="$(ls "$out"/open/*.jsonl 2>/dev/null)" \
      bun run tui/script/converge-capture.ts >"$at/life.log" 2>&1 || fail "$part" life "$at/life.log"
    rm -f tui/script/converge-capture.ts
  )
done"""

SHOTS = r"""import difflib, hashlib, json, os, re, shutil, sys
out, images, repo, now_dir, was_dir, whole, was_label, now_label = sys.argv[1:9]
if not images:
  sessions = os.path.join(repo, ".furb", "sessions")
  records = [os.path.join(sessions, f) for f in os.listdir(sessions) if f.endswith(".jsonl") and not f.endswith(".changes.jsonl")]
  images = max(records, key=os.path.getmtime) + ".images"
os.makedirs(images, exist_ok=True)
SPIN = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
TIME = re.compile(r"( *)(?<![\w.])(\d+(?:\.\d+)?(?:ms|s|m|h)(?: \d+[smh])?|\d+ (?:minute|hour|day|week)s? ago)(?!\w)( *)")
EPOCH = re.compile(r"()(?<!\d)(\d{13})(?!\d)()")
CONTEXT = 2
ATTRIBUTES = ((1, "bold"), (2, "dim"), (4, "italic"), (8, "underline"), (16, "blink"), (32, "inverse"), (64, "hidden"), (128, "strikethrough"))


def digest(path):
  with open(path, "rb") as r:
    return hashlib.sha256(r.read()).hexdigest() + ".png"


def stored(path):
  name = digest(path)
  if not os.path.exists(os.path.join(images, name)):
    shutil.copyfile(path, os.path.join(images, name))
  return name


def grid(path):
  try:
    with open(path, encoding="utf-8") as r:
      lines = json.load(r)["lines"]
    return [
      [(c, fg + " on " + bg + "".join(" " + word for bit, word in ATTRIBUTES if attrs & bit)) for text, fg, bg, attrs in spans for c in text]
      for spans in lines
    ]
  except (OSError, ValueError, KeyError, TypeError):
    return None


def masked(cells, pattern, mark):
  text, kept, at = "".join(c for c, _ in cells), [], 0
  hits = list(pattern.finditer(text))
  for hit in hits:
    kept += cells[at : hit.start()]
    if hit[1]:
      kept.append((" ", cells[hit.start(1)][1]))
    style = cells[hit.start(2)][1]
    kept += [(c, style) for c in mark(hit[2])]
    if hit[3]:
      kept.append((" ", cells[hit.start(3)][1]))
    at = hit.end()
  return kept + cells[at:], bool(hits)


def still(cells):
  kept, timed = masked(cells, TIME, lambda word: re.sub(r"s ago$", " ago", re.sub(r"\d+(?:\.\d+)?", "0", word)))
  kept, dated = masked(kept, EPOCH, lambda word: "0")
  if timed or dated:
    kept = [one for at, one in enumerate(kept) if not (one[0] == " " and at and kept[at - 1][0] == " ")]
  return "".join("⠋" if c in SPIN else c for c, _ in kept), tuple(s for _, s in kept)


def norm(png, lines):
  return digest(png) if lines is None else hashlib.sha256(json.dumps([still(x) for x in lines]).encode()).hexdigest()


def plain(cells):
  return "".join(c for c, _ in cells).rstrip()


def colors(cells, keys, lo=0, hi=None):
  hi = len(cells) - 1 if hi is None else min(hi, len(cells) - 1)
  runs, start = [], lo
  for at in range(lo + 1, hi + 2):
    if at == hi + 1 or cells[at][1] != cells[start][1]:
      if cells[start][1] not in keys:
        n, name = len(keys), ""
        while n >= 0:
          name, n = chr(97 + n % 26) + name, n // 26 - 1
        keys[cells[start][1]] = name
      runs.append(keys[cells[start][1]] + str(start + 1) + (f"-{at}" if at - start > 1 else ""))
      start = at
  return " ".join(runs)


def frame(lines):
  return "\n".join(f" {n:>3} {plain(line)}" for n, line in enumerate(lines, 1))


def span(was, now):
  size = max(len(was), len(now))
  pad = [(" ", "")] * size
  a, b = (was + pad)[:size], (now + pad)[:size]
  differ = [at for at in range(size) if a[at] != b[at]]
  return differ[0], differ[-1]


def paired(was, now, j, keys):
  lo, hi = span(was, now)
  where = f"{lo + 1}-{hi + 1}" if hi > lo else f"{lo + 1}"
  if plain(was[lo : hi + 1]) == plain(now[lo : hi + 1]) and len(was) == len(now):
    return [f"~{j + 1:>3} {plain(now)}", f"     colors {was_label} {where}: {colors(was, keys, lo, hi)}", f"     colors {now_label} {where}: {colors(now, keys, lo, hi)}"]
  told = [f"+{j + 1:>3} {plain(now)}", f"     {was_label} {where}: {''.join(c for c, _ in was[lo : hi + 1]).rstrip()}"]
  return [*told, f"     colors {where}: {colors(now, keys, lo, hi)}"] if lo < len(now) else told


def compared(was, now):
  a, b, keys, rows = [still(x) for x in was], [still(x) for x in now], {}, []
  for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, [x[0] for x in a], [x[0] for x in b], autojunk=False).get_opcodes():
    if tag == "equal" or (tag == "replace" and i2 - i1 == j2 - j1):
      for i, j in zip(range(i1, i2), range(j1, j2)):
        rows.append((j + 1, [f" {j + 1:>3} {plain(now[j])}"]) if a[i] == b[j] else (0, paired(was[i], now[j], j, keys)))
      continue
    told = [f"-{i + 1:>3} {plain(was[i])}" for i in range(i1, i2)]
    for j in range(j1, j2):
      told += [f"+{j + 1:>3} {plain(now[j])}", f"     colors: {colors(now[j], keys)}"]
    rows.append((0, told))
  marks = [k for k, (same, _) in enumerate(rows) if not same]
  told, hidden = [], []
  for k, (same, lines) in enumerate(rows):
    if same and not any(abs(k - m) <= CONTEXT for m in marks):
      hidden.append(same)
      continue
    if hidden:
      told.append(f"     rows {hidden[0]}-{hidden[-1]} as {was_label}")
      hidden = []
    told += lines
  if hidden:
    told.append(f"     rows {hidden[0]}-{hidden[-1]} as {was_label}")
  return "\n".join(["     styles: " + ", ".join(f"{name} {style}" for style, name in keys.items()), *told])


def found(root):
  shots = {}
  for part in ("demo", "life"):
    folder = os.path.join(root, part)
    for f in sorted(os.listdir(folder)) if root and os.path.isdir(folder) else []:
      if f.endswith(".png"):
        shots[part + "/" + f[:-4]] = (os.path.join(folder, f), os.path.join(folder, f[:-4] + ".json"))
  return shots


nows, wases = found(now_dir), found(was_dir)
seen = {"shots": {}, "failed": [], "faults": [], "changed": [], "kept": [], "removed": sorted(set(wases) - set(nows)), "records": {}}
for name, (png, cells) in nows.items():
  mine = seen["shots"][name] = stored(png)
  now = grid(cells)
  if name not in wases:
    seen["changed"].append({"name": name, "kind": "new", "text": "" if now is None else frame(now), "images": [[now_label, mine]]})
    continue
  was_png, was_cells = wases[name]
  was = grid(was_cells)
  if norm(was_png, was) == norm(png, now):
    seen["kept"].append(name)
    continue
  pair = [[was_label, stored(was_png)], [now_label, mine]]
  if now is None or was is None:
    seen["changed"].append({"name": name, "kind": "image", "text": "", "images": pair})
  elif [still(x)[0] for x in was] == [still(x)[0] for x in now]:
    seen["changed"].append({"name": name, "kind": "colors", "text": compared(was, now), "images": pair})
  else:
    shown = [[now_label, mine]] if whole == "1" else []
    text = compared(was, now)
    first = next((one["name"] for one in seen["changed"] if one["text"] == text and not shown), "")
    told = {"kind": "same", "text": f"the same change as {first}"} if first else {"kind": "text", "text": text}
    seen["changed"].append({"name": name, **told, "images": shown})
fails = os.path.join(out, "fails.txt")
for line in open(fails, encoding="utf-8").read().splitlines() if os.path.exists(fails) else []:
  fields = line.split("\t")
  part, name, log = fields if len(fields) == 3 else ["head", *fields, ""][:3]
  tail = open(log, encoding="utf-8", errors="replace").read()[-800:] if os.path.exists(log) else ""
  entry = [name, f"the step {name} of the capture failed: " + " ".join(tail.split()), ""]
  seen["failed" if part == os.path.basename(now_dir) and name.startswith("check: ") else "faults"].append(entry)
pins = os.path.join(out, "records.json")
if os.path.exists(pins):
  with open(pins, encoding="utf-8") as r:
    seen["records"] = json.load(r)
with open(os.path.join(out, "shots.json"), "w", encoding="utf-8") as w:
  json.dump(seen, w)
print("wrote " + str(len(seen["changed"])) + " changed captures")
"""


def conductor() -> dict:
  return module()


def tree_of(target: dict) -> str:
  return str(target.get("tree") or f"{cwd()}-converge-{target['name']}")


def tui_capture(target: dict, pairs: list[tuple[str, str]], out: str) -> str:
  shq = conductor()["shq"]
  given = {
    "{repo}": shq(cwd()),
    "{tree}": shq(tree_of(target)),
    "{out}": shq(out),
    "{pairs}": " ".join(shq(f"{part}:{sha}") for part, sha in pairs),
    "{name}": str(target["name"]),
    "{records}": str(RECORDS),
    "{heaviest}": str(HEAVIEST),
    "{still}": str(STILL),
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
    ("faulted", ""),
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


def capturing(target: dict, pairs: list[tuple[str, str]], out: str, pins: dict, fresh: bool = False) -> str:
  shq = conductor()["shq"]
  checks = [
    f"( cd {shq(tree_of(target))} && {check} ) >{shq(f'{out}/check{i}.log')} 2>&1"
    f" || printf 'head\\t%s\\t%s\\n' {shq('check: ' + check)} {shq(f'{out}/check{i}.log')} >>{shq(out + '/fails.txt')}"
    for i, check in enumerate(target["checks"])
  ]
  pinned = f"printf '%s' {shq(json.dumps(pins))} >{shq(out + '/pins.json')}"
  return "\n".join([f"rm -rf {shq(out)} && mkdir -p {shq(out)}", *(["fresh=1"] if fresh else []), pinned, target["capture"](target, pairs, out), *checks])


async def shots(out: str, was: str, whole: bool, labels: tuple[str, str]) -> dict:
  shq = conductor()["shq"]
  words = [out, IMAGES, cwd(), f"{out}/head", was, "1" if whole else "0", *labels]
  got = await bash("python3 -I - " + " ".join(shq(x) for x in words), stdin=SHOTS, show=HIDDEN, timeout=900)
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


async def started(target: dict, book: dict, sha: str, n: int = 0) -> dict:
  shq = conductor()["shq"]
  pairs = [("head", sha)]
  if n:
    label = f"pr{n}-{sha[:12]}"
    # The head is main with the PR merged in it, so a change of main shows in neither part of the pair.
    make = "t=$(git merge-tree --write-tree origin/main " + shq(sha) + ") && git rev-parse origin/main && git commit-tree \"$t\" -p origin/main -m converge"
    got = await bash(f"git fetch -q origin main refs/pull/{n}/head && {make}", show=HIDDEN, timeout=300)
    made = got.stdout.lines
    if got.code != 0 or len(made) != 2:
      return {}
    pairs = [("before", made[0].strip()), ("head", made[1].strip())]
  else:
    label = f"main{len(book['passes'])}-{sha[:12]}"
  out = f"{cwd()}/{BOOKS}/{target['name']}/{label}"
  act = str(bash(capturing(target, pairs, out, book["records"]), show=HIDDEN, timeout=7200))
  return {"kind": "pr" if n else "main", "sha": sha, "n": n, "out": out, "label": label, "act": act, "pairs": pairs, "asked": []}


async def converge_veto(pr: dict) -> bool:
  key = f"{pr['n']} {pr['sha']}"
  for target in TARGETS:
    book = shared(target)
    state = await classified(target, book, pr)
    if state == "held" and key in book["heads"]:
      job = OPEN.get(key)
      if not job:
        job = await started(target, book, str(pr["sha"]), int(pr["n"]))
        if not job:
          return True
        job["words"] = await ruling(target)
      job = await advanced(target, book, job, job["words"], str(pr["sha"]))
      if job:
        OPEN[key] = job
      else:
        OPEN.pop(key, None)
      booked(target, book)
      state = book["heads"][key]["state"]
    if state not in ("outside", "clean", "unjudged"):
      return True
  return False


def faulted(target: dict, book: dict, job: dict, faults: list[list[str]], words: str, head: str) -> dict:
  said = " ".join(why for _, why, _ in faults)[:800]
  name, sha = target["name"], job["sha"][:7]
  if job["kind"] == "pr":
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
    was = f"{job['out']}/before" if pr else book["frames"] if book["words"] == words else ""
    seen = await shots(job["out"], was, not pr, ("main", "head") if pr else ("before", "now"))
    if seen["faults"]:
      if not job.get("again"):
        job.update(again=True, act=str(bash(capturing(target, job["pairs"], job["out"], book["records"], True), show=HIDDEN, timeout=7200)))
        return job
      return faulted(target, book, job, seen["faults"], words, head)
    book["jobs"] += 1
    job.update(taken=seen["shots"], failed=seen["failed"], kept=seen["kept"], records=seen["records"])
    job["shown"] = [str(one["name"]) for one in seen["changed"]]
    brief = (SCREEN if pr else JUDGE).replace("{legend}", LEGEND.replace("{was}", "main" if pr else "before"))
    brief = brief.replace("{name}", str(target["name"])).replace("{sha}", job["sha"]).replace("{n}", str(job["n"]))
    captures = sections(seen["changed"])
    job["asked"] = judges(target, brief.replace("{words}", words), captures, job["label"], BATCH)
    job["asked"] = job["asked"] or ["none"]
    job["at"] = clock()
  asked = [a for a in job["asked"] if a != "none"]
  late = clock() - float(job.get("at", 0.0)) >= BOUND
  for one in asked:
    if not over(one) and (late or paused(str(get(one)[3]))):
      cancel(one)
  if not all(over(a) for a in asked):
    return job
  found, broken = gaps_of(asked, words, set(job["shown"]))
  book["spent"] += cost_of(asked)
  if broken:
    book["passes"].append({"sha": job["sha"], "n": job["n"], "inconclusive": broken})
    if pr:
      key = f"{job['n']} {job['sha']}"
      tries = book["heads"].get(key, {}).get("tries", 0) + 1
      book["heads"][key] = {"state": "held" if tries < 2 else "stuck", "tries": tries}
      if tries >= 2:
        tell_home(
          f"the judges of the PR #{job['n']} at {job['sha'][:7]} gave no answer twice, so the conductor does not"
          f" merge it until a new head: {'; '.join(broken)[:600]}"
        )
    return {}
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
    said = book["faulted"] if book["state"] == "faulted" else book["words"]
    if book["state"] in ("converged", "stalled", "faulted") and (words != said or await moved(target, book["since"], head)):
      book.update(state="", clean=0)
    if book["step"] and await settled(target, book["step"], head):
      book["step"] = None
    if not job and book["state"] != "spent" and (book["jobs"] >= JOBS or book["spent"] >= USD):
      book["state"] = "spent"
      tell_home(f"{target['name']} spent its budget: {book['jobs']} captures and {book['spent']:.2f} dollars.")
    if not job and book["state"] != "spent" and head:
      if not book["state"] and not book["step"]:
        job = await started(target, book, head)
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

conductor().get("vetoes", {})["converge"] = converge_veto
