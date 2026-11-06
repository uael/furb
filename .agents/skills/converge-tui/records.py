"""Copy the newest records of the project into <out>/records, each cut at its last whole line, and list each with its
size in <out>/records.json. A record that <out>/pins.json names keeps its pinned size, so that a later capture opens the
same records at the same sizes.

It also names in <out>/slice.json the log of a long real life, which tui/script/fixture.ts cuts into a recorded life: the
log and the moment that pins.json pins under the key slice, else the newest log of at least LONG bytes, at its end. Once
the fixture stands, `records.py pin <out> <fixture>` pins the count of facts that it cut at, so that a later capture
shows the same moment of the same life.

Usage: python3 -I records.py <folder of the records> <out> <how many> <largest size in bytes>
       python3 -I records.py pin <out> <fixture>
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


LONG = 300_000_000
"""The least size of the log of a life that the slice takes, in bytes: a life of hours of work."""


def sliced(source: Path, pins: dict) -> dict | None:
  pin = pins.get("slice")
  if isinstance(pin, dict) and isinstance(pin.get("moment"), int) and (source / str(pin.get("log"))).is_file():
    return {"log": str(source / pin["log"]), "moment": pin["moment"]}
  logs = [one for one in source.iterdir() if one.name.endswith(".jsonl.log") and one.stat().st_size >= LONG]
  newest = max(logs, key=lambda one: one.stat().st_mtime, default=None)
  return newest and {"log": str(newest), "moment": "end"}


def pin(out: Path, fixture: Path) -> None:
  with fixture.open(encoding="utf-8") as given:
    header = json.loads(given.readline())
  named = json.loads((out / "slice.json").read_text(encoding="utf-8"))
  taken = json.loads((out / "records.json").read_text(encoding="utf-8"))
  taken["slice"] = {"log": Path(named["log"]).name, "moment": header["cut"]}
  (out / "records.json").write_text(json.dumps(taken), encoding="utf-8")


if __name__ == "__main__" and sys.argv[1] == "pin":
  pin(Path(sys.argv[2]), Path(sys.argv[3]))
elif __name__ == "__main__":
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
  (out / "slice.json").write_text(json.dumps(sliced(source, pins if isinstance(pins, dict) else {})), encoding="utf-8")
