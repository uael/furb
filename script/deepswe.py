"""furb against a DeepSWE task: seed a checkout, let one life work in it, grade it with the task's own verifier.

`script/CLAUDE.md` says how to run it: its verbs, its one law, and where everything stands.
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
import tarfile
import time
import tomllib
from collections.abc import Mapping, Sequence
from datetime import UTC, datetime
from email import message_from_string
from http.client import HTTPResponse
from pathlib import Path
from tempfile import gettempdir, mkdtemp
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from real import ready

from furb import engine
from furb.cli import lived, say, turned
from furb.provider.claude import cool
from furb.world import answered, kept

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
"""HOMES is what outlives a workroot: a reporter whose path this host does not let the rig write, and the layers.

A layer of an image is read once, and what it holds of python is kept under `layers`. The layers of the base image
stand in every task image, so a later task reads only its own.
"""
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
CONTAINER = ("apt-get", "apt", "apk", "yum", "dnf")
"""CONTAINER is every step of a Dockerfile that only a container can take, which this rig steps over."""
INSTALLERS = ("pip", "poetry")
"""INSTALLERS is every installer of the python of an image that a step of a Dockerfile calls.

The interpreter of the checkout holds each one that the image holds, at the version of the image. pip 26.2 could not
build an sdist that a step of dateutil fetches, and the steps of textual and tomlkit call a poetry this host lacks.
"""
MANIFEST = "application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json"
"""MANIFEST is the two forms of the manifest of one image that a registry is asked for."""
SAID = (
  ("bash -lc ", "bash -c "),
  ("python3 -m pip ", "pip "),
  ("python -m pip ", "pip "),
  ("pip3 ", "pip "),
  ("pip ", "python3 -m pip "),
)
"""SAID is how a step of a Dockerfile is said on this host, in the order the swaps apply.

