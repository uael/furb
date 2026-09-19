"""furb against a DeepSWE task: seed a checkout, let one life work in it, grade it with the task's own verifier.

  uv run python script/deepswe.py tools
  uv run python script/deepswe.py validate <task>
  uv run python script/deepswe.py run <task> --to opus/low --ceiling 8
  uv run python script/deepswe.py turns <task>

A model is told the task and the shape of its answer, and nothing else. Its system prompt is the engine, minified,
which is every word it is given about what it is and what it may do; the message of the prompt is the instruction
of the task, as the task wrote it, and one line that asks it to close with how sure it is that the task is
complete. Under the confidence the rig wants, the operator says so and asks it to go on, which is the one other
word the model ever reads. No rule of this rig reaches the model.

A task is fetched as a codeload tarball at its base commit and stood up as a one-commit git repo, so what is
graded is the upstream tree byte for byte, with no clone. The checkout the life works in is a copy of that base,
and the grade always runs on a fresh copy with the submission applied, never on the tree the life left behind.

`validate` grades the task's own reference solution and is the control: a rig that scores that solution under 1
is measuring itself and not the life. Everything stands under $DEEPSWE_WORK/<task>, which is /tmp/furb-deepswe by
default: `base` is the pristine checkout with the dependencies of the task, `app` is the checkout the life works
in, and `.run` holds the record, the numbers, the frozen submission and the reward.
"""

import argparse
import asyncio
import base64
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
import tomllib
from collections.abc import Mapping, Sequence
from datetime import UTC, datetime
from pathlib import Path
from tempfile import gettempdir, mkdtemp

from furb import engine
from furb.cli import lived, say
from furb.provider.claude import BIN, cool
from furb.world import Live, kept, rendered

ROOT = Path(__file__).resolve().parent.parent
"""ROOT is the root of this repository, which the archive of a run stands under."""
TRACES = ROOT / "traces"
"""TRACES is where the record and the result of every graded run are kept."""
COLLECTION = "https://github.com/datacurve-ai/deep-swe"
"""COLLECTION is the repository of tasks, fetched once and kept."""
CACHE = Path(os.environ.get("DEEPSWE_DIR") or Path(gettempdir()) / "deep-swe")
"""CACHE is where the collection of tasks is kept."""
WORK = Path(os.environ.get("DEEPSWE_WORK") or Path(gettempdir()) / "furb-deepswe")
"""WORK is where the checkout of every task stands."""
HOMES = Path.home() / ".cache/furb-deepswe"
"""HOMES is where a reporter lives when the path its frame names is not this host's to write."""
REPORTERS = {
  "/opt/ctrf": ("mocha-ctrf-json-reporter@0.0.11",),
  "/opt/jest-ctrf": ("jest-ctrf-json-reporter@0.0.11", "jest-environment-node@29.7.0"),
  "/opt/nextest": (),
}
"""REPORTERS is every reporter a task's verifier opens by an absolute path, with what to install there."""
NEXTEST = '[profile.junit]\nfail-fast = false\n\n[profile.junit.junit]\npath = "junit.xml"\n'
"""NEXTEST is the profile a rust task's verifier reads its results by."""
KEEP = ("node_modules", ".venv", "target", "vendor")
"""KEEP is what a clean of the checkout leaves standing, since installing it again costs minutes."""
ROOTS = re.compile(r"(?<![\w./}$])(/opt/jest-ctrf|/opt/ctrf|/opt/nextest|/app|/tests|/logs)\b")
"""ROOTS is every absolute root a frame of a task opens, rewritten in one pass so no swap lands inside another.

The lookbehind carries the rule: a relative `./tests/x.js` of the frame, and any `<dir>/tests` it makes, must
stand untouched, or the suite looks for its modules under the rewritten root and finds none.
"""
VCS = {
  "SETUPTOOLS_SCM_PRETEND_VERSION": "0.0.0",
  "HATCH_VCS_PRETEND_VERSION": "0.0.0",
  "PDM_BUILD_SCM_VERSION": "0.0.0",
}
"""VCS is what a build backend that reads a version off git is told instead, since the base holds one commit."""
CONTAINER = ("apt-get", "apt", "apk", "yum", "dnf")
"""CONTAINER is every step of a Dockerfile that only a container can take, which this rig steps over."""
PIP = (("python3 -m pip ", "pip "), ("python -m pip ", "pip "), ("pip3 ", "pip "), ("pip ", "python3 -m pip "))
"""PIP is how a step that installs is said on this host, in the order the swaps apply."""
TICK = 2.0
"""TICK is the seconds between two looks at a life that is working."""
STEP = 1800.0
"""STEP is the seconds one step of the rig may take, since a step that waits for ever holds the whole of it."""
ASKED = "\n\nWhen the work is done, close with a float from 0 to 1: how sure you are that this task is complete."
"""ASKED is what the rig adds to the instruction of the task, which says the shape of the answer and no more."""
AGAIN = "confidence level {} is too low, continue"
"""AGAIN is what the operator says to a model that closed under the confidence it wants."""
CONFIDENT = 0.9
"""CONFIDENT is the confidence the rig takes for done, under which it asks the model to go on."""
LOOK = 10
"""LOOK is how many looks stand between two readings of the record, since each reading folds the whole of it."""
STUCK = 25
"""STUCK is how many answers a life may buy without doing one thing before the rig reads it as wedged.

A model that answers with a value and no call leaves its prompt open, so the chain asks again, and the answer it
already gave stands in the turns and is answered the same way. One run bought 412 of those and six dollars, and
the ceiling of the grant was the only thing that ended it, since a grant bounds the money and not the work.
"""


