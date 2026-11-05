"""Write <out>/shots.json, the captures of the TUI under <now> that changed since those under <was>, each as the text of
its frame with each changed row marked, and the steps of the capture that failed, which <out>/fails.txt lists.

Usage: python3 -I diff.py <out> <images> <main checkout> <now> <was> <whole> <label of was> <label of now>
A capture changed when its cells changed, with each time that the view measures and each frame of its spinner read as
one mark. <images> keeps each image by the digest of its bytes, and an empty <images> names the folder of the images of
the newest record of the main checkout. With <whole> 1, each capture that changed comes with its image.
"""

from __future__ import annotations

import difflib
import hashlib
import json
import re
import shutil
import sys
from collections.abc import Callable
from pathlib import Path

Cell = tuple[str, str]
Row = list[Cell]
SPIN = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
TIME = re.compile(
  r"( *)(?<![\w.])(\d+(?:\.\d+)?(?:ms|s|m|h)(?: \d+[smh])?|\d+ (?:minute|hour|day|week)s? ago)(?!\w)( *)"
)
EPOCH = re.compile(r"()(?<!\d)(\d{13})(?!\d)()")
CONTEXT = 2
FIELDS = 3
ATTRIBUTES = (
  (1, "bold"),
  (2, "dim"),
  (4, "italic"),
  (8, "underline"),
  (16, "blink"),
  (32, "inverse"),
  (64, "hidden"),
  (128, "strikethrough"),
)


def digest(path: Path) -> str:
  return hashlib.sha256(path.read_bytes()).hexdigest() + ".png"


def stored(path: Path, images: Path) -> str:
  name = digest(path)
  if not (images / name).exists():
    shutil.copyfile(path, images / name)
  return name


def grid(path: Path) -> list[Row] | None:
  try:
    lines = json.loads(path.read_text(encoding="utf-8"))["lines"]
    return [
      [
        (c, fg + " on " + bg + "".join(" " + word for bit, word in ATTRIBUTES if attrs & bit))
        for text, fg, bg, attrs in spans
        for c in text
      ]
      for spans in lines
    ]
  except (OSError, ValueError, KeyError, TypeError):
    return None


def masked(cells: Row, pattern: re.Pattern[str], mark: Callable[[str], str]) -> tuple[Row, bool]:
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


def still(cells: Row) -> tuple[str, tuple[str, ...]]:
  kept, timed = masked(cells, TIME, lambda word: re.sub(r"s ago$", " ago", re.sub(r"\d+(?:\.\d+)?", "0", word)))
  kept, dated = masked(kept, EPOCH, lambda _: "0")
  if timed or dated:
    kept = [one for at, one in enumerate(kept) if not (one[0] == " " and at and kept[at - 1][0] == " ")]
  return "".join("⠋" if c in SPIN else c for c, _ in kept), tuple(s for _, s in kept)


def norm(png: Path, lines: list[Row] | None) -> str:
  return digest(png) if lines is None else hashlib.sha256(json.dumps([still(x) for x in lines]).encode()).hexdigest()


def plain(cells: Row) -> str:
  return "".join(c for c, _ in cells).rstrip()


def colors(cells: Row, keys: dict[str, str], lo: int = 0, hi: int | None = None) -> str:
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


def frame(lines: list[Row]) -> str:
  return "\n".join(f" {n:>3} {plain(line)}" for n, line in enumerate(lines, 1))


def span(was: Row, now: Row) -> tuple[int, int]:
  size = max(len(was), len(now))
  pad = [(" ", "")] * size
  a, b = (was + pad)[:size], (now + pad)[:size]
  differ = [at for at in range(size) if a[at] != b[at]]
  return differ[0], differ[-1]


def paired(was: Row, now: Row, j: int, keys: dict[str, str], labels: tuple[str, str]) -> list[str]:
  lo, hi = span(was, now)
  where = f"{lo + 1}-{hi + 1}" if hi > lo else f"{lo + 1}"
  if plain(was[lo : hi + 1]) == plain(now[lo : hi + 1]) and len(was) == len(now):
    return [
      f"~{j + 1:>3} {plain(now)}",
      f"     colors {labels[0]} {where}: {colors(was, keys, lo, hi)}",
      f"     colors {labels[1]} {where}: {colors(now, keys, lo, hi)}",
    ]
  told = [f"+{j + 1:>3} {plain(now)}", f"     {labels[0]} {where}: {''.join(c for c, _ in was[lo : hi + 1]).rstrip()}"]
  return [*told, f"     colors {where}: {colors(now, keys, lo, hi)}"] if lo < len(now) else told


