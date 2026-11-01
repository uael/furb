"""The bench of a crash: lives of furb on the local model on tasks of DeepSWE, killed and opened again on their record.

Run it from the root of the repository, one task at a time, through the lock of the local server:

    sh $TMPDIR/strata.sh .venv/bin/python script/bench/crash.py run <task> --out <directory>
    .venv/bin/python script/bench/crash.py report --out <directory>

`run` puts the task to a life on the local model as the rig of DeepSWE puts it, with no extension, in a process of
its own. It kills the group of that process at a random point after the first answer of the life, a number of times, and opens a new life on
the record each time, which wakes the work that the record holds. When the last life ends, it grades the checkout.
`report` reads what each run kept, and gives for each task: whether the work ended before the cap, each command that
ran again after a kill, the extra calls to the model, and the seconds from the start of a process to the wake.

FURB_LOCAL_URL names the server, and FURB_LOCAL_KEY holds its key.
"""

import argparse
import asyncio
import json
import os
import random
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import TypedDict
from urllib.request import Request, urlopen

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import deepswe
from real import lived, say

from furb import engine
from furb.world import answered, kept

ACTOR = "local:qwen3.8-flash-next-iq2_xs"
"""ACTOR is the model of the local server that each life works on."""
OPERATOR = "operator"
"""OPERATOR is the site of each act that the bench makes."""
KILLS = 3
"""KILLS is how many times a run is killed."""
CAP = 1200.0
"""CAP is the seconds that the lives of a run may work, summed, from the wake of each to its end."""
EARLY, LATE = 0.0, 300.0
"""EARLY and LATE bound the seconds from the first answer of a life to its kill, which the bench draws at random."""


def server() -> dict[str, float]:
  """The requests and the tokens of the local server so far, or nothing when it does not say them."""
  url = os.environ.get("FURB_LOCAL_URL", "").rstrip("/").removesuffix("/v1") + "/metrics"
  try:
    asked = Request(url, headers={"Authorization": f"Bearer {os.environ.get('FURB_LOCAL_KEY', '')}"})  # noqa: S310
    with urlopen(asked, timeout=30) as got:  # noqa: S310
      totals = json.load(got)["totals"]
    return {name: float(totals[name]) for name in ("requests", "prompt_tokens", "output_tokens")}
  except OSError, ValueError, KeyError:
    return {}


def line(path: Path, **words: object) -> None:
  """One line of JSON at the end of a file."""
  with path.open("a", encoding="utf-8") as out:
    out.write(json.dumps(words, default=repr) + "\n")


def lines(path: Path) -> list[dict]:
  """Each line of JSON of a file, and none when the file is not there."""
  if not path.is_file():
    return []
  return [json.loads(one) for one in path.read_text(encoding="utf-8").splitlines() if one.strip()]


def asked(root: str) -> str:
  """The last prompt that the bench made on the root, or nothing in a new record."""
  made = [str(x[1]) for x in engine.transcript(root) if x[0] == "prompt" and x[2] == OPERATOR and x[3] == root]
  return made[-1] if made else ""


async def life(args: argparse.Namespace) -> int:
  """One life of a run: a new one asks the task, and a later one wakes the work of its record and waits for it."""
  run = args.out / args.task
  app = deepswe.WORK / args.task / "app"
  told = (deepswe.read_task(args.task)[0] / "instruction.md").read_text(encoding="utf-8").rstrip()
  if (app / ".venv" / "bin").is_dir():
    os.environ["PATH"] = os.pathsep.join([str(app / ".venv" / "bin"), os.environ["PATH"]])
  os.chdir(app)
  record = run / "record.jsonl"
  world, root, held = lived(record, app, ACTOR, keeps=True, extensions=False)
  facts = [one for (one, *_) in held]
  pending = sorted({str(x[1]) for x in facts if x[0] == "started"} - {str(x[1]) for x in facts if x[0] == "done"})
  for chain in dict.fromkeys([root, *(engine.scope(one) for one in pending)]):
    engine.wake(chain)
  act = asked(root) or engine.prompt(float, told + deepswe.ASKED, ACTOR, on=root)
  line(run / "lives.jsonl", life=args.life, ready=time.time(), entries=len(held), pending=pending, act=act)
  began, got, stopped, looks = time.monotonic(), None, "", 0
  mark: dict[str, float] = {"did": 0, "answers": 0}
  try:
    while True:
      while engine.peek(act, ...) is ...:
        if time.monotonic() - began > args.cap:
          stopped = "the cap ran out"
          break
        if (looks := looks + 1) % deepswe.LOOK == 0 and (why := deepswe.watched(record, root, began, mark)):
          stopped = why
          break
        await asyncio.sleep(deepswe.TICK)
      if stopped:
        break
      got = engine.peek(act)
      if not isinstance(got, (int, float)) or float(got) >= deepswe.CONFIDENT:
        break
      act = engine.prompt(float, deepswe.AGAIN.format(got), ACTOR, on=root)
  except Exception as no:
    stopped = f"the task was refused: {no!r}"
  finally:
    world.end()
  line(run / "lives.jsonl", life=args.life, end=time.time(), value=got, stopped=stopped)
  return 0


