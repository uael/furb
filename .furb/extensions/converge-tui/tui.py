TUI_CONVERGING = """The TUI is the first target of converge, and its parts are these.
Each capture holds the clock of the view still at STILL, so that no spinner turns and no time passes in a frame, and a capture changed when its cells changed, with each time that the view measures and each frame of its spinner read as one mark: a capture of the same code in another run did not change. A judge reads a capture that changed as the text of its frame, with a mark on each row that changed, the row that it replaced and the colors of each changed row. A judge gets an image of a capture only when its change is in its colors alone, when the capture is new, and in a pass of main.
Each tree of the target has a home of the gallery of its own under FURB_GALLERY_ROOT, so that the captures that run at once hold no lock of one another. The capture opens RECORDS records of the project anew in each part, none over HEAVIEST bytes."""
RECORDS = 3
HEAVIEST = 160_000_000
STILL = 1_767_225_600_000
IMAGES = ""
CAPTURER = ""
TUI_BUSY = r"furb-gallery\.lock|holds the home|lock held|is busy"
TUI_LOOK = "each line, each color, each cut, each overlap, each empty place"

TUI_LEGEND = """Each capture stands in a capture tag, under its name, and its change says why it is here. Its text is the frame that the renderer drew, one numbered line for each row of the screen, less the runs of rows that did not change, which it names. A row marked + changed: under it, the line {was} gives what {was} showed in the columns that changed, and colors gives the colors of those columns now. A row marked ~ changed in its colors alone, which it gives for both. A row marked - is a row of {was} that is gone, and a row marked + with no line {was} under it is new, with the colors of its whole row. A color is a key and its columns, and styles names each key: its foreground, its background and its attributes. A capture whose change is the same as the change of another capture names that capture. An image shows a capture as a terminal draws it, when the capture comes with one."""

TUI_CAPTURE = r"""repo={repo}; tree={tree}; out={out}; export FURB_GALLERY_ROOT={gallery}
fail() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >>"$out/fails.txt"; }
[ -d "$tree" ] || "$repo/script/worktree.sh" "$tree" converge/{branch} >"$out/tree.log" 2>&1 || { fail head tree "$out/tree.log"; exit 0; }
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


def gallery_of(target: dict) -> str:
  """The root of the homes of a run of the gallery in a tree, so that a run in another tree holds no lock of it."""
  return f"/tmp/furb-converge-{target['name']}-{int(target.get('slot', 1))}"


def tui_capture(target: dict, pairs: list[tuple[str, str]], out: str) -> str:
  shq = conductor()["shq"]
  given = {
    "{repo}": shq(cwd()),
    "{tree}": shq(tree_of(target)),
    "{gallery}": shq(gallery_of(target)),
    "{branch}": f"{target['name']}-{int(target.get('slot', 1))}",
    "{out}": shq(out),
    "{pairs}": " ".join(shq(f"{part}:{sha}") for part, sha in pairs),
    "{name}": str(target["name"]),
    "{records}": str(RECORDS),
    "{heaviest}": str(HEAVIEST),
    "{still}": str(STILL),
    "{capturer}": shq(CAPTURER or f"{cwd()}/.furb/extensions/converge-tui/capture.ts"),
  }
  command = TUI_CAPTURE
  for key, value in given.items():
    command = command.replace(key, value)
  return command


def tui_differ(target: dict, out: str, was: str, whole: bool, labels: tuple[str, str]) -> tuple[str, str]:
  shq = conductor()["shq"]
  words = [out, IMAGES, cwd(), f"{out}/head", was, "1" if whole else "0", *labels]
  return "python3 -I - " + " ".join(shq(x) for x in words), SHOTS


TUI = {
  "name": "tui",
  "words": (14, 15, 16, 17, 28),
  "paths": ("tui/", "bind/typescript/"),
  "capture": tui_capture,
  "differ": tui_differ,
  "legend": TUI_LEGEND,
  "look": TUI_LOOK,
  "busy": TUI_BUSY,
  "checks": (),
  "owners": ("tui-design", "tui-compact"),
  "thread": "tui",
  "tree": "",
  "capturing": "Capture each screen that you change with the real renderer and no window, as"
  " FURB_GALLERY_ONLY=<pattern> FURB_GALLERY_OUT=<folder> bun run screenshots does, and read each capture against his"
  " words before you push. Never open a window on this machine.",
}

TARGETS = [*[one for one in TARGETS if one["name"] != TUI["name"]], TUI]