def ran(args: Sequence[str], where: Path | None = None, env: Mapping[str, str] | None = None) -> int:
  """One step whose voice is the operator's, and whose exit code is the answer.

  Every step is bounded: an install that waits on a prompt it will never be given, or a fetch that stalls, would
  otherwise hold the rig for as long as it lives, and one such step held a slot of this machine for an hour.
  """
  try:
    got = subprocess.run(  # noqa: S603
      list(args), cwd=where, env=None if env is None else {**os.environ, **env}, check=False, timeout=STEP
    )
  except subprocess.TimeoutExpired:
    say(f"[deepswe] the step gave nothing in {STEP:.0f} seconds and was ended: {' '.join(args)[:80]}")
    return 1
  return got.returncode


def spoke(args: Sequence[str], where: Path | None = None) -> str:
  """One short step whose answer is the point, and nothing at all when the step failed."""
  got = subprocess.run(list(args), cwd=where, capture_output=True, text=True, check=False)  # noqa: S603
  return got.stdout.strip() if got.returncode == 0 else ""


def quiet(args: Sequence[str]) -> int:
  """A probe of what this host can do, whose answer stands out of the log of the operator."""
  return subprocess.run(list(args), capture_output=True, check=False).returncode  # noqa: S603


def bytes_of(args: Sequence[str]) -> bytes:
  """One short step whose answer is bytes, kept exactly as the tool wrote them."""
  return subprocess.run(list(args), capture_output=True, check=False).stdout  # noqa: S603


def git(*args: str, where: Path) -> int:
  """Git, always told which tree it speaks about."""
  return ran(["git", "-C", str(where), *args])


def tool(name: str) -> str:
  """A program this rig cannot work without, or a refusal that names the one that is missing."""
  got = shutil.which(name)
  if got is None:
    say(f"[deepswe] no {name} on PATH; the rig cannot run without it")
    raise SystemExit(1)
  return got


def extracted(url: str, ref: str, into: Path) -> None:
  """A tree of github at one ref, with no clone: the codeload tarball, unwrapped where it stands."""
  slug = url.removeprefix("https://github.com/").removesuffix(".git")
  say(f"[deepswe] codeload {slug} at {ref[:12]} into {into}")
  shutil.rmtree(into, ignore_errors=True)
  into.mkdir(parents=True)
  tgz = into.parent / f".{into.name}.tar.gz"
  fetch = ["curl", "-fsSL", "--retry", "6", "--retry-delay", "3", "--retry-all-errors", "--retry-max-time", "240"]
  if ran([*fetch, f"https://codeload.github.com/{slug}/tar.gz/{ref}", "-o", str(tgz)]):
    say(f"[deepswe] codeload failed for {slug} at {ref}")
    raise SystemExit(1)
  # codeload wraps the tree in one directory named for the repo and the ref, which the strip takes off.
  if ran(["tar", "xzf", str(tgz), "-C", str(into), "--strip-components=1"]):
    say(f"[deepswe] could not unpack {tgz}")
    raise SystemExit(1)
  tgz.unlink()