def woken(run: Path, n: int, child: subprocess.Popen) -> float:
  """The time at which the life n woke its work, or nothing when its process ended before that."""
  while True:
    ended = child.poll() is not None
    for one in lines(run / "lives.jsonl"):
      if one.get("life") == n and "ready" in one:
        return float(one["ready"])
    if ended:
      return 0.0
    time.sleep(0.5)


def answers(record: Path) -> int:
  """How many answers of a model the record holds."""
  return len(answered(kept(record))) if record.is_file() else 0


def answered_once(record: Path, before: int, child: subprocess.Popen) -> float:
  """The time at which the record holds an answer more than it held before, or nothing when the process ended first,
  so that each life works before its kill."""
  while child.poll() is None:
    if answers(record) > before:
      return time.time()
    time.sleep(2.0)
  return 0.0


def crash(args: argparse.Namespace) -> int:
  """A run of a task: a life, killed at random points and opened again on its record, then the grade of the checkout."""
  run = args.out / args.task
  shutil.rmtree(run, ignore_errors=True)
  run.mkdir(parents=True)
  seed = args.seed if args.seed is not None else random.randrange(1 << 30)  # noqa: S311
  draw = random.Random(seed)  # noqa: S311
  work, task, _, syn = deepswe.seeded(args.task)
  record = run / "record.jsonl"
  before, worked = server(), 0.0
  say(f"[crash] {args.task}: {args.kills} kills, a cap of {args.cap:.0f} seconds, seed {seed}")
  for n in range(args.kills + 1):
    spawned = time.time()
    with (run / f"life{n}.out").open("w", encoding="utf-8") as out:
      child = subprocess.Popen(  # noqa: S603
        [
          sys.executable,
          __file__,
          "life",
          args.task,
          *("--out", str(args.out), "--life", str(n), "--cap", str(max(args.cap - worked, 1.0))),
        ],
        stdin=subprocess.DEVNULL,
        stdout=out,
        stderr=subprocess.STDOUT,
        start_new_session=True,
      )
      held = answers(record)
      woke = woken(run, n, child)
      delay, killed = draw.uniform(EARLY, LATE), False
      if n < args.kills and woke and (first := answered_once(record, held, child)):
        try:
          child.wait(timeout=max(delay - (time.time() - first), 0.0))
        except subprocess.TimeoutExpired:
          os.killpg(child.pid, signal.SIGKILL)
          child.wait()
          killed = True
      else:
        child.wait()
    ended = time.time()
    worked += ended - (woke or spawned)
    if Path(f"{record}.log").is_file():
      shutil.copy(f"{record}.log", run / f"life{n}.log")
    line(run / "spawns.jsonl", life=n, spawned=spawned, woke=woke, ended=ended, delay=delay, killed=killed)
    say(f"[crash] life {n}: woke {round(woke - spawned, 2) if woke else None} s after its start, killed {killed}")
    if not killed:
      break
  after = server()
  spent = {name: after[name] - before[name] for name in after if name in before}
  patch = run / "model.patch"
  deepswe.frozen(work / "app", syn, patch)
  reward = deepswe.graded(task, work / "base", syn, patch, run / "reward.json")
  line(run / "run.jsonl", seed=seed, server=spent, answers=len(answered(kept(record))), reward=reward)
  return 0


def logged(path: Path) -> tuple[int, list[list]]:
  """The entries that the record held when a life opened, and each fact of the log of that life."""
  if not path.is_file():
    return 0, []
  said = [json.loads(one) for one in path.read_text(encoding="utf-8").splitlines() if one.strip()]
  head = said[0] if said and isinstance(said[0], dict) else {}
  return int(head.get("entries", 0)), [one for one in said if isinstance(one, list) and one and one[0] != "keep"]


class Measure(TypedDict):
  """What one run came to."""

  ended: bool
  stopped: str
  value: object
  kills: int
  first_wake: float | None
  resume_seconds: list[float]
  commands: int
  interrupted: list[str]
  ran_again_after_a_kill: list[dict[str, object]]
  one_act_ran_in_two_lives: list[str]
  replies_asked_again: list[str]
  calls: float | None
  answers: int | None
  extra_calls: float | None
  reward: object
  f2p: str


def number(name: str) -> int:
  """The number of an act in its name."""
  return int("".join(ch for ch in name if ch.isdigit()) or 0)


