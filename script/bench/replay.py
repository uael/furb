"""The bench of replay: one life of furb on the local model on one task of DeepSWE, then later lives on its record.

Run it from the root of the repository through the lock of the local server, which takes one run at a time:

    sh $TMPDIR/strata.sh uv run python script/bench/replay.py <task> --out <directory>

The first life works the task as the rig of DeepSWE puts it, on the actor of the local server, with no extension. Then
the bench opens later lives on its record with `furb turns`, each a process of its own, and measures for each the
calls to the model, the seconds and the tokens. Then it changes one file of the World that the first life read, opens
one more life, and finds the act where that life stops at the drift, if it stops. Last, as the baseline, it puts the
same task to Claude Code on the same model, in a fresh checkout each time. The server counts the calls and the tokens
of every run, so the bench must hold the server alone.

FURB_LOCAL_URL names the server, and FURB_LOCAL_KEY holds its key.
"""

import argparse
import asyncio
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from urllib.request import Request, urlopen

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import deepswe
from real import lived, say

from furb import engine
from furb.world import answered, kept

ACTOR = "local:qwen3.8-flash-next-iq2_xs"
"""ACTOR is the model of the local server that the first life works on."""
CLAUDE = Path.home() / ".claude" / "strata-claude"
"""CLAUDE is the claude command line on the local server, which the baseline runs."""
FURB = Path(sys.executable).with_name("furb")
"""FURB is the command line that opens each later life."""
CHANGE = "\n# The bench changed this file before this life.\n"
"""CHANGE is what the bench adds to a file of the World before the life that it watches for the drift."""
DRIFT = re.compile(r"(\w+?\d+) drifts")
"""DRIFT finds the act where a life stops in what the life says when its record drifts."""
COUNTS = ("requests", "prompt_tokens", "reused", "output_tokens")
"""COUNTS are the totals of the server that a run changes."""


def server() -> dict[str, float]:
  """The totals of the server now: its requests, the tokens it read, the tokens it took from its cache, and the
  tokens it wrote."""
  url = os.environ["FURB_LOCAL_URL"].rstrip("/").removesuffix("/v1") + "/metrics"
  asked = Request(url, headers={"Authorization": f"Bearer {os.environ['FURB_LOCAL_KEY']}"})  # noqa: S310
  with urlopen(asked, timeout=30) as got:  # noqa: S310
    totals = json.load(got)["totals"]
  return {name: float(totals[name]) for name in COUNTS}


def counted(before: dict[str, float]) -> dict[str, float]:
  """What the server did since `before`."""
  after = server()
  return {name: after[name] - before[name] for name in COUNTS}


def told(task: str) -> tuple[Path, str]:
  """A fresh checkout of the task, and its instruction as the task wrote it."""
  _, path, _, _ = deepswe.seeded(task)
  return deepswe.WORK / task / "app", (path / "instruction.md").read_text(encoding="utf-8").rstrip()


async def first(app: Path, message: str, record: Path, cap: float) -> object:
  """The life that works the task, as the rig puts it: a float that says how sure the model is, and the same chain
  asked to go on under the confidence that the rig wants. At the cap, the bench cancels the prompt, so the record
  ends with no question open and a later life asks the model nothing."""
  world, root, _ = lived(record, app, ACTOR, keeps=True, extensions=False)
  began, got = time.monotonic(), None
  try:
    while (left := cap - (time.monotonic() - began)) > 0:
      act = engine.prompt(float, message, ACTOR, on=root)

      async def answer(of: str = act) -> object:
        return await engine.Act(of)

      try:
        got = await asyncio.wait_for(answer(), left)
      except TimeoutError:
        engine.cancel(act)
        await asyncio.sleep(10)
        return None
      if not isinstance(got, (int, float)) or float(got) >= deepswe.CONFIDENT:
        return got
      message = deepswe.AGAIN.format(got)
    return got
  finally:
    world.end()


def life(record: Path, app: Path) -> dict[str, object]:
  """One later life on the record, a process of its own, and what it cost."""
  before, began = server(), time.monotonic()
  ran = subprocess.run(  # noqa: S603
    [str(FURB), "turns", "--record", str(record), "--cwd", str(app), "--model", ACTOR],
    capture_output=True,
    text=True,
    stdin=subprocess.DEVNULL,
    check=False,
  )
  seconds = round(time.monotonic() - began, 2)
  return {"code": ran.returncode, "seconds": seconds, **counted(before), "said": ran.stderr[-2000:]}


