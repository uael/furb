import asyncio
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent.parent
SOURCE = (HERE / "conductor.py").read_text(encoding="utf-8")
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)
POSIX = pytest.mark.skipif(sys.platform == "win32", reason="the round runs its commands in sh")
JQ = pytest.mark.skipif(not shutil.which("jq"), reason="the stub of gh applies --jq with jq")
GH = """import json, os, re, subprocess, sys
args = sys.argv[1:]
at = args.index("--jq") if "--jq" in args else len(args)
jq = args[at + 1] if at < len(args) else None
said = " ".join(args[:at] + args[at + 2:])
with open(os.environ["GH_LOG"], "a", encoding="utf-8") as f:
  f.write(said + chr(10))
with open(os.environ["GH_SAYS"], encoding="utf-8") as f:
  says = json.load(f)
for pattern, out, code in says:
  if re.search(pattern, said):
    break
else:
  sys.exit(0)
if jq and out:
  out = subprocess.run(["jq", "-r", jq], input=out, capture_output=True, text=True, check=True).stdout
sys.stdout.write(out)
sys.exit(code)
"""


@dataclass
class Text:
  path: str
  content: str = ""

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Refused(Exception):
  pass


def iso(at: float) -> str:
  return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(at))


def check(name: str, status: str, conclusion: str, started: str | None) -> dict:
  return {"__typename": "CheckRun", "name": name, "status": status, "conclusion": conclusion, "startedAt": started}


def raw(n: int, head: str, minutes: int, rollup: list[dict]) -> dict:
  return {
    "number": n,
    "isDraft": False,
    "labels": [],
    "mergeable": "MERGEABLE",
    "mergeStateStatus": "CLEAN",
    "baseRefName": "main",
    "headRefName": head,
    "headRefOid": f"sha{n}",
    "statusCheckRollup": rollup,
    "updatedAt": iso(time.time() - minutes * 60),
    "body": "",
  }


def run(n: int, workflow: str, at: str, conclusion: str) -> dict:
  return {"databaseId": n, "workflowName": workflow, "createdAt": f"2026-10-23T{at}:00Z", "conclusion": conclusion}


class Round:
  def __init__(self, tmp: Path, chain: str = "chain1", modules: dict | None = None) -> None:
    self.tmp, self.now = tmp, 1000.0
    self.chain, self.modules = chain, {} if modules is None else modules
    self.rungs: list[tuple[str, str]] = []
    self.targets: list[str] = []
    self.says: list[tuple[str, str, int]] = []
    self.prompts: list[tuple[str, str]] = []
    self.hired: list[tuple[str, str]] = []
    self.files: dict[str, str] = {}
    self.facts: dict[str, tuple] = {}
    self.done: dict[str, object] = {}
    folder = tmp / "bin"
    folder.mkdir()
    (folder / "gh").write_text(f"#!{sys.executable}\n{GH}", encoding="utf-8")
    (folder / "gh").chmod(0o755)
    (tmp / "home.gitconfig").write_text("", encoding="utf-8")
    who = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
    self.env = {
      **{name: value for name, value in os.environ.items() if not name.startswith("GIT_")},
      **who,
      "GIT_CONFIG_GLOBAL": str(tmp / "home.gitconfig"),
      "GIT_CONFIG_NOSYSTEM": "1",
      "PATH": f"{folder}{os.pathsep}{os.environ['PATH']}",
      "GH_LOG": str(tmp / "gh.log"),
      "GH_SAYS": str(tmp / "says.json"),
    }
    self.say("^$", "")
    self.says.clear()
    self.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
    ns: dict = {"__name__": "conductor", "re": re, "Text": Text, "Exit": Exit, "Refused": Refused}
    ns.update(HIDDEN=None, OPERATOR="operator", ROOT="chain1", bash=self.bash, clock=lambda: self.now)
    ns.update(cwd=lambda: str(tmp), get=self.facts.get, over=lambda name: name in self.done, prompt=self.prompt)
    ns.update(peek=lambda name, waiting=None: self.done.get(name, waiting), read=self.read, write=self.write)
    ns.update(transcript=lambda on="", of="": [], paused=lambda name: False, scope=lambda name: self.chain)
    ns.update(
      acting=lambda: "rung1",
      wake=lambda name: None,
      span=lambda lo, hi: None,
      module=lambda on="": self.modules.get(on, {}),
    )
    ns.update(chain=lambda label="", source="": f"chain-{label}", rung=self.rung)
    exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), ns)  # noqa: S102
    self.real_hire = ns["hire"]
    ns.update(quietly=lambda verb, *args, **words: verb(*args, **words), hire=self.hire)
    self.ns = ns
    self.modules[chain] = ns

  def __getattr__(self, name: str) -> object:
    return self.ns[name]

  def say(self, pattern: str, out: str, code: int = 0) -> None:
    self.says.append((pattern, out, code))
    (self.tmp / "says.json").write_text(json.dumps(self.says), encoding="utf-8")

  def log(self) -> str:
    path = self.tmp / "gh.log"
    return path.read_text(encoding="utf-8") if path.exists() else ""

  def bash(self, command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object):
    async def done() -> Exit:
      got = subprocess.run(
        ["sh", "-c", command],
        input=stdin or "",
        capture_output=True,
        text=True,
        env=self.env,
        cwd=self.tmp,
        check=False,
      )
      return Exit(got.returncode, Text("stdout", got.stdout), Text("stderr", got.stderr))

    return done()

  def prompt(self, shape: object, message: str = "", to: str = "", on: str = "") -> str:
    self.prompts.append((message, on))
    self.targets.append(to)
    return f"prompt{100 + len(self.prompts)}"

  def rung(self, word: str = "", retells: str = "", to: str = "", on: str = "") -> str:
    self.rungs.append((word, on))
    return f"rung{len(self.rungs)}"

  def read(self, path: str, show: object = None, on: str = "") -> Text:
    if path not in self.files:
      raise Refused(path)
    return Text(path, self.files[path])

  def write(self, text: Text, on: str = "") -> Text:
    self.files[text.path] = text.content
    return text

  def hire(self, key: str, what: str, thread: str = "", brief: str = "") -> str:
    self.hired.append((key, brief))
    name = f"worker{len(self.hired)}"
    self.ns["working"][name] = key
    self.ns["models"][name] = self.ns["model_of"](key)
    return name

  def busy(self, name: str, key: str, model: str) -> None:
    self.ns["working"][name] = key
    self.ns["models"][name] = model
    self.facts[name] = ("prompt", name, "rung1", f"chain-{key}", "None", "", "")

  def pr(self, n: int = 5, checks: tuple = (), **more: object) -> dict:
    one = {
      "n": n,
      "sha": "abc",
      "head": f"w/k{n}",
      "base": "main",
      "labels": [],
      "mergeable": "MERGEABLE",
      "draft": False,
      "checks": list(checks),
      "names": [f"c{i}" for i in range(len(checks))],
      "open": [],
      "idle": 0,
      **more,
    }
    one["verdict"] = self.ns["verdict"](one)
    return one