def measured(run: Path) -> Measure:
  """What one run came to: its end, the commands that ran again after a kill, the extra calls to the model, and the
  seconds to the wake of each life."""
  record = [one for (one, *_) in kept(run / "record.jsonl")] if (run / "record.jsonl").is_file() else []
  lives, spawns, ran = lines(run / "lives.jsonl"), lines(run / "spawns.jsonl"), lines(run / "run.jsonl")
  ends = [one for one in lives if "end" in one]
  commands = {str(x[1]): str(x[4]) for x in record if x[0] == "bash"}
  interrupted = list(
    dict.fromkeys(
      str(x[1])
      for x in record
      if x[0] in ("close", "done")
      and str(x[1]) in commands
      and isinstance(x[3], BaseException)
      and "was interrupted" in str(x[3])
    )
  )
  lived_in: dict[str, list[int]] = {}
  for one in spawns:
    entries, said = logged(run / f"life{one['life']}.log")
    before = {str(x[1]) for x in record[:entries] if x[0] == "started"}
    for x in said:
      if x[0] == "started" and str(x[1]) in commands and str(x[1]) not in before:
        lived_in.setdefault(str(x[1]), []).append(one["life"])
  again: list[dict[str, object]] = [
    {"command": commands[cut], "interrupted": cut, "again": later}
    for cut in interrupted
    if (later := [b for b in commands if number(b) > number(cut) and commands[b] == commands[cut]])
  ]
  reasked = [p for one in lives if one.get("life", 0) > 0 for p in one.get("pending", []) if p.startswith("reply")]
  done = ran[-1] if ran else {}
  calls, answers = done.get("server", {}).get("requests"), done.get("answers")
  graded = done.get("reward", {})
  ready = {one["life"]: one["ready"] for one in lives if "ready" in one}
  resume = [round(ready[one["life"]] - one["spawned"], 2) for one in spawns if one["life"] > 0 and one["life"] in ready]
  return {
    "ended": bool(ends) and not ends[-1]["stopped"],
    "stopped": ends[-1]["stopped"] if ends else "no life ended",
    "value": ends[-1]["value"] if ends else None,
    "kills": sum(1 for one in spawns if one["killed"]),
    "first_wake": round(ready[0] - spawns[0]["spawned"], 2) if spawns and 0 in ready else None,
    "resume_seconds": resume,
    "commands": len(commands),
    "interrupted": [commands[one] for one in interrupted],
    "ran_again_after_a_kill": again,
    "one_act_ran_in_two_lives": [one for one, at in lived_in.items() if len(at) > 1],
    "replies_asked_again": reasked,
    "calls": calls,
    "answers": answers,
    "extra_calls": None if calls is None or answers is None else calls - answers,
    "reward": graded.get("reward"),
    "f2p": f"{graded.get('f2p_passed')}/{graded.get('f2p_total')}",
  }


def report(args: argparse.Namespace) -> int:
  """The measures of each run under the directory out, and their sums."""
  found: dict[str, Measure] = {
    run.name: measured(run) for run in sorted(args.out.iterdir()) if (run / "spawns.jsonl").is_file()
  }
  runs = list(found.values())
  waits = [s for one in runs for s in one["resume_seconds"]]
  sums: dict[str, object] = {
    "tasks": len(runs),
    "ended": sum(1 for one in runs if one["ended"]),
    "kills": sum(one["kills"] for one in runs),
    "commands_ran_again": sum(len(one["ran_again_after_a_kill"]) for one in runs),
    "acts_ran_in_two_lives": sum(len(one["one_act_ran_in_two_lives"]) for one in runs),
    "replies_asked_again": sum(len(one["replies_asked_again"]) for one in runs),
    "extra_calls": sum(one["extra_calls"] or 0 for one in runs),
    "resume_seconds_mean": round(sum(waits) / len(waits), 2) if waits else None,
    "resume_seconds_max": max(waits, default=None),
  }
  said = json.dumps({"runs": found, "all": sums}, indent=2, default=repr) + "\n"
  (args.out / "report.json").write_text(said, encoding="utf-8")
  sys.stdout.write(said)
  return 0


def main() -> None:
  """The door of the operator onto the bench."""
  deepswe.outside()
  whole = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  verbs = whole.add_subparsers(dest="verb", required=True)
  doing = verbs.add_parser("run", help=crash.__doc__)
  doing.add_argument("task", help="The task of DeepSWE, as the collection names it.")
  doing.add_argument("--kills", type=int, default=KILLS, help="How many times the run is killed.")
  doing.add_argument("--cap", type=float, default=CAP, help="The seconds that the lives may work, summed.")
  doing.add_argument("--seed", type=int, default=None, help="The seed of the points of the kills.")
  doing.set_defaults(go=crash)
  living = verbs.add_parser("life", help=life.__doc__)
  living.add_argument("task", help="The task of DeepSWE, as the collection names it.")
  living.add_argument("--life", type=int, required=True, help="The number of the life in its run, from 0.")
  living.add_argument("--cap", type=float, required=True, help="The seconds that this life may work.")
  living.set_defaults(go=lambda args: asyncio.run(life(args)))
  reading = verbs.add_parser("report", help=report.__doc__)
  reading.set_defaults(go=report)
  for one in (doing, living, reading):
    one.add_argument("--out", type=Path, required=True, help="The directory of the records and of the results.")
  args = whole.parse_args()
  raise SystemExit(args.go(args))


if __name__ == "__main__":
  main()
