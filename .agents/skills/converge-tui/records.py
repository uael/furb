"""Copy the newest records of the project into <out>/records, each cut at its last whole line, and list each with its
size in <out>/records.json. A record that <out>/pins.json names keeps its pinned size, so that a later capture opens the
same records at the same sizes.

Usage: python3 -I records.py <folder of the records> <out> <how many> <largest size in bytes>
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def chosen(source: Path, pins: dict, many: int, most: int) -> dict[str, int]:
  names = [one for one in source.iterdir() if one.suffix == ".jsonl" and not one.name.endswith(".changes.jsonl")]
  size = {one.name: one.stat().st_size for one in names}
  kept = {name: n for name, n in pins.items() if name in size and isinstance(n, int) and 0 < n <= size[name]}
  for one in sorted(names, key=lambda one: -one.stat().st_mtime):
    if len(kept) < many and one.name not in kept and 0 < size[one.name] <= most:
      kept[one.name] = size[one.name]
  return kept


if __name__ == "__main__":
  source, out, many, most = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
  try:
    pins = json.loads((out / "pins.json").read_text(encoding="utf-8"))
  except (OSError, ValueError):
    pins = {}
  taken = {}
  for name, n in chosen(source, pins if isinstance(pins, dict) else {}, many, most).items():
    with (source / name).open("rb") as given:
      data = given.read(n)
    data = data[: data.rfind(b"\n") + 1]
    (out / "records" / name).write_bytes(data)
    taken[name] = len(data)
  (out / "records.json").write_text(json.dumps(taken), encoding="utf-8")
