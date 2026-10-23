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
  def __init__(self, tmp: Path) -> None:
    self.tmp, self.now = tmp, 1000.0
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
      **os.environ,
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
    ns.update(transcript=lambda on="", of="": [], paused=lambda name: False, scope=lambda name: "chain1")
    ns.update(acting=lambda: "rung1", wake=lambda name: None)
    exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), ns)  # noqa: S102
    ns.update(quietly=lambda verb, *args, **words: verb(*args, **words), hire=self.hire)
    self.ns = ns

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
    return f"prompt{100 + len(self.prompts)}"

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
    return name

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
  program = conductor.PRS.split("--jq '", 1)[1].split("'\necho ---", 1)[0]
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
  conductor.queue[:] = [("a", "", ""), ("b", "", "")]
  conductor.backlog[:] = [("c", "", ""), ("d", "", ""), ("e", "", "")]
  conductor.prs[7] = conductor.pr(n=7, checks=("FAILURE",))
  conductor.staff()
  assert conductor.hired == []
  conductor.working["prompt9"] = "fix7"
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["a", "b", "c"]
  assert conductor.backlog == [("d", "", ""), ("e", "", "")]


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
  conductor.say(r"runs\?per_page=100", json.dumps({"workflow_runs": flaky}))
  conductor.say(r"runs/50/attempts/1/jobs", json.dumps({"jobs": [job]}))
  conductor.say(r"jobs/7/logs", "2026-10-23T10:00:30.0000000Z (fail) a host test [30000.17ms]\n")
  await conductor.watch_ci()
  await conductor.watch_ci()
  assert [(key, thread) for key, _, thread in conductor.queue] == [("flake-a-host-test", "ci")]
  assert f"{url}/attempts/1" in conductor.queue[0][1]
  assert "a host test" in conductor.queue[0][1]


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
  conductor.say(r"runs\?per_page=100", '{"workflow_runs": []}')
  await conductor.watch_ci()
  await conductor.watch_ci()
  assert [(key, thread) for key, _, thread in conductor.queue] == [("budget-the-hooks", "ci")]
  assert "took 180 s" in conductor.queue[0][1]
  assert "budget of 60 s" in conductor.queue[0][1]