def based(url: str, sha: str, into: Path) -> None:
  """The repo at one commit as a repo of one commit, so a submission is a diff against a base that is real."""
  extracted(url, sha, into)
  git("init", "-q", where=into)
  git("config", "user.email", "deepswe@local", where=into)
  git("config", "user.name", "deepswe", where=into)
  git("add", "-A", where=into)
  git("commit", "-q", "-m", f"deepswe base: {url} at {sha}", "--no-verify", where=into)


def collected() -> Path:
  """The collection of tasks, fetched once and kept.

  What says it is there is a task that holds its own toml: the reaper of this host takes the files of an old
  cache and leaves its directories, and a tree of empty directories answers every question with nothing.
  """
  if not next((CACHE / "tasks").glob("*/task.toml"), None):
    extracted(COLLECTION, "refs/heads/main", CACHE)
  return CACHE


def home(where: str) -> Path:
  """Where a reporter the frame of a task opens by an absolute path stands on this host."""
  named = Path(where)
  return named if os.access(named.parent, os.W_OK) else HOMES / named.name


def tools(args: argparse.Namespace) -> int:
  """Install the reporters the verifier of a task wants, which its image bakes in and a host does not hold."""
  del args
  npm = tool("npm")
  if not shutil.which("junit-to-ctrf"):
    say("[deepswe] npm -g junit-to-ctrf@0.0.14")
    ran([npm, "install", "-g", "junit-to-ctrf@0.0.14", "--no-audit", "--no-fund"])
  for where, packages in REPORTERS.items():
    into = home(where)
    into.mkdir(parents=True, exist_ok=True)
    if packages and not (into / "node_modules").is_dir():
      say(f"[deepswe] npm --prefix {into} {' '.join(packages)}")
      ran([npm, "install", "--prefix", str(into), *packages, "--no-audit", "--no-fund"])
  (home("/opt/nextest") / "nextest.toml").write_text(NEXTEST, encoding="utf-8")
  for name in ("go-ctrf-json-reporter", "cargo-nextest"):
    if not shutil.which(name):
      say(f"[deepswe] no {name} on PATH: a go or rust task cannot be graded until it is installed")
  say(f"[deepswe] reporters under {HOMES}")
  return 0


def read_task(task: str) -> tuple[Path, Mapping[str, str]]:
  """The directory of one task and what it says of the repo and the commit it stands on."""
  where = collected() / "tasks" / task
  if not where.is_dir():
    say(f"[deepswe] no task {task!r}; try: ls {CACHE / 'tasks'}")
    raise SystemExit(1)
  with (where / "task.toml").open("rb") as fh:
    told = tomllib.load(fh)
  meta = told.get("metadata", {})
  for key in ("repository_url", "base_commit_hash", "language"):
    if not meta.get(key):
      say(f"[deepswe] {task}: task.toml names no {key}")
      raise SystemExit(1)
  return where, meta


def steps(task: Path) -> list[str]:
  """The steps of dependency the Dockerfile of a task takes, without the clone the tarball already stands for."""
  df = task / "environment" / "Dockerfile"
  if not df.is_file():
    return []
  text = df.read_text(encoding="utf-8", errors="replace").replace("\\\n", " ")
  return [
    cmd
    for line in text.split("\n")
    if line.strip().startswith("RUN ") and "git clone" not in (cmd := line.strip()[4:].strip())
  ]


def venved(repo: Path) -> str:
  """What puts the interpreter of the checkout first, once one stands there.

  A python task gets an interpreter of its own inside the checkout, made from this one, so its dependencies land
  neither in the python of the system nor in the one of this repository.
  """
  first = repo / ".venv" / "bin"
  return f'export PATH="{first}:$PATH"; ' if first.is_dir() else ""


def generic(repo: Path, lang: str) -> None:
  """What is installed for a task whose Dockerfile says nothing this host can take."""
  words = {
    "typescript": "npm ci --include=dev --no-audit --no-fund || npm install --no-audit --no-fund",
    "javascript": "npm ci --include=dev --no-audit --no-fund || npm install --no-audit --no-fund",
    "python": "python3 -m pip install -q -e '.[dev,test]' || python3 -m pip install -q -e .",
    "go": "go mod download",
    "rust": "cargo fetch",
  }.get(lang)
  if words is None:
    say(f"[deepswe] unknown language {lang!r}; nothing installed")
    return
  ran(["bash", "-lc", venved(repo) + words], where=repo, env={**VCS, "npm_config_confirm_modules_purge": "false"})


