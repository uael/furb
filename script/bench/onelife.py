"""The proof of one life: a life goes on over three builds of one version, reloads an extension and compacts a chain.

It builds the command line three times from this checkout: as it stands, then with the engine changed twice in a way
that makes the same acts, so each build runs another engine of the same version. Then three lives run on one record,
one on each build, on a claude that answers from a script: the first makes a chain, runs a command, asks a model and
makes the extension kit; the second makes kit again with a new word, which reloads it, and compacts the chain in
place; the third reads what the two left and asks the model again. It checks that no life drifts, that no chain is
lost, and that no command and no turn of a model runs twice, and prints the measures.

  uv run python script/bench/onelife.py [--out <directory>] [--builds <directory>]

--builds names a directory that holds furb-a, furb-b and furb-c from an earlier run, so it builds nothing. Each build
takes a minute or two after the first. The script puts src/furb/engine.py back as it was, even when a build fails.
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
  """The three builds of the command line, as furb-a, furb-b and furb-c in out."""
  cargo = shutil.which("cargo") or "cargo"
  text = ENGINE.read_text(encoding="utf-8")
  try:
    now = text
    for name, change in zip(BUILDS, [None, *CHANGES], strict=True):
      if change:
        assert change[0] in now, f"the engine holds no {change[0]!r}"
        now = now.replace(*change)
        ENGINE.write_text(now, encoding="utf-8")
      subprocess.run([cargo, "build", "-q", "-p", "furb-cli"], cwd=ROOT, check=True)  # noqa: S603
      shutil.copy2(ROOT / "target" / "debug" / "furb", out / f"furb-{name}")
      say(f"build {name}: engine.py {hashlib.sha256(now.encode()).hexdigest()[:12]}")
  finally:
    ENGINE.write_text(text, encoding="utf-8")


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
  return rows, project, fake


def measure(rows: list[tuple[str, str, float, int]], project: Path, fake: Path) -> dict[str, bool]:
  """What the lives left, as the log of the record holds it, said, and each check of the proof."""
  # The log beside the record holds every fact that the lives said, and the record only what a replay needs.
  logged = (project / "record.jsonl.log").read_text(encoding="utf-8").splitlines()
  facts = [fact for fact in map(json.loads, logged) if isinstance(fact, list)]
  questions = [one for one in facts if re.fullmatch(re.escape(one[0]) + r"\d+", one[1])]
  names = [one[1] for one in questions]
  dones = [one[1] for one in facts if one[0] == "done"]
  twice = sorted({one for one in dones if dones.count(one) > 1})
  chains = [one[1] for one in questions if one[0] == "chain"]
  replies = [one[1] for one in questions if one[0] == "reply"]
  ran = (project / "ran.txt").read_text(encoding="utf-8").split()
  turns = int((fake / "count").read_text(encoding="utf-8").strip() or 0)
  enables = [one[3] for one in facts if one[0] == "enable"]
  prefixes = [one[1] for one in facts if one[0] == "prefix"]
  say()
  say("life  build  said                                  seconds  entries")
  for name, said, took, count in rows:
    say(f"{BUILDS.index(name) + 1:<5} {name:<6} {said[:36]:<37} {took:7.2f}  {count}")
  say()
  say(f"chains: {chains}")
  say(f"commands run: {ran}")
  say(f"turns of the model: {turns}, replies: {len(replies)}")
  say(f"acts with two dones: {twice}")
  say(f"names made twice: {sorted({one for one in names if names.count(one) > 1})}")
  say(f"extensions enabled, in order: {enables}")
  say(f"chains that a prefix superseded: {prefixes}")
  return {
    "no drift": len(rows) == len(BUILDS),
    "no command ran twice": ran == ["one", "two", "three"],
    "no turn of a model ran twice": turns == len(replies),
    "no act was answered twice": not twice,
    "no chain lost, and none made in its place": chains == ["chain1", "chain2"],
    "the extension reloaded": enables.count("kit") == len(("made", "made again")) and rows[1][1].startswith("[20"),
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