@pytest.fixture
def conductor(tmp_path: Path) -> Round:
  return Round(tmp_path)


@JQ
def test_a_check_counts_by_its_newest_run(conductor: Round) -> None:
  program = conductor.PRS.split("--jq '", 1)[1].split("'\nc=$?", 1)[0]
  title = [
    check("the title", "COMPLETED", c, f"2026-10-23T0{h}:00:00Z")
    for h, c in ((1, "FAILURE"), (3, "SUCCESS"), (2, "FAILURE"))
  ]
  hooks = [check("the hooks", "COMPLETED", "CANCELLED", "2026-10-23T01:00:00Z"), check("the hooks", "QUEUED", "", None)]
  given = json.dumps([raw(252, "w/x", 5, title + hooks)])
  got = json.loads(subprocess.run(["jq", program], input=given, capture_output=True, text=True, check=True).stdout)[0]
  assert dict(zip(got["names"], got["checks"], strict=True)) == {"the title": "SUCCESS", "the hooks": ""}


@POSIX
@JQ
async def test_a_cancelled_head_runs_again_once_then_counts_as_failed(conductor: Round) -> None:
  conductor.say(r"pr view 5 --json headRefOid", '{"headRefOid": "abc"}')
  runs = [
    run(1, "gates", "10:00", "cancelled"),
    run(2, "gates", "09:00", "success"),
    run(3, "titles", "09:30", "success"),
  ]
  conductor.say(r"run list --commit abc", json.dumps(runs))
  first = conductor.pr(checks=("CANCELLED", "SUCCESS"))
  assert first["verdict"] == "pending"
  assert await conductor.mend(first, 0.0, False) == ""
  assert "run rerun 1\n" in conductor.log()
  assert conductor.heard == ["#5 had a run cancelled at its head abc, and the round ran it again."]
  assert conductor.hired == []
  again = conductor.pr(checks=("CANCELLED", "SUCCESS"))
  assert again["verdict"] == "fail"
  await conductor.mend(again, 0.0, False)
  assert conductor.log().count("run rerun") == 1
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_failure_behind_a_green_main_updates_its_branch_before_a_fixer(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, False)
  assert "pr update-branch 5\n" in conductor.log()
  assert conductor.heard == ["#5 failed at abc behind main, which is green, so the round updated its branch."]
  assert conductor.hired == []
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, False)
  assert conductor.log().count("update-branch") == 1
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_red_main_updates_no_branch(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, True)
  assert "update-branch" not in conductor.log()
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_green_pr_behind_main_updates_its_branch_though_heal_updated_it_at_that_head(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  conductor.say(r"compare/abc\.\.\.main", '{"files": [{"filename": "b.py"}]}')
  conductor.say(r"pr view 5 --json files", '{"files": [{"path": "b.py"}]}')
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, False)
  assert conductor.log().count("update-branch") == 1
  green = conductor.pr(checks=("SUCCESS",), names=["the hooks"])
  await conductor.land(green, False)
  assert conductor.log().count("update-branch") == 2
  await conductor.land(green, False)
  assert conductor.log().count("update-branch") == 2


@POSIX
@JQ
async def test_a_draft_or_a_held_or_a_paused_pr_gets_no_branch_update_from_heal(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  for more in ({"draft": True}, {"labels": ["hold"]}, {"labels": ["paused"]}):
    assert await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE"), **more), False) == ""
  assert "update-branch" not in conductor.log()
  assert "heal update abc" not in conductor.noticed
  assert "behind main" in await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE")), False)


@POSIX
@JQ
async def test_a_lone_failure_reruns_its_failed_jobs_once_before_a_fixer(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 0}')
  conductor.say(r"pr view 5 --json headRefOid", '{"headRefOid": "abc"}')
  conductor.say(r"run list --commit abc", json.dumps([run(1, "gates", "10:00", "failure")]))
  await conductor.mend(conductor.pr(checks=("FAILURE", "SUCCESS")), 0.0, False)
  assert "run rerun 1 --failed\n" in conductor.log()
  assert conductor.heard == ["#5 failed one check alone at abc, and the round ran its failed jobs again."]
  assert conductor.hired == []
  await conductor.mend(conductor.pr(checks=("FAILURE", "SUCCESS")), 0.0, False)
  assert conductor.log().count("run rerun") == 1
  assert [key for key, _ in conductor.hired] == ["fix5"]


def test_new_items_wait_while_an_open_pr_needs_a_worker(conductor: Round) -> None:
  conductor.efforts.update({key: SONNET for key in "abcdef"})
  conductor.queue[:] = [("a", "", ""), ("b", "", "")]
  conductor.backlog[:] = [("c", "", ""), ("d", "", ""), ("e", "", ""), ("f", "", "")]
  conductor.prs[7] = conductor.pr(n=7, checks=("FAILURE",))
  conductor.staff()
  assert conductor.hired == []
  conductor.working["prompt9"] = "fix7"
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["a", "b", "c"]
  assert conductor.queue == [("d", "", "")]
  assert conductor.backlog == [("e", "", ""), ("f", "", "")]


def test_each_brief_forbids_a_window_on_this_machine(conductor: Round) -> None:
  rule = "Never open a window on this machine, not even for a test"
  assert rule in conductor.WORKER
  assert rule in conductor.FIX
  conductor.boxes(conductor.pr(n=8, open=["- [ ] It is one change."]), 0.0)
  assert [key for key, _ in conductor.hired] == ["box8"]
  assert rule in conductor.hired[0][1]


@POSIX
async def test_a_held_green_pr_shows_s_held(conductor: Round) -> None:
  conductor.prs[9] = conductor.pr(n=9, checks=("SUCCESS",), names=["the hooks"], labels=["hold"])
  await conductor.mark()
  assert "pr edit 9 --add-label s:held " in conductor.log()


@POSIX
@JQ
async def test_an_upstream_ready_fork_pr_is_told_once(conductor: Round) -> None:
  ready = [{"name": "upstream-ready"}]
  forks = [
    {"number": 4, "headRefName": "w/x", "title": "T", "url": "u4", "labels": ready},
    {"number": 5, "headRefName": "w/y", "title": "U", "url": "u5", "labels": [*ready, {"name": "upstreamed"}]},
  ]
  conductor.say(r"pr list --repo uael/monty", json.dumps(forks))
  await conductor.upstream_watch()
  await conductor.upstream_watch()
  assert len(conductor.prompts) == 1
  assert conductor.prompts[0][0].startswith("The owner put the label upstream-ready on the fork PR u4 (w/x: T).")


def test_a_worker_that_ends_with_a_pr_is_a_line_and_its_debrief_feeds_the_ledger(conductor: Round) -> None:
  conductor.working["prompt1"] = "item1"
  conductor.facts["prompt1"] = ("prompt", "prompt1", "rung1", "chain5", "tuple[int, str]", "brief", "")
  conductor.done["prompt1"] = (12, "report")
  conductor.tend()
  assert conductor.heard == ["The worker item1 (prompt1) ended with PR #12, and results['prompt1'] keeps its report."]
  assert conductor.prompts == [(conductor.DEBRIEF, "chain5")]
  conductor.done["prompt101"] = ["No jq on the PATH", "A  slow gate", "a slow gate"]
  conductor.tend()
  conductor.tend()
  ledger = json.loads(conductor.files[conductor.TOOLING])
  assert [(x["what"], x["count"], x["source"]) for x in ledger] == [
    ("No jq on the PATH", 1, "debrief of item1"),
    ("A  slow gate", 2, "debrief of item1"),
  ]
  assert len(conductor.prompts) == 1

  async def pruned() -> str:
    return "The prune found nothing to remove."

  conductor.ns["prune"] = pruned
  asyncio.run(conductor.audit(False))
  told = conductor.prompts[-1][0]
  assert "- 2x A  slow gate\n- 1x No jq on the PATH\nThe prune found nothing to remove." in told


@POSIX
@JQ
async def test_a_notice_carries_its_context_and_each_other_event_is_a_line(conductor: Round) -> None:
  prs = [
    raw(1, "feature", 5, []),
    raw(2, "w/idle", 60, [check("the title", "COMPLETED", "SUCCESS", "2026-10-23T10:00:00Z")]),
    raw(3, "w/busy", 60, [check("the hooks", "IN_PROGRESS", "", "2026-10-23T10:00:00Z")]),
  ]
  conductor.say(r"pr list --state open", json.dumps(prs))
  conductor.say(r"pr list --state merged", "[]")
  conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "failure", "databaseId": 9}]))
  conductor.say(r"run view 9", "unit\tpytest\tFAILED test/test_x.py::test_y - boom\nunit\tpytest\tok\n")
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  await conductor.sweep(300)
  told = [message for message, _ in conductor.prompts]
  assert told[0] == (
    "main is red at m1. Find its cause first. The lines that failed:\nunit\tFAILED test/test_x.py::test_y - boom"
  )
  assert told[1].startswith("#2 has had no change for ")
  assert "Its checks at sha2: the title SUCCESS. Move it on, or say why it waits." in told[1]
  assert told[1].endswith(
    "- #1 (feature) was not made by a worker of the round, so the round neither mends nor merges it."
  )
  assert len(told) == 2