def installed(task: Path, repo: Path, lang: str) -> None:
  """Take the steps of the task's own Dockerfile in the checkout, so the tree is the one its verifier wants."""
  if lang == "python" and not (repo / ".venv").is_dir():
    say("[deepswe] making the interpreter of the checkout")
    ran([sys.executable, "-m", "venv", str(repo / ".venv")])
  told = steps(task)
  if not told:
    say(f"[deepswe] {task.name}: the Dockerfile says no step; installing by language instead")
    return generic(repo, lang)
  for step in told:
    if step.split(" ", 1)[0] in CONTAINER:
      say(f"[deepswe] over (a container alone takes it): {step[:80]}")
      continue
    named = step
    for was, now in PIP:
      named = named.replace(was, now)
    cmd = re.sub(r"(?<![\w./])/app\b", str(repo), named)
    say(f"[deepswe] RUN {cmd[:90]}")
    if ran(["bash", "-lc", venved(repo) + cmd], where=repo, env={**VCS, "npm_config_confirm_modules_purge": "false"}):
      say("[deepswe]   the step failed; the rig goes on with what did land")
  if lang in ("typescript", "javascript") and not (repo / "node_modules").is_dir():
    generic(repo, lang)
  return None


def submodules(slug: str, base: str, into: Path) -> None:
  """Pin the submodules a tarball cannot carry, without which a suite that wants one cannot even be collected."""
  if not shutil.which("gh"):
    return
  told = spoke(["gh", "api", f"repos/{slug}/contents/.gitmodules?ref={base}", "--jq", ".content"])
  if not told:
    return
  path = ""
  for line in base64.b64decode(told).decode(errors="replace").splitlines():
    if "path = " in line:
      path = line.split("path = ", 1)[1].strip()
    elif "url = " in line and path:
      url = line.split("url = ", 1)[1].strip()
      if not (into / path / ".git").exists():
        sha = spoke(
          ["gh", "api", f"repos/{slug}/contents/{path}?ref={base}", "--jq", 'select(.type=="submodule") | .sha']
        )
        if sha:
          say(f"[deepswe] submodule {path} from {url} at {sha[:10]}")
          if ran(["git", "clone", "-q", url, str(into / path)]) or git("checkout", "-q", sha, where=into / path):
            say(f"[deepswe]   could not pin {path}; a suite that wants it cannot be collected")
            shutil.rmtree(into / path, ignore_errors=True)
      path = ""


def seeded(task: str, *, keep_app: bool = False) -> tuple[Path, Path, Mapping[str, str], str]:
  """Everything one run wants before a model is asked anything: the base, its dependencies, and the checkout."""
  where, meta = read_task(task)
  work = WORK / task
  base, app = work / "base", work / "app"
  work.mkdir(parents=True, exist_ok=True)
  if not (base / ".git").is_dir():
    based(meta["repository_url"], meta["base_commit_hash"], base)
    say(f"[deepswe] installing dependencies ({meta['language']}, from the Dockerfile of the task)")
    installed(where, base, meta["language"])
  submodules(
    meta["repository_url"].removeprefix("https://github.com/").removesuffix(".git"), meta["base_commit_hash"], base
  )
  syn = spoke(["git", "-C", str(base), "rev-parse", "HEAD"])
  say(f"[deepswe] base {syn[:10]}, whose tree is upstream {meta['base_commit_hash'][:10]}")
  if keep_app and (app / ".git").is_dir():
    say(
      f"[deepswe] keeping the checkout as it stands ({spoke(['git', '-C', str(app), 'rev-parse', '--short', 'HEAD'])})"
    )
  else:
    say("[deepswe] seeding the checkout from the base")
    shutil.rmtree(app, ignore_errors=True)
    ran(["cp", "-a", str(base), str(app)])
    git("reset", "-q", "--hard", syn, where=app)
    git("clean", "-qfd", *(a for k in KEEP for a in ("-e", k)), where=app)
  return work, where, meta, syn


def frozen(app: Path, syn: str, into: Path) -> None:
  """The submission, taken the moment the tree is final: everything the life left, as one diff against the base."""
  if spoke(["git", "-C", str(app), "status", "--porcelain"]):
    git("add", "-A", where=app)
    named = ["-c", "user.email=deepswe@local", "-c", "user.name=deepswe"]
    git(*named, "commit", "-q", "-m", "submission", "--no-verify", where=app)
  said = bytes_of(["git", "-C", str(app), "diff", "--binary", syn, "HEAD"])
  into.write_bytes(said)
  say(f"[deepswe] submission frozen: {len(said)} bytes into {into}")