A login shell of this host puts the paths of the system first, so its python is the one of the host and not the
interpreter of the checkout. In the image, a login shell and a plain shell find the same python and the same pip,
so a step takes a plain shell here.
"""
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


def captured(args: Sequence[str]) -> subprocess.CompletedProcess[bytes]:
  """One short step whose answer stands out of the log of the operator: its code, and its stdout as the bytes the
  tool wrote."""
  return subprocess.run(list(args), capture_output=True, check=False)  # noqa: S603


def spoke(args: Sequence[str]) -> str:
  """One short step whose answer is the point, as text, and nothing at all when the step failed."""
  got = captured(args)
  return got.stdout.decode(errors="replace").strip() if got.returncode == 0 else ""


def gits(tree: Path) -> list[str]:
  """Git, told which tree it speaks about and that the repo of that tree is the one it holds.

  Git told only a directory climbs to the repo above it when the .git there is empty, so a reset or a commit would
  land in a tree that is not the rig's.
  """
  return ["git", "-C", str(tree), "--git-dir", ".git"]


def git(*args: str, where: Path) -> int:
  """One step of git on one tree of the rig."""
  return ran([*gits(where), *args])


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


def image(task: Path) -> str:
  """The image a task names, which holds the environment of the task and runs its verifier."""
  with (task / "task.toml").open("rb") as fh:
    named = tomllib.load(fh).get("environment", {}).get("docker_image", "")
  if not named:
    say(f"[deepswe] {task.name}: task.toml names no docker_image")
    raise SystemExit(1)
  return named


def registry(named: str, path: str, accept: str) -> HTTPResponse:
  """One read of the registry that holds an image, open for its caller, with the anonymous token it asks for."""
  host, _, rest = named.partition("/")
  repo = rest.rpartition(":")[0]
  url = f"https://{host}/v2/{repo}/{path}"
  heard = {"Accept": accept}
  try:
    return urlopen(Request(url, headers=heard), timeout=60)
  except HTTPError as no:
    if no.code != 401:  # noqa: PLR2004
      raise
    told = dict(re.findall(r'(\w+)="([^"]*)"', no.headers.get("WWW-Authenticate", "")))
  asked = urlencode({"service": told["service"], "scope": f"repository:{repo}:pull"})
  with urlopen(f"{told['realm']}?{asked}", timeout=60) as got:  # noqa: S310
    heard["Authorization"] = f"Bearer {json.load(got)['token']}"
  return urlopen(Request(url, headers=heard), timeout=60)


def layer(named: str, digest: str) -> tuple[dict[str, dict[str, str | bool]], list[str]]:
  """What one layer of an image does to the python of the image, read once and kept.

  It is every distribution the layer writes, by the path of its dist-info, with its name, its version, the URL it
  was installed from and whether it was installed editable; and every path the layer takes away, where a path that
  ends with a slash takes the whole of its directory.
  """
  kept_at = HOMES / "layers" / f"{digest.replace(':', '-')}.json"
  if kept_at.is_file():
    said = json.loads(kept_at.read_text(encoding="utf-8"))
    return said["dists"], said["gone"]
  dists: dict[str, dict[str, str | bool]] = {}
  gone: list[str] = []
  with registry(named, f"blobs/{digest}", "*/*") as got, tarfile.open(fileobj=got, mode="r|*") as tar:
    for member in tar:
      parent, _, leaf = member.name.removeprefix("./").rpartition("/")
      if leaf == ".wh..wh..opq":
        gone.append(f"{parent}/")
      elif leaf.startswith(".wh."):
        gone.append(f"{parent}/{leaf.removeprefix('.wh.')}")
      elif parent.endswith(".dist-info") and leaf in ("METADATA", "direct_url.json") and member.isfile():
        read = tar.extractfile(member)
        text = read.read().decode(errors="replace") if read else ""
        one = dists.setdefault(parent, {"url": "", "editable": False})
        if leaf == "METADATA":
          headers = message_from_string(text)
          one |= {"name": str(headers["Name"]), "version": str(headers["Version"])}
        else:
          came = json.loads(text)
          one |= {"url": came.get("url", ""), "editable": bool(came.get("dir_info", {}).get("editable"))}
  kept_at.parent.mkdir(parents=True, exist_ok=True)
  kept_at.write_text(json.dumps({"dists": dists, "gone": gone}), encoding="utf-8")
  return dists, gone


def interpreter(named: str) -> tuple[str, dict[str, tuple[str, str, bool]]]:
  """The python that the image of a task runs, which is the python its verifier runs.

  It is the version of that python, and every distribution its site holds, by name, with its version, the URL it
  was installed from, which is empty for a distribution of an index, and whether it was installed editable. The config of the image names the version
  in PYTHON_VERSION, as the official python image sets it, and the venv in VIRTUAL_ENV when the image makes one.
  The layers say what the site holds, each in order over the ones before it. All of it is read from the registry,
  with no pull and no docker.
  """
  with registry(named, f"manifests/{named.rpartition(':')[2]}", MANIFEST) as got:
    manifest = json.load(got)
  if "config" not in manifest:
    say(f"[deepswe] {named} is an index of images, and the rig reads the python of one image only")
    raise SystemExit(1)
  with registry(named, f"blobs/{manifest['config']['digest']}", "*/*") as got:
    env = dict(one.partition("=")[::2] for one in json.load(got).get("config", {}).get("Env") or [])
  version = env.get("PYTHON_VERSION", "")
  if not version:
    say(f"[deepswe] {named} names no PYTHON_VERSION, so the rig cannot run the python that its verifier runs")
    raise SystemExit(1)
  prefix = (env.get("VIRTUAL_ENV") or "/usr/local").strip("/")
  site = f"{prefix}/lib/python{'.'.join(version.split('.')[:2])}/site-packages/"
  held: dict[str, dict[str, str | bool]] = {}
  for one in manifest["layers"]:
    dists, gone = layer(named, one["digest"])
    for path in gone:
      held = {at: dist for at, dist in held.items() if at != path and not at.startswith(path.rstrip("/") + "/")}
    held |= dists
  return version, {
    str(dist["name"]): (str(dist["version"]), str(dist["url"]), bool(dist["editable"]))
    for at, dist in held.items()
    if at.startswith(site) and "/" not in at.removeprefix(site) and "name" in dist
  }


def rebuilt(repo: Path, dists: Mapping[str, tuple[str, str, bool]]) -> None:
  """Install again each distribution that the image built from its tree, from the tree, at the version it holds.

  The base holds one commit and no tag, so a step builds the tree at a version that git makes up, and a release
  that wants a newer one goes over the editable install of the tree: the tree then imports the release. The
  version is told to every build backend in this one install, and in no step, since a version told to a step also
  reaches the build of an sdist that the step fetches, and the build of setuptools-scm itself then fails.
  """
  python = str(repo / ".venv" / "bin" / "python")
  for name, (version, url, editable) in dists.items():
    if url == "file:///app" or url.startswith("file:///app/"):
      where = str(repo / url.removeprefix("file:///app").lstrip("/"))
      told = {"SETUPTOOLS_SCM_PRETEND_VERSION": version, "PDM_BUILD_SCM_VERSION": version}
      say(f"[deepswe] {name} {version} from the tree, as the image holds it")
      if ran([python, "-m", "pip", "install", "-q", "--no-deps", *(["-e"] if editable else []), where], env=told):
        say(f"[deepswe] {name} could not be installed from {where}")
        raise SystemExit(1)


def constrained(dists: Mapping[str, tuple[str, str, bool]], into: Path) -> Path:
  """A file of constraints that holds pip to the version the image holds of each distribution it took from an index.

  A step resolves its requirements now, and the image resolved them when it was built, so a step that is free takes
  newer releases than the verifier runs. A newer pytest refused to collect the suite of dateutil. A distribution
  that the image built from the tree or took from a URL is left free, since the step names where it comes from.
  """
  pinned = sorted(f"{name}=={version}" for name, (version, url, _) in dists.items() if not url)
  into.write_text("\n".join(pinned) + "\n", encoding="utf-8")
  return into


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

  A python task gets an interpreter of its own inside the checkout, at the version its image runs, so its
  dependencies land neither in the python of the system nor in the one of this repository.
  """
  first = repo / ".venv" / "bin"
  return f'export PATH="{first}:$PATH"; ' if first.is_dir() else ""