@POSIX
@JQ
async def test_a_pr_that_does_not_merge_tells_what_gh_said(conductor: Round) -> None:
  conductor.say(r"compare/sha7\.\.\.main", '{"files": [{"filename": "b.py"}]}')
  conductor.say(r"pr view 7 --json files", '{"files": [{"path": "a.py"}]}')
  conductor.say(r"pr merge 7", "X Pull request #7 is not mergeable: the merge commit cannot be cleanly created.\n", 1)
  conductor.say(r"pr view 7 --json state", '{"state": "OPEN"}')
  await conductor.land(conductor.pr(n=7, sha="sha7", checks=("SUCCESS",), names=["the hooks"]), False)
  assert conductor.prompts[0][0].startswith(
    "#7 did not merge at sha7, and gh said:\nX Pull request #7 is not mergeable: the merge commit cannot be cleanly"
  )


def git(*args: str, cwd: Path, env: dict) -> str:
  return subprocess.run(["git", *args], cwd=cwd, env=env, capture_output=True, text=True, check=True).stdout


def aged(path: Path, days: float) -> None:
  then = time.time() - days * 86400
  for one in [path, *path.rglob("*")]:
    os.utime(one, (then, then), follow_symlinks=False)


@POSIX
@pytest.mark.skipif(not shutil.which("lsof"), reason="the prune reads the open files with lsof")
def test_the_prune_removes_what_the_work_left_and_keeps_its_work(conductor: Round, tmp_path: Path) -> None:
  env = conductor.env
  origin, root = tmp_path / "origin.git", tmp_path / "furb"
  git("init", "-q", "--bare", "-b", "main", str(origin), cwd=tmp_path, env=env)
  git("init", "-q", "-b", "main", str(root), cwd=tmp_path, env=env)
  (root / "a.txt").write_text("a\n", encoding="utf-8")
  git("add", "-A", cwd=root, env=env)
  git("commit", "-q", "-m", "a", cwd=root, env=env)
  git("remote", "add", "origin", str(origin), cwd=root, env=env)
  git("push", "-q", "-u", "origin", "main", cwd=root, env=env)
  main = git("rev-parse", "main", cwd=root, env=env).strip()
  for key in ("merged", "open", "busy", "empty"):
    git("worktree", "add", "-q", "-b", f"w/{key}", str(tmp_path / f"furb-w-{key}"), cwd=root, env=env)
  merged = tmp_path / "furb-w-merged"
  (merged / "b.txt").write_text("b\n", encoding="utf-8")
  git("add", "-A", cwd=merged, env=env)
  git("commit", "-q", "-m", "b", cwd=merged, env=env)
  (merged / "notes.txt").write_text("not committed\n", encoding="utf-8")
  git("push", "-q", "origin", "w/open", "main:refs/heads/w/closed", "main:refs/heads/w/moved", cwd=root, env=env)
  listed = [
    {"number": 1, "state": "MERGED", "headRefName": "w/merged", "headRefOid": "x", "isCrossRepository": False},
    {"number": 2, "state": "OPEN", "headRefName": "w/open", "headRefOid": main, "isCrossRepository": False},
    {"number": 3, "state": "CLOSED", "headRefName": "w/busy", "headRefOid": main, "isCrossRepository": False},
    {"number": 4, "state": "CLOSED", "headRefName": "w/closed", "headRefOid": main, "isCrossRepository": False},
    {"number": 5, "state": "CLOSED", "headRefName": "w/moved", "headRefOid": "old", "isCrossRepository": False},
  ]
  conductor.say(r"pr list --state all", json.dumps(listed))
  scratch, folder = tmp_path / "scratch", tmp_path / "tmp"
  record = tmp_path / "life.jsonl"
  record.write_text("", encoding="utf-8")
  life = hashlib.sha256(os.path.realpath(record).encode()).hexdigest()[:16]
  for name in ("old1", "new1", "held1", life):
    (scratch / name).mkdir(parents=True)
    (scratch / name / "f").write_text("f", encoding="utf-8")
  for name in ("furb-old", "furb-new", "other-old"):
    (folder / name).mkdir(parents=True)
  for one in (scratch / "old1", scratch / "held1", scratch / life, folder / "furb-old", folder / "other-old"):
    aged(one, 3)
  spec = {"root": str(root), "keys": ["busy"], "text": "", "archive": ".furb/archive", "trashed": ".furb/trashed.tsv"}
  (tmp_path / "spec.json").write_text(json.dumps({**spec, "tmp": str(folder)}), encoding="utf-8")
  with (scratch / "held1" / "f").open(encoding="utf-8"), record.open(encoding="utf-8"):
    got = subprocess.run(
      ["python3", "-I", "-", str(tmp_path / "spec.json"), str(scratch)],
      input=conductor.PRUNE_PY,
      capture_output=True,
      text=True,
      env=env,
      cwd=tmp_path,
      check=True,
      timeout=25,
    )
  assert got.stdout.startswith("The prune removed the worktrees ")
  assert "; 1 remote branch, listed in .furb/trashed.tsv; 1 scratch directory; 1 folder of " in got.stdout
  assert sorted(p.name for p in tmp_path.glob("furb-w-*")) == ["furb-w-busy", "furb-w-open"]
  assert git("for-each-ref", "--format=%(refname:short)", "refs/heads/w", cwd=root, env=env).split() == [
    "w/busy",
    "w/open",
  ]
  kept = sorted((root / ".furb" / "archive").glob("*/*"))
  assert [re.sub(r"-\d{6}\.", ".", p.name) for p in kept] == ["merged.bundle", "merged.patch"]
  assert "notes.txt" in kept[1].read_text(encoding="utf-8")
  assert "refs/heads/w/merged" in git("bundle", "list-heads", str(kept[0]), cwd=root, env=env)
  heads = git("ls-remote", "--heads", "origin", cwd=root, env=env)
  assert sorted(line.split("\t")[1] for line in heads.splitlines()) == [
    "refs/heads/main",
    "refs/heads/w/moved",
    "refs/heads/w/open",
  ]
  assert (root / ".furb" / "trashed.tsv").read_text(encoding="utf-8").split("\t")[1:4] == ["w/closed", main, "4"]
  assert sorted(p.name for p in scratch.iterdir()) == sorted(["new1", "held1", life])
  assert sorted(p.name for p in folder.iterdir()) == ["furb-new", "other-old"]