def mode() -> str:
  """How this host can run the verifier of a task: its own image, a mount namespace of its own, or plain paths."""
  told = os.environ.get("DEEPSWE_GRADE")
  if told:
    return told
  if shutil.which("unshare") and quiet(["unshare", "-m", "true"]) == 0:
    return "unshare"
  return "docker" if shutil.which("docker") and quiet(["docker", "info"]) == 0 else "local"


def pristine(base: Path, syn: str, into: Path) -> None:
  """A fresh copy of the base for the grade: the patch lands on the base and never on the tree of a run."""
  ran(["cp", "-a", str(base), str(into)])
  git("reset", "-q", "--hard", syn, where=into)
  git("clean", "-qfd", *(a for k in KEEP for a in ("-e", k)), where=into)


def rewritten(tests: Path, app: Path, logs: Path, syn: str) -> None:
  """The frame of the task, aimed at the roots of this grade: the same steps, where this host can take them.

  test.sh and grader.py open absolute paths, so those are rewritten in this copy of them. config.json is data and
  no script: the names of its nodes are names of tests that may hold "/app" as plain text, so only the paths of
  the reports and the base commit move.
  """
  swaps = {"/app": str(app), "/tests": str(tests), "/logs": str(logs)}
  swaps |= {where: str(home(where)) for where in REPORTERS}
  for name in ("test.sh", "grader.py"):
    said = tests / name
    said.write_text(ROOTS.sub(lambda m: swaps[m[1]], said.read_text(encoding="utf-8")), encoding="utf-8")
  told = json.loads((tests / "config.json").read_text(encoding="utf-8"))
  told["base_commit"] = syn
  reports = told.get("grade", {}).get("reports")
  if isinstance(reports, list):
    told["grade"]["reports"] = [str(logs) + r[len("/logs") :] if r.startswith("/logs") else r for r in reports]
  (tests / "config.json").write_text(json.dumps(told, indent=1), encoding="utf-8")


def graded(task: Path, base: Path, syn: str, patch: Path, out: Path) -> Mapping[str, object]:
  """Run the held out verifier of the task over the submission, and keep the reward it writes."""
  how = mode()
  say(f"[deepswe] grading by {how}")
  vroot = Path(mkdtemp(prefix="furb-grade-")).resolve()
  logs = vroot / "logs"
  (logs / "verifier").mkdir(parents=True)
  (logs / "artifacts").mkdir(parents=True)
  shutil.copy(patch, logs / "artifacts" / "model.patch")
  tests = vroot / "tests"
  shutil.copytree(task / "tests", tests)
  if how == "docker":
    # The image the task pins is the pristine verifier: the tests and the logs alone are mounted into it.
    with (task / "task.toml").open("rb") as fh:
      image = tomllib.load(fh).get("environment", {}).get("docker_image", "")
    if not image:
      say(f"[deepswe] {task.name}: task.toml names no docker_image")
      raise SystemExit(1)
    ran(["docker", "run", "--rm", "-v", f"{tests}:/tests:ro", "-v", f"{logs}:/logs", image, "bash", "/tests/test.sh"])
  else:
    app = vroot / "app"
    pristine(base, syn, app)
    if (app / "pyproject.toml").is_file() or (app / "setup.py").is_file():
      # A package of src layout resolves through the newest editable hook, which points at the base: without this
      # the grade imports the pristine code and every test that must turn from fail to pass dies as it is collected.
      ran(["bash", "-lc", venved(app) + f"python3 -m pip install -e {shlex.quote(str(app))} --no-deps"], env=VCS)
    rewritten(tests, app, logs, syn)
    node = os.environ.get("DEEPSWE_NODE_HOME", "")
    first = [p for p in (str(app / ".venv" / "bin") if (app / ".venv").is_dir() else "", node) if p]
    env = {"npm_config_confirm_modules_purge": "false", "PATH": os.pathsep.join([*first, os.environ["PATH"]])}
    if how == "unshare":
      bind = (
        "mount --bind $1 /app && mount --bind $2 /tests && mount --bind $3 /logs && cd /app && exec bash /tests/test.sh"
      )
      ran(["unshare", "-m", "bash", "-c", bind, "_", str(app), str(tests), str(logs)], env=env)
    else:
      ran(["bash", str(tests / "test.sh")], where=app, env=env)
  said = logs / "verifier" / "reward.json"
  reward: Mapping[str, object] = json.loads(said.read_text(encoding="utf-8")) if said.is_file() else {"reward": None}
  out.write_text(json.dumps(reward, indent=2) + "\n", encoding="utf-8")
  for held in ("run.log", "reports"):
    if (logs / "verifier" / held).exists():
      shutil.move(str(logs / "verifier" / held), str(out.parent / held))
  shutil.rmtree(vroot, ignore_errors=True)
  say(f"[deepswe] reward {reward.get('reward')}: {json.dumps(reward)}")
  return reward