def hosted(command: str, swaps: Sequence[tuple[str, str]]) -> str:
  """A command of a Dockerfile, as this host says it."""
  for was, now in swaps:
    command = command.replace(was, now)
  return command


def generic(repo: Path, lang: str, env: Mapping[str, str], swaps: Sequence[tuple[str, str]]) -> None:
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
  ran(["bash", "-lc", venved(repo) + hosted(words, swaps)], where=repo, env=env)


def installed(task: Path, repo: Path, lang: str) -> None:
  """Take the steps of the task's own Dockerfile in the checkout, so the tree is the one its verifier wants."""
  # A step of poetry reads and writes its config in the workroot: no setting of the host reaches the step, and
  # `poetry config` changes nothing of the host.
  env = {"npm_config_confirm_modules_purge": "false", "POETRY_CONFIG_DIR": str(repo.parent / "poetry")}
  swaps = SAID
  dists: Mapping[str, tuple[str, str, bool]] = {}
  if lang == "python":
    version, dists = interpreter(image(task))
    say(f"[deepswe] making the interpreter of the checkout: python {version}, as the image of the task runs")
    python = repo / ".venv" / "bin" / "python"
    if ran([tool("uv"), "venv", "--no-project", "--seed", "--python", version, str(repo / ".venv")]):
      say(f"[deepswe] uv could not make an interpreter of python {version}")
      raise SystemExit(1)
    pins = constrained(dists, repo.parent / "constraints.txt")
    held = [f"{name}=={dists[name][0]}" for name in INSTALLERS if name in dists]
    if held and ran([tool("uv"), "pip", "install", "-q", "--python", str(python), "-c", str(pins), *held]):
      say(f"[deepswe] uv could not install {' and '.join(held)}, as the image holds them")
      raise SystemExit(1)
    # A constraint in the environment of pip reaches the builds that pip isolates, where the image had none, so it
    # is told on the command line of each install.
    swaps = (*swaps, ("python3 -m pip install ", f"python3 -m pip install -c {shlex.quote(str(pins))} "))
  told = steps(task)
  if not told:
    say(f"[deepswe] {task.name}: the Dockerfile says no step; installing by language instead")
    generic(repo, lang, env, swaps)
  for step in told:
    if step.split(" ", 1)[0] in CONTAINER:
      say(f"[deepswe] over (a container alone takes it): {step[:80]}")
      continue
    cmd = re.sub(r"(?<![\w./])/app\b", str(repo), hosted(step, swaps))
    say(f"[deepswe] RUN {cmd[:90]}")
    if ran(["bash", "-lc", venved(repo) + cmd], where=repo, env=env):
      say("[deepswe]   the step failed; the rig goes on with what did land")
  if told and lang in ("typescript", "javascript") and not (repo / "node_modules").is_dir():
    generic(repo, lang, env, swaps)
  rebuilt(repo, dists)


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