@POSIX
@JQ
async def test_a_flake_of_the_gates_is_queued_at_once_and_read_once(conductor: Round) -> None:
  conductor.says.clear()
  conductor.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
  await conductor.watch_ci()
  assert conductor.queue == []
  conductor.says.clear()
  url = "https://github.com/o/r/actions/runs/50"
  flaky = [{"id": 50, "head_sha": "s1", "run_attempt": 2, "conclusion": "success", "html_url": url}]
  job = {
    "id": 7,
    "name": "the bindings and the TUI on ubuntu-latest",
    "conclusion": "failure",
    "started_at": "2026-10-23T10:00:00Z",
    "completed_at": "2026-10-23T10:01:00Z",
  }
  conductor.say(r"runs\?branch=main&status=success", '{"workflow_runs": []}')
  conductor.say(r"runs\?per_page=30", json.dumps({"workflow_runs": flaky}))
  conductor.say(r"runs/50/attempts/1/jobs", json.dumps({"jobs": [job]}))
  conductor.say(r"jobs/7/logs", "2026-10-23T10:00:30.0000000Z (fail) a host test [30000.17ms]\n")
  await conductor.watch_ci()
  await conductor.watch_ci()
  assert [(key, thread) for key, _, thread in conductor.queue] == [("flake-a-host-test", "ci")]
  assert f"{url}/attempts/1" in conductor.queue[0][1]
  assert "a host test" in conductor.queue[0][1]


@POSIX
@JQ
async def test_a_flake_that_the_newest_runs_of_main_healed_is_queued_by_no_item(conductor: Round) -> None:
  conductor.says.clear()
  conductor.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
  await conductor.watch_ci()
  conductor.says.clear()
  url = "https://github.com/o/r/actions/runs/50"
  name = "the bindings and the TUI on ubuntu-latest"
  flaky = [
    {
      "id": 50,
      "head_sha": "s1",
      "run_attempt": 2,
      "conclusion": "success",
      "html_url": url,
      "created_at": "2026-10-23T10:00:00Z",
    }
  ]
  green = [{"id": n, "run_attempt": 1, "created_at": "2026-10-24T10:00:00Z"} for n in range(1, 6)]
  ok = {
    "id": 1,
    "name": name,
    "conclusion": "success",
    "started_at": "2026-10-24T10:00:00Z",
    "completed_at": "2026-10-24T10:01:00Z",
  }
  bad = {**ok, "id": 7, "conclusion": "failure"}
  conductor.say(r"runs\?branch=main&status=success", json.dumps({"workflow_runs": green}))
  conductor.say(r"runs\?per_page=30", json.dumps({"workflow_runs": flaky}))
  conductor.say(r"runs/50/attempts/1/jobs", json.dumps({"jobs": [bad]}))
  conductor.say(r"runs/\d+/jobs", json.dumps({"jobs": [ok]}))
  conductor.say(r"jobs/7/logs", "fps p95 frame   21.3 ms <= 16.7 OVER\n")
  await conductor.watch_ci()
  assert conductor.queue == []
  assert conductor.flaky["bench: fps p95 frame"] == [f"{url}/attempts/1"]
  assert any("the 5 newest runs of main" in line for line in conductor.heard)


@POSIX
@JQ
async def test_a_job_over_its_budget_is_queued_once_until_new_runs_measure_it(conductor: Round) -> None:
  tree = conductor.tmp
  (tree / ".github").mkdir()
  (tree / ".github" / "budgets.json").write_text('{"the hooks": 60}\n', encoding="utf-8")
  git("init", "-q", "-b", "main", cwd=tree, env=conductor.env)
  git("add", ".github", cwd=tree, env=conductor.env)
  git("commit", "-q", "-m", "budgets", cwd=tree, env=conductor.env)
  git("update-ref", "refs/remotes/origin/main", "HEAD", cwd=tree, env=conductor.env)
  conductor.says.clear()
  green = {"workflow_runs": [{"id": n} for n in range(1, 6)]}
  job = {
    "id": 1,
    "name": "the hooks",
    "conclusion": "success",
    "started_at": "2026-10-23T10:00:00Z",
    "completed_at": "2026-10-23T10:03:00Z",
  }
  conductor.say(r"runs\?branch=main&status=success", json.dumps(green))
  conductor.say(r"runs/\d+/jobs", json.dumps({"jobs": [job]}))
  conductor.say(r"runs\?per_page=30", '{"workflow_runs": []}')
  await conductor.watch_ci()
  await conductor.watch_ci()
  assert [(key, thread) for key, _, thread in conductor.queue] == [("budget-the-hooks", "ci")]
  assert "took 180 s" in conductor.queue[0][1]
  assert "budget of 60 s" in conductor.queue[0][1]


