"""The proof of one life: a life goes on over three builds, the last of a new version, reloads an extension and
compacts a chain.

It builds the command line three times from this checkout: as it stands, then with the engine changed twice in a way
that makes the same acts, and the last build with the version of src/world/store.rs one higher. Then three lives run
on one record, one on each build, on a claude that answers from a script: the first makes a chain, runs a command,
asks a model and makes the extension kit; the second makes kit again with a new word, which reloads it, and compacts
the chain in place; the third finds a record of the old version, so the build of that version, from the store of
builds, writes a checkpoint, and the new build goes on from it, reads what the two left and asks the model again. It
checks that no life drifts, that no chain is lost, that no name is taken twice, and that no command and no turn of a
model runs twice, and prints the measures.

  uv run python script/bench/onelife.py [--out <directory>] [--builds <directory>]

--builds names a directory that holds furb-a, furb-b and furb-c from an earlier run, so it builds nothing. Each build
takes a minute or two after the first. The script puts src/furb/engine.py and src/world/store.rs back as they were,
even when a build fails.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENGINE = ROOT / "src" / "furb" / "engine.py"
FAKE = ROOT / "src" / "world" / "provider" / "fake-claude.sh"
STORE = ROOT / "src" / "world" / "store.rs"

# Two changes of the engine that make the same acts: each gives a build of another engine and of the same version.
CHANGES = [
  ("def idle(id):\n  while True:\n", "def idle(id):\n  while 1:\n"),
  ("def over(id):\n  return peek(id, ...) is not ...", "def over(id):\n  return not peek(id, ...) is ..."),
]

# The builds, in the order that the lives run on them.
BUILDS = "abc"

# The seconds that one life takes at most: each takes a few seconds, so a life that waits longer hangs.
LIFE = 180

FIRST = """work = chain("work")
await bash("echo one >> ran.txt")
said = await prompt(str, "Say hi.", on=work)
kit = rung("LIMIT = 1\\n\\ndef f() -> int:\\n  return LIMIT")
await kit
extend("kit", kit)
close([said, globals()["f"]()])"""

SECOND = """kit = rung("LIMIT = 2\\n\\ndef f() -> int:\\n  return LIMIT * 10")
await kit
extend("kit", kit)
await bash("echo two >> ran.txt")
await rung("noted = 'before the compaction'", on="chain2")
await rung("compact([recent(0)], summary='The work said hi.')", on="chain2")
close(await rung("close([globals()[name]() if name == 'f' else globals()[name] for name in ('f', 'noted')])", on="chain2"))"""

THIRD = """wake("chain2")
await bash("echo three >> ran.txt")
said = await prompt(str, "Say hi again.", on="chain2")
close([said, await rung("close([globals()[name]() if name == 'f' else globals()[name] for name in ('f', 'noted')])", on="chain2")])"""


def say(line: str = "") -> None:
  """A line of what the proof measured, on stdout."""
  sys.stdout.write(line + "\n")
  sys.stdout.flush()


def build(out: Path) -> None:
  """The three builds of the command line, as furb-a, furb-b and furb-c in out: the last one of the next version."""
  cargo = shutil.which("cargo") or "cargo"
  text, kept = ENGINE.read_text(encoding="utf-8"), STORE.read_text(encoding="utf-8")
  found = re.search(r"^pub const VERSION: i64 = (\d+);$", kept, re.MULTILINE)
  assert found, f"{STORE} names no VERSION"
  version = int(found[1])
  try:
    now = text
    for name, change in zip(BUILDS, [None, *CHANGES], strict=True):
      if change:
        assert change[0] in now, f"the engine holds no {change[0]!r}"
        now = now.replace(*change)
        ENGINE.write_text(now, encoding="utf-8")
      if name == BUILDS[-1]:
        STORE.write_text(kept.replace(f"VERSION: i64 = {version};", f"VERSION: i64 = {version + 1};"), encoding="utf-8")
      subprocess.run([cargo, "build", "-q", "-p", "furb-cli"], cwd=ROOT, check=True)  # noqa: S603
      shutil.copy2(ROOT / "target" / "debug" / "furb", out / f"furb-{name}")
      stood = version + (name == BUILDS[-1])
      say(f"build {name}: engine.py {hashlib.sha256(now.encode()).hexdigest()[:12]}, version {stood}")
  finally:
    ENGINE.write_text(text, encoding="utf-8")
    STORE.write_text(kept, encoding="utf-8")


def live(builds: Path, out: Path) -> tuple[list[tuple[str, str, float, int]], Path, Path]:
  """Each life on its build, on one record: its build, what it said, its seconds and the entries of the record after
  it, with the folder of the project and the folder of the fake claude."""
  project, fake = out / "project", out / "fake"
  for folder in (project, fake):
    shutil.rmtree(folder, ignore_errors=True)
    folder.mkdir()
  shutil.copy2(FAKE, fake / "claude")
  (fake / "claude").chmod(0o755)
  for at, word in enumerate(["close('hi')", "close(None)", "close('hi again')"], 1):
    (fake / f"word.{at}").write_text(json.dumps(word), encoding="utf-8")
  record = project / "record.jsonl"
  env: dict[str, str] = {
    **os.environ,
    "FURB_CLAUDE_BIN": str(fake / "claude"),
    "FURB_CONFIG_DIR": str(out / "config"),
    "XDG_CACHE_HOME": str(out / "cache"),
  }
  for name in ("FURB_TUI", "FURB_LOCAL_URL", "MAX_THINKING_TOKENS"):
    env.pop(name, None)
  rows = []
  for name, word in zip(BUILDS, [FIRST, SECOND, THIRD], strict=True):
    started = time.monotonic()
    got = subprocess.run(  # noqa: S603
      [str(builds / f"furb-{name}"), "run", word, "--record", str(record), "--cwd", str(project)],
      env=env,
      capture_output=True,
      text=True,
      timeout=LIFE,
      check=False,
    )
    took = time.monotonic() - started
    if got.returncode != 0:
      sys.exit(f"the life on build {name} failed with {got.returncode}: {got.stdout}{got.stderr}")
    rows.append((name, got.stdout.strip(), took, len(record.read_text(encoding="utf-8").splitlines())))
    # A life that goes on from a checkpoint writes the log again from the checkpoint, so each log is kept as it ends.
    shutil.copy2(record.with_name(record.name + ".log"), out / f"log-{name}.jsonl")
  return rows, project, fake


def measure(rows: list[tuple[str, str, float, int]], project: Path, fake: Path) -> dict[str, bool]:
  """What the lives left, as the log of each life holds it, said, and each check of the proof."""
  logs = []
  for name in BUILDS:
    lines = (project.parent / f"log-{name}.jsonl").read_text(encoding="utf-8").splitlines()
    logs.append([fact for fact in map(json.loads, lines) if isinstance(fact, list)])
  acts: dict[str, list] = {}
  again = set()
  for facts in logs:
    for one in facts:
      if re.fullmatch(re.escape(one[0]) + r"\d+", one[1]):
        # A checkpoint says a carried act again as the operator's, so only what it is and its words count.
        if one[1] in acts and acts[one[1]][3:] != one[3:] and one[3] != "" and one[0] != "run":
          again.add(one[1])
        acts.setdefault(one[1], one)
  twice = sorted(
    {
      one[1]
      for facts in logs
      for one in facts
      if one[0] == "done" and [x[1] for x in facts if x[0] == "done"].count(one[1]) > 1
    }
  )
  chains = [name for name, one in acts.items() if one[0] == "chain"]
  # A reply that a life left with no answer is cut, and its prompt asks again under a new name.
  answered = {one[1] for facts in logs for one in facts if one[0] == "done"}
  replies = [name for name, one in acts.items() if one[0] == "reply" and name in answered]
  ran = (project / "ran.txt").read_text(encoding="utf-8").split()
  turns = int((fake / "count").read_text(encoding="utf-8").strip() or 0)
  enables = [one[3] for one in logs[1] if one[0] == "enable"]
  prefixes = sorted({one[1] for facts in logs for one in facts if one[0] == "prefix"})
  kept = [json.loads(line)[0] for line in (project / "record.jsonl").read_text(encoding="utf-8").splitlines()]
  cut = max((at for at, one in enumerate(kept) if one[0] == "checkpoint"), default=-1)
  stood = [one[4] for one in kept[cut + 1 :] if one[0] == "made"] if cut >= 0 else []
  say()
  say("life  build  said                                  seconds  entries")
  for name, said, took, count in rows:
    say(f"{BUILDS.index(name) + 1:<5} {name:<6} {said[:36]:<37} {took:7.2f}  {count}")
  say()
  say(f"chains: {chains}")
  say(f"commands run: {ran}")
  say(f"turns of the model: {turns}, replies answered: {len(replies)}")
  say(f"acts with two dones: {twice}")
  say(f"names made again for another act: {sorted(again)}")
  say(f"extensions enabled by life 2, in order: {enables}")
  say(f"versions named after the checkpoint at entry {cut + 1}: {stood}")
  say(f"chains that a prefix superseded: {prefixes}")
  return {
    "no drift": len(rows) == len(BUILDS),
    "no command ran twice": ran == ["one", "two", "three"],
    "no turn of a model ran twice": turns == len(replies),
    "no act was answered twice": not twice,
    "no name of the past was taken again": not again,
    "the last life crossed a version through a checkpoint": len(stood) == 1 and stood[0] == kept[1][4] + 1,
    "no chain lost, and none made in its place": chains == ["chain1", "chain2"],
    "the extension reloaded": enables.count("kit") == len(("made", "made again"))
    and rows[2][1].startswith("['hi again', [20"),
    "the chain compacted in place": "chain2" in prefixes,
    "the state of the chain stayed": "before the compaction" in rows[2][1],
  }


def main() -> None:
  parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  parser.add_argument("--out", type=Path)
  parser.add_argument("--builds", type=Path)
  args = parser.parse_args()
  out = args.out or Path(tempfile.mkdtemp(prefix="furb-onelife-"))
  out.mkdir(parents=True, exist_ok=True)
  if not args.builds:
    build(out)
  checks = measure(*live(args.builds or out, out))
  say()
  for check, held in checks.items():
    say(f"{'ok  ' if held else 'FAIL'} {check}")
  if not all(checks.values()):
    sys.exit(1)


if __name__ == "__main__":
  main()