def compared(was: list[Row], now: list[Row], labels: tuple[str, str]) -> str:
  a, b, keys, rows = [still(x) for x in was], [still(x) for x in now], {}, []
  matcher = difflib.SequenceMatcher(None, [x[0] for x in a], [x[0] for x in b], autojunk=False)
  for tag, i1, i2, j1, j2 in matcher.get_opcodes():
    if tag == "equal" or (tag == "replace" and i2 - i1 == j2 - j1):
      for i, j in zip(range(i1, i2), range(j1, j2)):
        same = a[i] == b[j]
        rows.append((j + 1, [f" {j + 1:>3} {plain(now[j])}"]) if same else (0, paired(was[i], now[j], j, keys, labels)))
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
      told.append(f"     rows {hidden[0]}-{hidden[-1]} as {labels[0]}")
      hidden = []
    told += lines
  if hidden:
    told.append(f"     rows {hidden[0]}-{hidden[-1]} as {labels[0]}")
  return "\n".join(["     styles: " + ", ".join(f"{name} {style}" for style, name in keys.items()), *told])


def found(root: str) -> dict[str, tuple[Path, Path]]:
  shots = {}
  for part in ("demo", "life"):
    folder = Path(root, part)
    for one in sorted(folder.iterdir()) if root and folder.is_dir() else []:
      if one.suffix == ".png":
        shots[f"{part}/{one.stem}"] = (one, one.with_suffix(".json"))
  return shots


def listed(out: Path, images: Path, parts: tuple[str, str], labels: tuple[str, str], *, whole: bool) -> dict:
  now_dir, was_dir = parts
  nows, wases = found(now_dir), found(was_dir)
  seen = {"shots": {}, "failed": [], "faults": [], "changed": [], "kept": [], "removed": sorted(set(wases) - set(nows))}
  for name, (png, cells) in nows.items():
    mine = seen["shots"][name] = stored(png, images)
    now = grid(cells)
    if name not in wases:
      text = "" if now is None else frame(now)
      seen["changed"].append({"name": name, "kind": "new", "text": text, "images": [[labels[1], mine]]})
      continue
    was_png, was_cells = wases[name]
    was = grid(was_cells)
    if norm(was_png, was) == norm(png, now):
      seen["kept"].append(name)
      continue
    pair = [[labels[0], stored(was_png, images)], [labels[1], mine]]
    if now is None or was is None:
      seen["changed"].append({"name": name, "kind": "image", "text": "", "images": pair})
    elif [still(x)[0] for x in was] == [still(x)[0] for x in now]:
      seen["changed"].append({"name": name, "kind": "colors", "text": compared(was, now, labels), "images": pair})
    else:
      shown = [[labels[1], mine]] if whole else []
      text = compared(was, now, labels)
      first = next((one["name"] for one in seen["changed"] if one["text"] == text and not shown), "")
      told = {"kind": "same", "text": f"the same change as {first}"} if first else {"kind": "text", "text": text}
      seen["changed"].append({"name": name, **told, "images": shown})
  fails = out / "fails.txt"
  for line in fails.read_text(encoding="utf-8").splitlines() if fails.exists() else []:
    fields = line.split("\t")
    part, name, log = fields if len(fields) == FIELDS else ["head", *fields, ""][:FIELDS]
    tail = Path(log).read_text(encoding="utf-8", errors="replace")[-800:] if log and Path(log).exists() else ""
    entry = [name, f"the step {name} of the capture failed: " + " ".join(tail.split()), ""]
    seen["failed" if part == Path(now_dir).name and name.startswith("check: ") else "faults"].append(entry)
  pins = out / "records.json"
  seen["records"] = json.loads(pins.read_text(encoding="utf-8")) if pins.exists() else {}
  return seen


if __name__ == "__main__":
  out, images, repo, now_dir, was_dir, whole, was_label, now_label = sys.argv[1:9]
  if not images:
    sessions = Path(repo, ".furb", "sessions")
    records = [one for one in sessions.iterdir() if one.suffix == ".jsonl" and not one.name.endswith(".changes.jsonl")]
    images = str(max(records, key=lambda one: one.stat().st_mtime)) + ".images"
  Path(images).mkdir(parents=True, exist_ok=True)
  seen = listed(Path(out), Path(images), (now_dir, was_dir), (was_label, now_label), whole=whole == "1")
  (Path(out) / "shots.json").write_text(json.dumps(seen), encoding="utf-8")
  sys.stdout.write(f"wrote {len(seen['changed'])} changed captures\n")