@POSIX
@JQ
async def test_a_veto_keeps_a_green_pr_from_a_merge(conductor: Round) -> None:
  asked: list[int] = []

  async def deny(pr: dict) -> bool:
    asked.append(pr["n"])
    return True

  conductor.vetoes["judge"] = deny
  await conductor.land(conductor.pr(n=7, sha="sha7", checks=("SUCCESS",), names=["the hooks"]), False)
  assert asked == [7]
  assert "pr merge" not in conductor.log()


def test_the_environment_of_each_git_holds_no_variable_of_a_commit(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  for name in ("GIT_INDEX_FILE", "GIT_DIR", "GIT_WORK_TREE", "GIT_PREFIX", "GIT_EXEC_PATH"):
    monkeypatch.setenv(name, "/elsewhere")
  env = Round(tmp_path).env
  assert sorted(name for name in env if name.startswith("GIT_")) == [
    "GIT_AUTHOR_EMAIL",
    "GIT_AUTHOR_NAME",
    "GIT_COMMITTER_EMAIL",
    "GIT_COMMITTER_NAME",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_NOSYSTEM",
  ]


OPUS = "claude-cli:opus/medium"
SONNET = "claude-cli:sonnet/medium"
TOP = "claude-cli:opus/high"
SONNET_HIGH = "claude-cli:sonnet/high"


def test_the_rulings_of_the_owner_stand_in_conducting_in_his_words(conductor: Round) -> None:
  for words in (
    "opus/medium should be your default, not opus/high",
    "xhigh for the coordination and my rulings only",
    "From now on: 2 opus workers, 3 sonnet ones. max.",
    "add one opus/high worker for the most complex stuff",
  ):
    assert words in conductor.CONDUCTING
  assert conductor.ACTOR == OPUS
  assert conductor.limits == {"room": {"opus": 1, "sonnet": 3, "opus/high": 1}, "fixer": SONNET_HIGH}
  assert conductor.cap() == 5
  for words in ('limits["room"] gives the room', 'limits["fixer"], sonnet/high', "a reload of the extension keeps it"):
    assert words in conductor.CONDUCTING


def test_a_fixer_runs_on_sonnet_high_unless_its_effort_says_otherwise(conductor: Round) -> None:
  assert [conductor.model_of(key) for key in ("fix5", "box8", "k")] == [SONNET_HIGH, SONNET_HIGH, OPUS]
  conductor.efforts["fix5"] = TOP
  assert conductor.model_of("fix5") == TOP
  assert conductor.model_of("fix50") == SONNET_HIGH


def test_the_limits_of_the_round_live_in_its_state_and_a_reload_keeps_them(conductor: Round) -> None:
  conductor.limits["room"]["opus"] = 2
  conductor.limits["fixer"] = OPUS
  exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), conductor.ns)  # noqa: S102
  assert conductor.limits == {"room": {"opus": 2, "sonnet": 3, "opus/high": 1}, "fixer": OPUS}
  assert (conductor.cap(), conductor.model_of("box8")) == (6, OPUS)
  conductor.busy("p1", "a", OPUS)
  assert conductor.room(OPUS)
  conductor.busy("p2", "b", OPUS)
  assert not conductor.room(OPUS)


def stacked(conductor: Round, tmp_path: Path, clash: bool = False) -> dict[str, str]:
  env, origin = conductor.env, tmp_path / "origin.git"
  git("init", "-q", "--bare", str(origin), cwd=tmp_path, env=env)
  git("init", "-q", "-b", "main", cwd=tmp_path, env=env)
  git("remote", "add", "origin", str(origin), cwd=tmp_path, env=env)

  def commit(name: str, text: str, message: str) -> None:
    (tmp_path / name).write_text(text, encoding="utf-8")
    git("add", name, cwd=tmp_path, env=env)
    git("commit", "-q", "-m", message, cwd=tmp_path, env=env)

  commit("base.txt", "base\n", "base")
  git("push", "-q", "origin", "main", cwd=tmp_path, env=env)
  shas = {}
  for branch, name, text in (
    ("w/parent", "p.txt", "one\n"),
    ("w/kid", "k.txt", "kid\n"),
    ("w/deep", "d.txt", "deep\n"),
  ):
    git("checkout", "-q", "-b", branch, cwd=tmp_path, env=env)
    if branch == "w/kid" and clash:
      name, text = "p.txt", "kid\n"
    commit(name, text, branch[2:])
    git("push", "-q", "origin", branch, cwd=tmp_path, env=env)
    shas[branch] = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  git("checkout", "-q", "main", cwd=tmp_path, env=env)
  git("merge", "--squash", "-q", "w/parent", cwd=tmp_path, env=env)
  git("commit", "-q", "-m", "parent (#7)", cwd=tmp_path, env=env)
  if clash:
    commit("p.txt", "main\n", "main moves")
  git("push", "-q", "origin", "main", cwd=tmp_path, env=env)
  shas["main"] = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  shas["squash"] = git("rev-parse", "HEAD" if not clash else "HEAD^", cwd=tmp_path, env=env).strip()
  conductor.say(r"compare/", '{"files": [{"filename": "b.py"}]}')
  conductor.say(r"pr view 7 --json files", '{"files": [{"path": "a.py"}]}')
  conductor.say(r"pr view 7 --json mergeCommit", json.dumps({"mergeCommit": {"oid": shas["squash"]}}))
  conductor.say(r"pr view 7 --json state", '{"state": "MERGED"}')
  conductor.prs[7] = conductor.pr(n=7, head="w/parent", sha=shas["w/parent"], checks=("SUCCESS",), names=["the hooks"])
  conductor.prs[8] = conductor.pr(n=8, head="w/kid", base="w/parent", sha=shas["w/kid"])
  conductor.prs[9] = conductor.pr(n=9, head="w/deep", base="w/kid", sha=shas["w/deep"])
  return shas


def history(tmp_path: Path, conductor: Round, branch: str) -> list[str]:
  out = git("log", "--format=%s", branch, cwd=tmp_path / "origin.git", env=conductor.env)
  return out.splitlines()


CHECKS = ["checked"] * 6


def branch(conductor: Round, name: str = "w/k") -> None:
  subprocess.run(["git", "init", "-q", "-b", name], cwd=conductor.tmp, env=conductor.env, check=True)
  conductor.say(r"pr view --json number", '{"number": 311}')
  conductor.say(r"pr create", "https://github.com/o/r/pull/9\n")