def whole(base: Path, seal: Path, sha: str) -> bool:
  """Whether the base stands as its seed left it: the seal names its commit, and no file of that commit is gone.

  The reaper of this host takes the files of an old checkout and leaves its directories. The seal is written last,
  so a seed that stopped halfway leaves none, and the reaper takes it with the rest.
  """
  said = seal.read_text(encoding="utf-8") if seal.is_file() else ""
  return said == sha and captured([*gits(base), "diff", "--quiet", "--diff-filter=D", "HEAD", "--"]).returncode == 0


def seeded(task: str, *, keep_app: bool = False) -> tuple[Path, Path, Mapping[str, str], str]:
  """Everything one run wants before a model is asked anything: the base, its dependencies, and the checkout."""
  where, meta = read_task(task)
  work = WORK / task
  base, app, seal = work / "base", work / "app", work / ".seeded"
  work.mkdir(parents=True, exist_ok=True)
  if not whole(base, seal, meta["base_commit_hash"]):
    seal.unlink(missing_ok=True)
    based(meta["repository_url"], meta["base_commit_hash"], base)
    say(f"[deepswe] installing dependencies ({meta['language']}, from the Dockerfile of the task)")
    installed(where, base, meta["language"])
    seal.write_text(meta["base_commit_hash"], encoding="utf-8")
  submodules(
    meta["repository_url"].removeprefix("https://github.com/").removesuffix(".git"), meta["base_commit_hash"], base
  )
  syn = spoke([*gits(base), "rev-parse", "HEAD"])
  say(f"[deepswe] base {syn[:10]}, whose tree is upstream {meta['base_commit_hash'][:10]}")
  if keep_app and captured([*gits(app), "rev-parse", "--verify", "-q", "HEAD"]).returncode == 0:
    say(f"[deepswe] keeping the checkout as it stands ({spoke([*gits(app), 'rev-parse', '--short', 'HEAD'])})")
  else:
    say("[deepswe] seeding the checkout from the base")
    pristine(base, syn, app)
  return work, where, meta, syn


def frozen(app: Path, syn: str, into: Path) -> None:
  """The submission, taken the moment the tree is final: everything the life left, as one diff against the base."""
  if spoke([*gits(app), "status", "--porcelain"]):
    git("add", "-A", where=app)
    named = ["-c", "user.email=deepswe@local", "-c", "user.name=deepswe"]
    git(*named, "commit", "-q", "-m", "submission", "--no-verify", where=app)
  said = captured([*gits(app), "diff", "--binary", syn, "HEAD"]).stdout
  into.write_bytes(said)
  say(f"[deepswe] submission frozen: {len(said)} bytes into {into}")


def mode() -> str:
  """How this host can run the verifier of a task: its own image, a mount namespace of its own, or plain paths."""
  told = os.environ.get("DEEPSWE_GRADE")
  if told:
    return told
  if shutil.which("unshare") and captured(["unshare", "-m", "true"]).returncode == 0:
    return "unshare"
  return "docker" if shutil.which("docker") and captured(["docker", "info"]).returncode == 0 else "local"