def read_json(path: Path) -> Mapping[str, object]:
  """What a file says, and nothing at all when no step ever wrote it."""
  return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}


def dollars(numbers: Mapping[str, object]) -> float:
  """What a run of the rig spent, as a number, however the record of it was read back."""
  got = numbers.get("usd")
  return float(got) if isinstance(got, (int, float)) else 0.0


def numbered(record: Path, root: str, began: float, got: object) -> Mapping[str, object]:
  """What the life did and what it cost, read off the record it kept."""
  held = [entry[1] for entry in kept(record)] if record.is_file() else []
  kinds = [one[0] for one in held]
  usage = [one[3][2] for one in held if one[0] == "answer" and one[3] and one[3][2]]
  return {
    "value": got,
    "facts": len(held),
    # The record keeps no ask, since a later life asks again for nothing it was answered; the answers say how many.
    "asks": kinds.count("answer"),
    "rungs": kinds.count("rung"),
    "prompts": kinds.count("prompt"),
    "commands": kinds.count("bash"),
    "reads": kinds.count("read"),
    "writes": kinds.count("write"),
    "input": sum(one[0] for one in usage),
    "output": sum(one[1] for one in usage),
    "cache_read": sum(one[2] for one in usage),
    "usd": sum(one[4] for one in usage),
    "wall_seconds": round(time.monotonic() - began, 1),
    "root": root,
  }


def archived(task: str, run_dir: Path, meta: Mapping[str, object]) -> None:
  """Keep one graded run where it stands on its own: its record, its numbers and its reward."""
  TRACES.mkdir(parents=True, exist_ok=True)
  stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
  name = f"{task}-{stamp}"
  numbers, reward = read_json(run_dir / "numbers.json"), read_json(run_dir / "reward.json")
  if (run_dir / "record.jsonl").is_file():
    shutil.copy(run_dir / "record.jsonl", TRACES / f"{name}.jsonl")
  record = {
    "task": task,
    "to": meta.get("to"),
    "reward": reward.get("reward"),
    "partial": reward.get("partial"),
    "f2p": f"{reward.get('f2p_passed')}/{reward.get('f2p_total')}" if reward.get("f2p_total") is not None else None,
    "p2p": f"{reward.get('p2p_passed')}/{reward.get('p2p_total')}" if reward.get("p2p_total") is not None else None,
    "usd": numbers.get("usd"),
    "wall_seconds": numbers.get("wall_seconds"),
    "asks": numbers.get("asks"),
    "commands": numbers.get("commands"),
    "value": numbers.get("value"),
    "stopped": meta.get("stopped"),
    "engine": spoke(["git", "-C", str(ROOT), "rev-parse", "HEAD"]),
    "base": meta.get("base"),
    "at": stamp,
  }
  (TRACES / f"{name}.result.json").write_text(json.dumps(record, indent=2, default=repr) + "\n", encoding="utf-8")
  say(f"[deepswe] archived into traces/{name}.result.json")


def counted(numbers: Mapping[str, object], name: str) -> int:
  """One count a reading of the record holds, as a number."""
  got = numbers.get(name)
  return int(got) if isinstance(got, int) else 0


def watched(record: Path, root: str, began: float, mark: dict[str, int]) -> str:
  """Say how the life is doing, and why it is going nowhere when it is.

  A chain past the ceiling of its grant is paused and buys nothing more, and nothing here wakes it, so the run is
  over the moment the pause stands; the turns say it, since a control tells a tag of its own name. A life that
  buys answers and does nothing with them is wedged, which the mark of the last thing it did says.
  """
  numbers = numbered(record, root, began, None)
  say(
    f"[deepswe] {numbers['wall_seconds']}s: asks={numbers['asks']} commands={numbers['commands']} "
    f"reads={numbers['reads']} writes={numbers['writes']} ${dollars(numbers):.4f}"
  )
  named = [one[0] for turn in engine.turns(on=root) for one in turn[1] if isinstance(one, tuple)]
  held = [one for one in named if one in ("paused", "woke")]
  if held[-1:] == ["paused"]:
    return "a pause stands over the chain, which buys nothing more"
  did = sum(counted(numbers, name) for name in ("commands", "reads", "writes"))
  asks = counted(numbers, "asks")
  if did > mark["did"]:
    mark.update(did=did, asks=asks)
  elif asks - mark["asks"] >= STUCK:
    return f"{asks - mark['asks']} answers bought and nothing done with them: the life is wedged"
  return ""