@POSIX
@JQ
def test_a_worker_runs_only_sonnet_or_opus_at_medium_or_high(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  for model in ("claude-cli:opus/xhigh", "claude-cli:opus/max", "claude-cli:sonnet/max", "claude-cli:haiku/medium"):
    conductor.efforts["k"] = model
    with pytest.raises(Refused, match="xhigh and max"):
      conductor.real_hire("k", "w")
  assert conductor.working == {}
  assert conductor.log() == ""
  for model in (SONNET, "claude-cli:sonnet/high", OPUS, TOP):
    conductor.efforts["k"] = model
    name = conductor.real_hire("k", "w")
    assert conductor.models[name] == model
    assert conductor.targets[-1] == model
  assert "label create w:k --force" in conductor.log()
  del conductor.efforts["k"]
  assert conductor.models[conductor.real_hire("k", "w")] == OPUS
  assert conductor.targets[-1] == OPUS
  assert conductor.files["scratch://build.sh"] == conductor.BUILD_SH


def test_an_item_that_asks_for_xhigh_waits_and_the_owner_hears(conductor: Round) -> None:
  conductor.efforts["x"] = "claude-cli:opus/xhigh"
  conductor.queue[:] = [("x", "", ""), ("y", "", "")]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["y"]
  assert conductor.queue == [("x", "", "")]
  assert len(conductor.prompts) == 1
  assert "xhigh" in conductor.prompts[0][0]
  conductor.staff()
  assert len(conductor.prompts) == 1


def test_the_round_runs_one_on_opus_medium_three_on_sonnet_and_one_on_opus_high(conductor: Round) -> None:
  models = {"o1": OPUS, "t1": TOP, "o2": OPUS, "t2": TOP, "s1": SONNET, "s2": SONNET, "s3": SONNET}
  models["s4"] = SONNET_HIGH
  conductor.efforts.update(models)
  conductor.queue[:] = [(key, "", "") for key in models]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["o1", "t1", "s1", "s2", "s3"]
  assert conductor.queue == [("o2", "", ""), ("t2", "", ""), ("s4", "", "")]
  conductor.done["worker1"] = None
  conductor.staff()
  assert [key for key, _ in conductor.hired][5:] == ["o2"]
  assert conductor.queue == [("t2", "", ""), ("s4", "", "")]
  conductor.done["worker2"] = None
  conductor.staff()
  assert [key for key, _ in conductor.hired][6:] == ["t2"]


def test_an_item_with_no_room_keeps_its_place_while_the_next_with_room_starts(conductor: Round) -> None:
  conductor.busy("p1", "a", OPUS)
  conductor.busy("p2", "b", OPUS)
  conductor.busy("p3", "e", TOP)
  conductor.efforts["d"] = SONNET
  conductor.queue[:] = [("c", "", ""), ("d", "", "")]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["d"]
  assert conductor.queue == [("c", "", "")]


def test_a_fixer_goes_before_a_new_item_and_waits_for_room_on_sonnet(conductor: Round) -> None:
  for name, key in (("p1", "a"), ("p2", "b"), ("p3", "e")):
    conductor.busy(name, key, SONNET)
  conductor.prs[8] = conductor.pr(n=8, open=["- [ ] It is one change."], verdict="open")
  conductor.queue[:] = [("c", "", "")]
  conductor.boxes(conductor.prs[8], 0.0)
  conductor.staff()
  assert conductor.hired == []
  conductor.done["p1"] = None
  conductor.boxes(conductor.prs[8], 0.0)
  assert [key for key, _ in conductor.hired] == ["box8"]
  assert conductor.models["worker1"] == SONNET_HIGH


def test_an_ended_worker_goes_on_in_its_own_chain_on_sonnet(conductor: Round) -> None:
  conductor.homes["k8"] = "chain7"
  pr = conductor.pr(n=8, open=["- [ ] It is one change."], labels=["w:k8"])
  for index, key in enumerate("abc"):
    conductor.busy(f"p{index}", key, SONNET)
  conductor.boxes(pr, 0.0)
  assert (conductor.hired, conductor.rungs, conductor.prompts) == ([], [], [])
  conductor.done["p0"] = None
  conductor.boxes(pr, 0.0)
  assert conductor.hired == []
  assert conductor.rungs == [(f"actor = {SONNET!r}", "chain7")]
  assert conductor.targets == [SONNET]
  message, on = conductor.prompts[0]
  assert on == "chain7"
  assert message.startswith(f"Your work on this chain goes on, now on {SONNET}. Your open PR #8")
  name = next(name for name, key in conductor.working.items() if key == "k8")
  assert conductor.models[name] == SONNET
  conductor.boxes(pr, 0.0)
  assert len(conductor.prompts) == 1


def test_the_round_follows_the_compaction_of_its_orchestrator(conductor: Round, tmp_path: Path) -> None:
  (tmp_path / "new").mkdir()
  new = Round(tmp_path / "new", "chain2", conductor.modules)
  calls: list[str] = []

  async def waits(every: float, *acts: str) -> None:
    new.lead.update(conductor.lead)
    assert new.follow() == "chain2"

  async def swept(every: float) -> None:
    calls.append("sweep")

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=lambda: calls.append("tend"), staff=lambda: None, wait=waits, sweep=swept)
  assert asyncio.run(conductor.conduct()) == {}
  assert calls == ["tend", "sweep"]
  assert conductor.lead == {"home": "chain2"}
  assert new.lead == {"home": "chain2"}
  assert conductor.prompts == []


@POSIX
@JQ
def test_a_gh_that_answers_nothing_once_ends_no_round(conductor: Round) -> None:
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  conductor.queue.append(("a", "", ""))
  rounds: list[int] = []

  async def waits(every: float, *acts: str) -> None:
    rounds.append(len(rounds))
    if len(rounds) == 1:
      assert "sweep" not in conductor.timers
      conductor.facts["worker1"] = ("prompt", "worker1", "rung1", "chain-a", "None", "", "")
      conductor.say(r"pr list --state open", json.dumps([raw(4, "w/four", 5, [])]))
      conductor.say(r"pr list --state merged", '[{"number": 3}]')
      conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "success"}]))
    else:
      conductor.lead["home"] = "chain2"

  conductor.ns["wait"] = waits
  assert asyncio.run(conductor.conduct()) == {}
  assert rounds == [0, 1]
  assert [message for message, _ in conductor.prompts] == [
    "The command that lists the PRs failed at the open PRs, the merged PRs, the gates of main: gh said nothing."
  ]
  assert "sweep" in conductor.timers
  assert list(conductor.prs) == [4]
  assert 3 in conductor.merged
  assert [key for key, _ in conductor.hired] == ["a"]


def test_a_fault_of_a_step_is_told_once_and_the_round_goes_on(conductor: Round) -> None:
  calls: list[str] = []

  def tend() -> None:
    calls.append("tend")
    json.loads("")

  async def swept(every: float) -> None:
    calls.append("sweep")

  async def waits(every: float, *acts: str) -> None:
    if calls.count("sweep") == 3:
      conductor.lead["home"] = "chain2"

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=tend, sweep=swept, staff=lambda: None, wait=waits)
  assert asyncio.run(conductor.conduct()) == {}
  assert calls == ["tend", "sweep"] * 3
  assert [message for message, _ in conductor.prompts] == [
    "The round on chain1 met a fault in its step tend: JSONDecodeError: Expecting value: line 1 column 1 (char 0)."
    " The round goes on at the next round, and tells this kind of fault once."
  ]