def pristine(base: Path, syn: str, into: Path) -> None:
  """A fresh copy of the base at its commit, whose environment is its own and not the base's.

  The checkout of a run is one, and the grade applies the patch on another, never on the tree of a run. A venv
  writes its own path into the scripts it installs and into the hook of an editable package, so a plain copy runs
  the python of the base, imports the code of the base, and installs into the base. Every name of the base in the
  venv of the copy therefore moves to the copy. A compiled module that holds the name is dropped, and python
  compiles it again.
  """
  shutil.rmtree(into, ignore_errors=True)
  ran(["cp", "-a", str(base), str(into)])
  git("reset", "-q", "--hard", syn, where=into)
  git("clean", "-qfd", *(a for k in KEEP for a in ("-e", k)), where=into)
  was, now = re.compile(re.escape(str(base).encode()) + rb"(?![\w.-])"), str(into).encode()
  for path in (into / ".venv").rglob("*"):
    if path.is_symlink():
      if was.search(to := os.fsencode(path.readlink())):
        path.unlink()
        path.symlink_to(os.fsdecode(was.sub(now, to)))
    elif path.is_file() and was.search(said := path.read_bytes()):
      if path.suffix == ".pyc":
        path.unlink()
      elif b"\0" in said:
        say(f"[deepswe] {path} is a binary that names the base, and the rig cannot move it to {into}")
        raise SystemExit(1)
      else:
        path.write_bytes(was.sub(now, said))


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
    pinned = image(task)
    ran(["docker", "run", "--rm", "-v", f"{tests}:/tests:ro", "-v", f"{logs}:/logs", pinned, "bash", "/tests/test.sh"])
  else:
    app = vroot / "app"
    pristine(base, syn, app)
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
  # The log and the reports beside a reward are the ones of that grade, so those of an earlier grade go first: a
  # move onto a directory that stands puts the new one inside it, and the move after that fails.
  for held in ("run.log", "reports"):
    beside = out.parent / held
    if beside.is_dir():
      shutil.rmtree(beside)
    beside.unlink(missing_ok=True)
    if (logs / "verifier" / held).exists():
      shutil.move(str(logs / "verifier" / held), str(beside))
  shutil.rmtree(vroot, ignore_errors=True)
  say(f"[deepswe] reward {reward.get('reward')}: {json.dumps(reward)}")
  return reward


def read_json(path: Path) -> Mapping[str, object]:
  """What a file says, and nothing at all when no step ever wrote it."""
  return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}


def numbered(record: Path, began: float) -> dict[str, float]:
  """What the life did and what it cost, read off the record it kept."""
  entries = kept(record) if record.is_file() else []
  kinds = [entry[0][0] for entry in entries]
  answers = [one[3] for one in answered(entries)]
  usage = [one[2] for one in answers if one[2]]
  return {
    "facts": len(entries),
    "answers": len(answers),
    # The journal keeps a rung that the operator wrote, and of a rung that a prompt made, only its reply.
    "rungs": kinds.count("rung") + kinds.count("reply"),
    "prompts": kinds.count("prompt"),
    "commands": kinds.count("bash"),
    "reads": kinds.count("read"),
    "writes": kinds.count("write"),
    "input": sum(one[0] for one in usage),
    "output": sum(one[1] for one in usage),
    "cache_read": sum(one[2] for one in usage),
    "usd": sum(one[4] for one in usage),
    "wall_seconds": round(time.monotonic() - began, 1),
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
    "answers": numbers.get("answers"),
    "commands": numbers.get("commands"),
    "value": numbers.get("value"),
    "stopped": meta.get("stopped"),
    "engine": spoke(["git", "-C", str(ROOT), "rev-parse", "HEAD"]),
    "base": meta.get("base"),
    "at": stamp,
  }
  (TRACES / f"{name}.result.json").write_text(json.dumps(record, indent=2, default=repr) + "\n", encoding="utf-8")
  say(f"[deepswe] archived into traces/{name}.result.json")