async def worked(told: str, app: Path, run_dir: Path, args: argparse.Namespace) -> tuple[object, str, Path]:
  """One life on the checkout, asked the task and nothing else, and what it answered.

  The message of the prompt is the instruction of the task as the task wrote it. The model is told nothing else:
  what it is and what it may say is the engine, which is its system prompt.
  """
  record = run_dir / "record.jsonl"
  world, root, held = lived(record, app, args.to)
  del world
  say(f"[deepswe] life on {app}, root {root}, {len(held)} facts kept")
  if args.ceiling:
    engine.grant(usd=args.ceiling, on=root)
  stopped, got, looks, message = "", None, 0, told + ASKED
  mark = {"did": 0, "asks": 0}
  began = time.monotonic()
  try:
    while True:
      act = engine.prompt(float, message, args.to, on=root)
      while act not in engine.outcomes:
        if args.timeout and time.monotonic() - began > args.timeout:
          stopped = f"the cap of {args.timeout:.0f} seconds ran out"
          break
        if (looks := looks + 1) % LOOK == 0 and (why := watched(record, root, began, mark)):
          stopped = why
          break
        await asyncio.sleep(TICK)
      if stopped:
        break
      got = await act
      if not isinstance(got, (int, float)) or float(got) >= CONFIDENT:
        break
      say(f"[deepswe] the model is {got} sure, under the {CONFIDENT} the rig wants; it is asked to go on")
      message = AGAIN.format(got)
  except Exception as no:
    stopped = f"the task was refused: {no!r}"
  finally:
    await cool()
  if stopped:
    say(f"[deepswe] {stopped}")
  return got, stopped, record


def run(args: argparse.Namespace) -> int:
  """Seed, let one life work the task, freeze what it left, grade it, and keep the whole of it."""
  if shutil.which(os.environ.get(BIN) or "claude") is None:
    say("[deepswe] no claude on PATH, so no model can be asked")
    raise SystemExit(1)
  work, task, meta, syn = seeded(args.task, keep_app=args.resume)
  app, run_dir = work / "app", work / ".run"
  # A fresh checkout takes a fresh record: a record beside a tree it never wrote is no run at all.
  if not args.resume:
    shutil.rmtree(run_dir, ignore_errors=True)
  run_dir.mkdir(parents=True, exist_ok=True)
  told = (task / "instruction.md").read_text(encoding="utf-8").rstrip()
  if (app / ".venv" / "bin").is_dir():
    # The commands of the model find the interpreter of the checkout first, as the verifier will.
    os.environ["PATH"] = os.pathsep.join([str(app / ".venv" / "bin"), os.environ["PATH"]])
  # The life works in the checkout, so the process that holds it stands there too: a word of a model runs in this
  # process, where python's own getcwd would otherwise name the directory of the rig and land a file outside the
  # tree that is graded.
  os.chdir(app)
  say(f"[deepswe] run {args.task}: to={args.to} ceiling={args.ceiling} timeout={args.timeout or 'none'}s")
  began = time.monotonic()
  got, stopped, record = asyncio.run(worked(told, app, run_dir, args))
  numbers = numbered(record, args.task, began, got)
  (run_dir / "numbers.json").write_text(json.dumps(numbers, indent=2, default=repr) + "\n", encoding="utf-8")
  say(
    f"[deepswe] asks={numbers['asks']} commands={numbers['commands']} reads={numbers['reads']} "
    f"writes={numbers['writes']} | ${dollars(numbers):.4f} | {numbers['wall_seconds']}s"
  )
  patch = run_dir / "model.patch"
  frozen(app, syn, patch)
  reward = graded(task, work / "base", syn, patch, run_dir / "reward.json")
  archived(args.task, run_dir, {"to": args.to, "base": syn, "stopped": stopped, "language": meta["language"]})
  return 0 if reward.get("reward") == 1 else 1