def test_a_round_that_ends_with_no_work_tells_it_as_a_decision(conductor: Round) -> None:
  async def swept(every: float) -> None:
    return None

  conductor.ns.update(tend=lambda: None, sweep=swept, staff=lambda: None)
  assert asyncio.run(conductor.conduct()) == {}
  assert conductor.prompts == [
    (
      "The round on chain1 ended: it holds no item in queue, in backlog or in working, and no open PR that it tends."
      " Decide whether it goes on: until you await conduct() again, no sweep, no heal, no merge, no fixer and no"
      " hire runs, and no worker is tended.",
      "",
    )
  ]


def test_a_fault_that_ends_the_round_tells_its_cause(conductor: Round) -> None:
  async def swept(every: float) -> None:
    return None

  async def waits(every: float, *acts: str) -> None:
    fault = RuntimeError("boom")
    raise fault

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=lambda: None, sweep=swept, staff=lambda: None, wait=waits)
  with pytest.raises(RuntimeError, match="boom"):
    asyncio.run(conductor.conduct())
  assert len(conductor.prompts) == 1
  assert conductor.prompts[0][0].startswith(
    "The round on chain1 ended: the fault RuntimeError: boom, in its step wait."
  )


@POSIX
@JQ
async def test_a_merge_rebases_the_child_onto_main_and_pushes_it(conductor: Round, tmp_path: Path) -> None:
  shas = stacked(conductor, tmp_path)
  del conductor.prs[9]
  await conductor.land(conductor.prs[7], False)
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent (#7)", "base"]
  assert git("rev-parse", "w/kid^", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["squash"]
  assert "pr edit 8 --base main" in conductor.log()
  assert "update-branch" not in conductor.log()
  assert conductor.heard == []
  assert conductor.prompts == []
  assert len(git("worktree", "list", cwd=tmp_path, env=conductor.env).splitlines()) == 1


@POSIX
@JQ
async def test_pr_creates_a_new_pr_when_the_branch_held_only_a_merged_one(conductor: Round) -> None:
  branch(conductor)
  got = await conductor.ns["pr"](
    "feat(tui): Show records", "It matters.", ["It works."], CHECKS, where=str(conductor.tmp)
  )
  assert got.code == 0
  log = conductor.log()
  assert "pr create" in log
  assert "pr edit" not in log


@POSIX
@JQ
async def test_a_rebase_that_conflicts_pushes_nothing_and_leaves_the_child_to_a_fixer(
  conductor: Round, tmp_path: Path
) -> None:
  shas = stacked(conductor, tmp_path, clash=True)
  del conductor.prs[9]
  await conductor.land(conductor.prs[7], False)
  assert git("rev-parse", "w/kid", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["w/kid"]
  assert len(conductor.heard) == 1
  assert conductor.heard[0].startswith("#8 was not rebased onto main, exit 2: w/kid conflicts with main")
  assert len(git("worktree", "list", cwd=tmp_path, env=conductor.env).splitlines()) == 1
  kid = conductor.pr(n=8, head="w/kid", mergeable="CONFLICTING", base="main", sha=shas["w/kid"])
  assert await conductor.mend(kid, 1e9, False) == f"#8 {shas['w/kid']} conflicts with its base"
  assert [key for key, _ in conductor.hired] == ["fix8"]


@POSIX
@JQ
async def test_a_child_that_moved_since_the_sweep_is_left_as_it_is(conductor: Round, tmp_path: Path) -> None:
  shas = stacked(conductor, tmp_path)
  conductor.prs[8]["sha"] = "0" * 40
  del conductor.prs[9]
  await conductor.land(conductor.prs[7], False)
  assert git("rev-parse", "w/kid", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["w/kid"]
  assert conductor.heard[0].startswith("#8 was not rebased onto main, exit 4: w/kid moved since the sweep")


@POSIX
@JQ
async def test_a_grandchild_follows_the_rebased_child(conductor: Round, tmp_path: Path) -> None:
  stacked(conductor, tmp_path)
  await conductor.land(conductor.prs[7], False)
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent (#7)", "base"]
  assert history(tmp_path, conductor, "w/deep") == ["deep", "kid", "parent (#7)", "base"]
  assert conductor.heard == []


@POSIX
@JQ
async def test_pr_edits_the_open_pr_of_the_branch(conductor: Round) -> None:
  branch(conductor)
  conductor.say(r"pr list --state open --head w/k", '[{"number": 7}]')
  conductor.say(r"pr view 7 --json url", '{"url": "https://github.com/o/r/pull/7"}')
  got = await conductor.ns["pr"](
    "feat(tui): Show records", "It matters.", ["It works."], CHECKS, where=str(conductor.tmp)
  )
  assert got.code == 0
  log = conductor.log()
  assert "pr edit 7 --title" in log
  assert "pr create" not in log


def viewing(sha: str, mergeable: str = "MERGEABLE") -> str:
  return json.dumps({"state": "OPEN", "headRefOid": sha, "mergeable": mergeable})


VIEW = r"pr view 5 --json state,headRefOid,mergeable"


@POSIX
@JQ
async def test_an_order_is_dropped_when_the_head_moved_before_it_goes(conductor: Round) -> None:
  conductor.say(VIEW, viewing("zzz", "CONFLICTING"))
  assert await conductor.mend(conductor.pr(mergeable="CONFLICTING"), 0.0, False) == ""
  assert (conductor.hired, conductor.prompts) == ([], [])


@POSIX
@JQ
async def test_an_order_is_dropped_when_the_conflict_is_gone_before_it_goes(conductor: Round) -> None:
  conductor.say(VIEW, viewing("abc", "MERGEABLE"))
  assert await conductor.mend(conductor.pr(mergeable="CONFLICTING"), 0.0, False) == ""
  assert (conductor.hired, conductor.prompts) == ([], [])


@POSIX
@JQ
async def test_an_order_goes_when_the_pr_still_holds_its_head_and_its_conflict(conductor: Round) -> None:
  conductor.say(VIEW, viewing("abc", "CONFLICTING"))
  await conductor.mend(conductor.pr(mergeable="CONFLICTING"), 0.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_pr_with_a_fixer_gets_no_order_and_no_second_fixer(conductor: Round) -> None:
  conductor.working["p9"] = "fix5"
  conductor.noticed.add("#5 abc conflicts with its base fixer")
  assert await conductor.mend(conductor.pr(sha="def", mergeable="CONFLICTING"), 0.0, False) == ""
  assert (conductor.hired, conductor.prompts) == ([], [])
  box = conductor.pr(n=8, open=["- [ ] It is one change."])
  conductor.working["p8"] = "box8"
  assert conductor.boxes(box, 0.0) == "#8 abc open boxes"
  assert (conductor.hired, conductor.prompts) == ([], [])


@POSIX
@JQ
async def test_the_round_leaves_the_branch_of_a_pr_with_a_fixer_alone_and_merges_it_not(conductor: Round) -> None:
  conductor.working["p9"] = "fix5"
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  assert await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE")), False) == ""
  await conductor.land(conductor.pr(n=5, checks=("SUCCESS",), names=["the hooks"]), False)
  log = conductor.log()
  assert "update-branch" not in log
  assert "pr merge" not in log


@POSIX
@JQ
async def test_the_worker_that_owns_a_pr_hears_that_the_round_updated_its_branch(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  pr = conductor.pr(checks=("FAILURE", "FAILURE"), labels=["w:k5"])
  assert "updated its branch" in await conductor.heal(pr, False)
  message, on = conductor.prompts[0]
  assert on == "chain-k5"
  assert message.startswith("The round updated the branch of your PR #5 with main. Fetch origin before you push")


@POSIX
@JQ
async def test_a_second_order_for_a_pr_waits_ten_minutes_after_the_first(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  one = {"labels": ["w:k5"], "mergeable": "CONFLICTING"}
  await conductor.mend(conductor.pr(**one), 0.0, False)
  assert len(conductor.prompts) == 1
  await conductor.mend(conductor.pr(sha="def", **one), 100.0, False)
  assert len(conductor.prompts) == 1
  await conductor.mend(conductor.pr(sha="ghi", **one), 700.0, False)
  assert len(conductor.prompts) == 2


@POSIX
@JQ
async def test_a_fixer_that_starts_on_a_pr_with_a_live_worker_tells_that_worker(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  pr = conductor.pr(labels=["w:k5"], mergeable="CONFLICTING")
  await conductor.mend(pr, 0.0, False)
  await conductor.mend(pr, 4000.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"]
  assert conductor.prompts[-1][0].startswith("A fixer starts on your PR #5. Leave its branch alone")


def test_the_brief_of_a_fixer_says_how_to_share_a_branch_and_to_wait_for_github(conductor: Round) -> None:
  for words in ("git worktree list", "If the head of the PR is not {sha} any more", "headRefOid,mergeable", "20 seconds"):
    assert words in conductor.FIX


def test_a_stacked_pr_whose_base_moved_names_the_old_and_the_new_head_in_its_order(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7, head="w/parent", sha="new")
  conductor.prs[8] = conductor.pr(n=8, head="w/kid", base="w/parent", mergeable="CONFLICTING", sha="kid1")
  conductor.tips["w/parent"] = "old"
  conductor.shift()
  assert conductor.shifted == {8: ("old", "new")}
  assert conductor.tips == {"w/parent": "new", "w/kid": "kid1"}
  assert conductor.based(conductor.prs[8]).endswith(
    "whose head changed from old to new: git rebase --onto origin/w/parent old w/kid puts the PR on it."
  )
  assert conductor.based(conductor.pr(n=9)) == ""


@POSIX
@JQ
async def test_the_fixer_of_a_stacked_pr_gets_the_base_that_changed(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7, head="w/parent", sha="new")
  kid = conductor.pr(n=8, head="w/kid", base="w/parent", mergeable="CONFLICTING", sha="kid1")
  conductor.prs[8] = kid
  conductor.tips["w/parent"] = "old"
  conductor.shift()
  await conductor.mend(kid, 0.0, False)
  assert "git rebase --onto origin/w/parent old w/kid" in conductor.hired[0][1]


@POSIX
@JQ
async def test_a_mergeable_that_gh_gives_as_unknown_is_probed_with_git_merge_tree(
  conductor: Round, tmp_path: Path
) -> None:
  shas = stacked(conductor, tmp_path, clash=True)
  kid = conductor.pr(n=8, head="w/kid", mergeable="UNKNOWN", sha=shas["w/kid"])
  await conductor.probe(kid)
  assert kid["mergeable"] == "CONFLICTING"


@POSIX
@JQ
async def test_a_probe_leaves_a_clean_pr_as_unknown(conductor: Round, tmp_path: Path) -> None:
  shas = stacked(conductor, tmp_path)
  kid = conductor.pr(n=8, head="w/kid", mergeable="UNKNOWN", sha=shas["w/kid"])
  await conductor.probe(kid)
  assert kid["mergeable"] == "UNKNOWN"


def push_parent(conductor: Round, tmp_path: Path, force: bool) -> tuple[str, str]:
  env = conductor.env
  git("checkout", "-q", "w/parent", cwd=tmp_path, env=env)
  old = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  if force:
    git("commit", "-q", "--amend", "-m", "parent2", cwd=tmp_path, env=env)
    git("push", "-q", "--force", "origin", "w/parent", cwd=tmp_path, env=env)
  else:
    (tmp_path / "more.txt").write_text("more\n", encoding="utf-8")
    git("add", "more.txt", cwd=tmp_path, env=env)
    git("commit", "-q", "-m", "parent2", cwd=tmp_path, env=env)
    git("push", "-q", "origin", "w/parent", cwd=tmp_path, env=env)
  new = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  git("checkout", "-q", "main", cwd=tmp_path, env=env)
  conductor.prs[7]["sha"] = new
  conductor.shifted[8] = (old, new)
  return old, new


@POSIX
@JQ
async def test_a_child_follows_the_force_push_of_its_parent(conductor: Round, tmp_path: Path) -> None:
  stacked(conductor, tmp_path)
  conductor.prs[8]["labels"] = ["w:k8"]
  conductor.busy("p1", "k8", SONNET)
  push_parent(conductor, tmp_path, True)
  assert await conductor.restack_forced() == {8}
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent2", "base"]
  assert history(tmp_path, conductor, "w/deep") == ["deep", "kid", "parent2", "base"]
  assert 8 not in conductor.shifted
  assert conductor.prompts[0][0].startswith("The round rebased your PR #8 onto w/parent and pushed it.")
  assert await conductor.restack_forced() == set()


@POSIX
@JQ
async def test_a_child_stays_when_its_parent_only_gains_a_commit(conductor: Round, tmp_path: Path) -> None:
  stacked(conductor, tmp_path)
  push_parent(conductor, tmp_path, False)
  assert await conductor.restack_forced() == set()
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent", "base"]


def test_each_brief_holds_the_recipes_that_fixers_found(conductor: Round) -> None:
  for word in ("credential.helper", "actions/jobs", "sed -i ''", "timeout=1800", "hypothesis"):
    assert word in conductor.WORKER
    assert word in conductor.FIX
  assert "Keep the worktree until your PR merges" in conductor.WORKER