def drifted(record: Path, app: Path) -> dict[str, object]:
  """One more life after a change of the first file of the World that the first life read, and the act where it
  stops: the line of the record that holds that act, of all its lines."""
  facts = [entry[0] for entry in kept(record)]
  paths = [
    Path(fact[3].path)
    for fact in facts
    if fact[0] == "done" and fact[1].startswith(("read", "write")) and isinstance(fact[3], engine.Text)
  ]
  read = next((one for one in paths if one.is_absolute() and one.is_file()), None)
  if read is None:
    return {"changed": None}
  held = read.read_text(encoding="utf-8")
  read.write_text(held + CHANGE, encoding="utf-8")
  try:
    got = life(record, app)
  finally:
    read.write_text(held, encoding="utf-8")
  found = DRIFT.search(str(got["said"]))
  at = next((n for n, fact in enumerate(facts, 1) if found and fact[1] == found[1]), None)
  return {"changed": str(read), **got, "act": found and found[1], "line": at, "lines": len(facts)}


def baseline(task: str, cap: float) -> dict[str, object]:
  """Claude Code on the same model, on a fresh checkout of the task, and what it cost."""
  app, message = told(task)
  before, began = server(), time.monotonic()
  try:
    ran = subprocess.run(  # noqa: S603
      [str(CLAUDE), "-p", message, "--dangerously-skip-permissions", "--output-format", "json"],
      cwd=app,
      capture_output=True,
      text=True,
      stdin=subprocess.DEVNULL,
      timeout=cap,
      check=False,
    )
    code, said = ran.returncode, ran.stdout[-2000:]
  except subprocess.TimeoutExpired:
    code, said = None, f"the cap of {cap:.0f} seconds ran out"
  return {"code": code, "seconds": round(time.monotonic() - began, 2), **counted(before), "said": said}


def main() -> int:
  """The first life, the later lives, the life after a change, and the baseline, each kept in the directory out."""
  deepswe.outside()
  whole = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  whole.add_argument("task", help="The task of DeepSWE, as the collection names it.")
  whole.add_argument("--out", type=Path, required=True, help="The directory of the record and of the results.")
  whole.add_argument(
    "--cap", type=float, default=900.0, help="The seconds that the first life and each run of the baseline may take."
  )
  whole.add_argument("--lives", type=int, default=10, help="How many later lives open on the record.")
  whole.add_argument("--baseline", type=int, default=3, help="How many times Claude Code runs the task.")
  args = whole.parse_args()
  args.out.mkdir(parents=True, exist_ok=True)
  record, results = args.out / "record.jsonl", args.out / "results.json"
  for stale in args.out.glob("record.jsonl*"):
    stale.unlink()
  found: dict[str, object] = {"task": args.task, "actor": ACTOR, "cap": args.cap}

  def keep() -> None:
    results.write_text(json.dumps(found, indent=2, default=repr) + "\n", encoding="utf-8")

  app, message = told(args.task)
  if (app / ".venv" / "bin").is_dir():
    os.environ["PATH"] = os.pathsep.join([str(app / ".venv" / "bin"), os.environ["PATH"]])
  os.chdir(app)
  say(f"[replay] the first life works {args.task} on {ACTOR}, for {args.cap:.0f} seconds at most")
  before, began = server(), time.monotonic()
  got = asyncio.run(first(app, message + deepswe.ASKED, record, args.cap))
  usage = [one[3][2] for one in answered(kept(record)) if one[3][2]]
  found["first"] = {
    "value": got,
    "seconds": round(time.monotonic() - began, 2),
    **counted(before),
    "answers": len(answered(kept(record))),
    "facts": len(kept(record)),
    "record_input": sum(one[0] for one in usage),
    "record_output": sum(one[1] for one in usage),
  }
  keep()
  say(f"[replay] the first life: {found['first']}")
  found["lives"] = []
  for n in range(args.lives):
    one = life(record, app)
    found["lives"].append(one)
    keep()
    say(f"[replay] life {n + 1}: { ({k: v for k, v in one.items() if k != 'said'}) }")
  found["drift"] = drifted(record, app)
  keep()
  say(f"[replay] the life after a change: { ({k: v for k, v in found['drift'].items() if k != 'said'}) }")
  found["baseline"] = []
  for n in range(args.baseline):
    one = baseline(args.task, args.cap)
    found["baseline"].append(one)
    keep()
    say(f"[replay] Claude Code, run {n + 1}: { ({k: v for k, v in one.items() if k != 'said'}) }")
  shutil.copy(record, args.out / "first.jsonl")
  say(f"[replay] the results are in {results}")
  return 0


if __name__ == "__main__":
  sys.exit(main())