def watched(record: Path, root: str, began: float, mark: dict[str, float]) -> str:
  """Say how the life is doing, and why it is going nowhere when it is.

  A chain past the ceiling of its grant is paused and buys nothing more, and nothing here wakes it, so the run is
  over the moment the pause stands. A life that buys answers and does nothing with them is wedged, which the mark of
  the last thing it did says.
  """
  numbers = numbered(record, began)
  say(
    f"[deepswe] {numbers['wall_seconds']}s: answers={numbers['answers']} commands={numbers['commands']} "
    f"reads={numbers['reads']} writes={numbers['writes']} ${numbers['usd']:.4f}"
  )
  if engine.paused(root):
    return "a pause stands over the chain, which buys nothing more"
  did = sum(numbers[name] for name in ("commands", "reads", "writes"))
  answers = numbers["answers"]
  if did > mark["did"]:
    mark.update(did=did, answers=answers)
  elif answers - mark["answers"] >= STUCK:
    return f"{answers - mark['answers']} answers bought and nothing done with them: the life is wedged"
  return ""


async def worked(told: str, app: Path, run_dir: Path, args: argparse.Namespace) -> tuple[object, str, Path]:
  """One life on the checkout, asked the task and nothing else, and what it answered.

  The message of the prompt is the instruction of the task as the task wrote it. The model is told nothing else:
  what it is and what it may say is the engine, which is its system prompt.
  """
  record = run_dir / "record.jsonl"
  world, root, held = lived(record, app, args.to, keeps=True)
  del world
  say(f"[deepswe] life on {app}, root {root}, {len(held)} facts kept")
  if args.ceiling:
    engine.grant(usd=args.ceiling, on=root)
  stopped, got, looks, message = "", None, 0, told + ASKED
  mark: dict[str, float] = {"did": 0, "answers": 0}
  began = time.monotonic()
  try:
    while True:
      act = engine.prompt(float, message, args.to, on=root)
      while engine.peek(act, ...) is ...:
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
  ready()
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
  numbers = numbered(record, began)
  (run_dir / "numbers.json").write_text(
    json.dumps({"value": got, **numbers, "root": args.task}, indent=2, default=repr) + "\n", encoding="utf-8"
  )
  say(
    f"[deepswe] answers={numbers['answers']} commands={numbers['commands']} reads={numbers['reads']} "
    f"writes={numbers['writes']} | ${numbers['usd']:.4f} | {numbers['wall_seconds']}s"
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
  """Every turn of the root of a run, as the model read them, which is how a friction is found."""
  record = WORK / args.task / ".run" / "record.jsonl"
  if not record.is_file():
    say(f"[deepswe] no record at {record}")
    raise SystemExit(1)
  asyncio.run(turned(record, WORK / args.task / "app", args.to))
  return 0


def freeze(args: argparse.Namespace) -> int:
  """Take the submission from the checkout as it stands, before anything else moves."""
  work = WORK / args.task
  frozen(work / "app", spoke([*gits(work / "base"), "rev-parse", "HEAD"]), work / ".run" / "model.patch")
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


def outside() -> None:
  """Take the venv that runs the rig out of the environment that every child of the rig inherits.

  `uv run` puts the venv of furb first on the PATH and names it in VIRTUAL_ENV. A command of a model, a step of a
  seed and a grade are children of the rig, and each would find the python, the pip and the pytest of furb there.
  A rig that runs on a python with no venv has nothing of its own to take out.
  """
  own = Path(sys.prefix).resolve()
  if own == Path(sys.base_prefix).resolve():
    return
  path = os.environ.get("PATH", "").split(os.pathsep)
  os.environ["PATH"] = os.pathsep.join(one for one in path if Path(one).resolve() != own / "bin")
  if (named := os.environ.get("VIRTUAL_ENV")) and Path(named).resolve() == own:
    del os.environ["VIRTUAL_ENV"]


def main() -> None:
  """The door of the operator onto the whole rig."""
  outside()
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