def validate(args: argparse.Namespace) -> int:
  """The control of the rig: grade the reference solution, which must score 1 before a run means anything."""
  work, task, _, syn = seeded(args.task)
  run_dir = work / ".validate"
  run_dir.mkdir(parents=True, exist_ok=True)
  patch = run_dir / "model.patch"
  shutil.copy(task / "solution" / "solution.patch", patch)
  say(f"[deepswe] grading the reference solution ({patch.stat().st_size} bytes)")
  reward = graded(task, work / "base", syn, patch, run_dir / "reward.json")
  if reward.get("reward") != 1:
    say("[deepswe] the reference solution did not score 1: this host cannot grade this task faithfully")
    return 1
  say("[deepswe] the rig grades this task faithfully")
  return 0


def turns(args: argparse.Namespace) -> int:
  """Every turn of the root of a run, as the model read them, which is how a friction is found.

  The life that folds them keeps no record of its own: a World given the record it is reading would append to it,
  and a run is read many times where it is only ever run once.
  """
  record = WORK / args.task / ".run" / "record.jsonl"
  if not record.is_file():
    say(f"[deepswe] no record at {record}")
    raise SystemExit(1)

  async def folded() -> None:
    """The life again on what the record kept, given room to say every word of it back before it is read."""
    root = engine.boot(kept(record), world=Live(str(WORK / args.task / "app"), None, args.to).hears())
    for _ in range(400):
      await asyncio.sleep(0)
    for n, (role, content, _, _) in enumerate(engine.turns(on=root)):
      say(f"{'=' * 100}\n[{n} {role}]")
      say(rendered(content))
    await cool()

  asyncio.run(folded())
  return 0


def freeze(args: argparse.Namespace) -> int:
  """Take the submission from the checkout as it stands, before anything else moves."""
  work = WORK / args.task
  frozen(work / "app", spoke(["git", "-C", str(work / "base"), "rev-parse", "HEAD"]), work / ".run" / "model.patch")
  return 0


def grade(args: argparse.Namespace) -> int:
  """Grade what the checkout holds now, or the submission a run already froze."""
  work, task, _, syn = seeded(args.task, keep_app=True)
  run_dir = work / ".run"
  run_dir.mkdir(parents=True, exist_ok=True)
  patch = run_dir / "model.patch"
  if not (patch.is_file() and patch.stat().st_size):
    frozen(work / "app", syn, patch)
  return 0 if graded(task, work / "base", syn, patch, run_dir / "reward.json").get("reward") == 1 else 1


def seed(args: argparse.Namespace) -> int:
  """Stand the workroot up and stop before any model is asked, so a run that costs money starts warm."""
  work, _, _, _ = seeded(args.task)
  say(f"[deepswe] the workroot stands at {work}")
  return 0


def main() -> None:
  """The door of the operator onto the whole rig."""
  whole = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  verbs = whole.add_subparsers(dest="verb", required=True)
  named = argparse.ArgumentParser(add_help=False)
  named.add_argument("task", help="The task, as it is named under the tasks of the collection.")
  verbs.add_parser("tools", help=tools.__doc__).set_defaults(go=tools)
  verbs.add_parser("seed", parents=[named], help=seed.__doc__).set_defaults(go=seed)
  verbs.add_parser("freeze", parents=[named], help=freeze.__doc__).set_defaults(go=freeze)
  verbs.add_parser("grade", parents=[named], help=grade.__doc__).set_defaults(go=grade)
  verbs.add_parser("validate", parents=[named], help=validate.__doc__).set_defaults(go=validate)
  reading = verbs.add_parser("turns", parents=[named], help=turns.__doc__)
  reading.add_argument("--to", default="opus/low", help="The actor the folded life stands on.")
  reading.set_defaults(go=turns)
  doing = verbs.add_parser("run", parents=[named], help=run.__doc__)
  doing.add_argument("--to", default="opus/low", help="The actor the task is put to, as model/effort.")
  doing.add_argument("--ceiling", type=float, default=8.0, help="Dollars the life may spend; 0 is no ceiling.")
  doing.add_argument("--timeout", type=float, default=1800.0, help="Seconds the run may take; 0 is no cap.")
  doing.add_argument("--resume", action="store_true", help="Go on with the checkout and the record that stand.")
  doing.set_defaults(go=run)
  args = whole.parse_args()
  raise SystemExit(args.go(args))


if __name__ == "__main__":
  main()
