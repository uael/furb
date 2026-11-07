# ruff: noqa: S603, S607
import os
import subprocess
from pathlib import Path

HOOK = Path(__file__).resolve().parents[2] / "script" / "hooks" / "pre-commit"
# A commit hook runs this suite with GIT_INDEX_FILE and more set, so a git command that inherits them acts on the repo
# that is committing and not on the repo of the test.
# The same hook sets PRE_COMMIT=1, which the script reads as the command that runs the hooks.
CLEAN = {key: value for key, value in os.environ.items() if not key.startswith("GIT_") and key != "PRE_COMMIT"}


def run(tmp_path: Path, body: str, *args: str) -> subprocess.CompletedProcess[str]:
  stub = tmp_path / "stub.sh"
  stub.write_text("#!/bin/sh\n" + body)
  stub.chmod(0o755)
  subprocess.run(["git", "init", "-q", str(tmp_path / "repo")], check=True, env=CLEAN)
  env = {**CLEAN, "PRE_COMMIT": stub.as_posix(), "TMPDIR": tmp_path.as_posix()}
  return subprocess.run(
    ["sh", HOOK.as_posix(), *args], cwd=tmp_path / "repo", env=env, capture_output=True, text=True, check=False
  )


def test_hook_prints_nothing_when_every_hook_passes(tmp_path: Path):
  got = run(tmp_path, "echo 'ruff-check...........Passed'\necho 'fmt...(no files to check)Skipped'\n")
  assert (got.returncode, got.stdout, got.stderr) == (0, "", "")


def test_hook_prints_only_what_fails(tmp_path: Path):
  got = run(
    tmp_path, "echo 'ruff-check...........Passed'\necho 'pytest...........Failed'\necho 'E assert 1 == 2'\nexit 1\n"
  )
  assert got.returncode == 1
  assert got.stdout == "pytest...........Failed\nE assert 1 == 2\n"


def merging(
  tmp_path: Path, *, conflict: bool, both: bool = False, tag: bool = False
) -> tuple[subprocess.CompletedProcess[str], Path, Path]:
  """A merge of a branch that adds bad.txt and deletes gone.txt, run through the hooks, with a stub that lists the files
  that it checks and fails on each one. With conflict, both branches change a.txt on one line; with both, they change
  b.txt on two lines apart, which git merges. With tag, it merges an annotated tag of the branch."""
  stub = tmp_path / "stub.sh"
  seen = tmp_path / "seen.txt"
  stub.write_text(
    "#!/bin/sh\n"
    'while [ "$#" -gt 0 ] && [ "$1" != --files ]; do shift; done\n'
    '[ "$#" -eq 0 ] || shift\n'
    f'printf "%s\\n" "$@" >"{seen.as_posix()}"\necho "check...........Failed"\nexit 1\n'
  )
  stub.chmod(0o755)
  repo = tmp_path / "repo"
  env = {**CLEAN, "PRE_COMMIT": stub.as_posix(), "TMPDIR": tmp_path.as_posix()}
  config = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", f"core.hooksPath={HOOK.parent.as_posix()}"]

  def git(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *config, *args], cwd=repo, env=env, capture_output=True, text=True, check=check)

  subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True, env=CLEAN)
  (repo / "a.txt").write_text("a\n")
  (repo / "gone.txt").write_text("x\n")
  (repo / "b.txt").write_text("1\n2\n3\n4\n5\n")
  git("add", ".")
  git("commit", "-q", "--no-verify", "-m", "base")
  git("switch", "-q", "-c", "side")
  (repo / "bad.txt").write_text("x\n")
  (repo / "gone.txt").unlink()
  (repo / "a.txt").write_text("side\n" if conflict else "a\n")
  (repo / "b.txt").write_text("side\n2\n3\n4\n5\n" if both else "1\n2\n3\n4\n5\n")
  git("add", "-A")
  git("commit", "-q", "--no-verify", "-m", "side")
  git("tag", "-a", "-m", "v", "v1", "side")
  git("switch", "-q", "main")
  (repo / "c.txt").write_text("x\n")
  (repo / "a.txt").write_text("main\n" if conflict else "a\n")
  (repo / "b.txt").write_text("1\n2\n3\n4\nmain\n" if both else "1\n2\n3\n4\n5\n")
  git("add", ".")
  git("commit", "-q", "--no-verify", "-m", "other")
  merged = git("merge", "--no-ff", "v1" if tag else "side", "-m", "chore(ci): Merge the side branch", check=False)
  if conflict:
    (repo / "a.txt").write_text("resolved\n")
    git("add", "a.txt")
    merged = git("commit", "--no-edit", check=False)
  return merged, seen, repo


def parents(repo: Path) -> int:
  head = subprocess.run(
    ["git", "rev-list", "--parents", "-1", "HEAD"], cwd=repo, env=CLEAN, capture_output=True, text=True, check=False
  )
  return len(head.stdout.split()) - 1


def test_hook_checks_no_file_that_a_merge_takes_whole_from_one_parent(tmp_path: Path):
  merged, seen, repo = merging(tmp_path, conflict=False)
  assert merged.returncode == 0, merged.stderr
  assert not seen.exists()
  assert parents(repo) == 2


def test_hook_checks_no_file_that_a_merge_of_an_annotated_tag_takes_whole(tmp_path: Path):
  merged, seen, repo = merging(tmp_path, conflict=False, tag=True)
  assert merged.returncode == 0, merged.stderr
  assert not seen.exists()
  assert parents(repo) == 2


def test_hook_gates_a_merge_that_git_makes_with_no_conflict_on_each_file_that_it_merges(tmp_path: Path):
  merged, seen, repo = merging(tmp_path, conflict=False, both=True)
  assert merged.returncode != 0
  assert seen.read_text().splitlines() == ["b.txt"]
  assert parents(repo) == 1


def test_hook_gates_the_commit_of_a_merge_on_each_file_whose_content_differs_from_every_parent(tmp_path: Path):
  merged, seen, repo = merging(tmp_path, conflict=True)
  assert merged.returncode != 0
  assert seen.read_text().splitlines() == ["a.txt"]
  assert "check...........Failed" in merged.stdout + merged.stderr
  assert parents(repo) == 1


def test_hook_runs_pre_commit_by_hand_with_the_arguments_that_it_is_given(tmp_path: Path):
  got = run(tmp_path, 'echo "$@"\n', "run", "--files", "a.py")
  assert (got.returncode, got.stdout) == (0, "run --files a.py\n")


def test_hook_takes_the_venv_of_the_main_checkout_in_a_worktree_that_has_none(tmp_path: Path):
  main = tmp_path / "main"
  subprocess.run(["git", "init", "-q", "-b", "main", str(main)], check=True, env=CLEAN)
  python = main / ".venv" / "bin" / "python"
  python.parent.mkdir(parents=True)
  python.write_text('#!/bin/sh\n[ "$1" = -c ] && exit 0\necho "main venv $*"\nexit 1\n')
  python.chmod(0o755)
  config = ["-c", "user.name=t", "-c", "user.email=t@t"]
  subprocess.run(["git", *config, "commit", "-q", "--allow-empty", "-m", "x"], cwd=main, check=True, env=CLEAN)
  tree = tmp_path / "tree"
  subprocess.run(["git", "worktree", "add", "-q", "-b", "side", str(tree)], cwd=main, check=True, env=CLEAN)
  env = {**CLEAN, "TMPDIR": tmp_path.as_posix()}
  got = subprocess.run(["sh", HOOK.as_posix()], cwd=tree, env=env, capture_output=True, text=True, check=False)
  assert got.returncode == 1, got.stderr
  assert got.stdout.startswith("main venv -mpre_commit hook-impl")


PUSH = HOOK.parent / "pre-push"


def pushing(
  tmp_path: Path, *refs: str, verify: bool = False, name: str = "a.txt", amend: bool = False, data: bool = False
) -> tuple[subprocess.CompletedProcess[str], str, list[str]]:
  """Pushes of a clone with the hooks of the repository, to each ref named, after one commit of a file that skipped its
  hooks or ran them, and with amend a second change that runs them and amends it, and with data the push of a commit of
  captures that git commit-tree makes with no history and no hook, beside origin/main. A stub runs the hooks: it logs each run and fails on bad.txt. Gives the push, the branches of
  origin after it, and the runs of the hooks."""
  origin, repo = tmp_path / "origin.git", tmp_path / "repo"
  stub, seen = tmp_path / "stub.sh", tmp_path / "seen.txt"
  stub.write_text(
    f'#!/bin/sh\necho "$*" >>"{seen.as_posix()}"\n'
    'for one; do [ "$one" = bad.txt ] && echo "check...........Failed" && exit 1; done\nexit 0\n'
  )
  stub.chmod(0o755)
  subprocess.run(["git", "init", "-q", "--bare", "-b", "main", str(origin)], check=True, env=CLEAN)
  subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True, env=CLEAN)
  env = {**CLEAN, "PRE_COMMIT": stub.as_posix(), "TMPDIR": tmp_path.as_posix()}
  config = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", f"core.hooksPath={PUSH.parent.as_posix()}"]

  def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *config, *args], cwd=repo, env=env, capture_output=True, text=True, check=False)

  (repo / name).write_text("a\n")
  git("add", ".")
  git("commit", "-q", *([] if verify else ["--no-verify"]), "-m", "chore(ci): Add a file")
  if amend:
    (repo / "c.txt").write_text("c\n")
    git("add", "c.txt")
    git("commit", "-q", "--amend", "--no-edit")
  git("remote", "add", "origin", origin.as_posix())
  tip = "HEAD"
  if data:
    git("update-ref", "refs/remotes/origin/main", "HEAD")
    (repo / "capture.png").write_bytes(b"\x89PNG\r\n")
    blob = git("hash-object", "-w", "capture.png").stdout.strip()
    tree = subprocess.run(
      ["git", "mktree"],
      input=f"100644 blob {blob}\tcapture.png\n",
      cwd=repo,
      env=env,
      capture_output=True,
      text=True,
      check=True,
    ).stdout.strip()
    tip = git("commit-tree", tree, "-m", "The screens of w/converge").stdout.strip()
  got = git("push", "origin", *[f"{tip}:{one}" for one in refs])
  heads = subprocess.run(
    ["git", "for-each-ref", "--format=%(refname)"], cwd=origin, env=CLEAN, capture_output=True, text=True, check=True
  )
  return got, heads.stdout, seen.read_text().splitlines() if seen.exists() else []


def test_a_push_to_main_is_refused_and_moves_nothing(tmp_path: Path):
  got, heads, _ = pushing(tmp_path, "refs/heads/w/mine", "refs/heads/main")
  assert got.returncode != 0 and heads == ""
  assert "The push to main of origin is refused: main changes only through a pull request." in got.stderr


def test_a_push_of_a_branch_of_its_own_goes_through(tmp_path: Path):
  got, heads, _ = pushing(tmp_path, "refs/heads/w/mine")
  assert got.returncode == 0 and heads == "refs/heads/w/mine\n"


def test_a_push_of_a_commit_that_skipped_its_hooks_runs_them_and_is_refused_when_they_fail(tmp_path: Path):
  got, heads, runs = pushing(tmp_path, "refs/heads/w/mine", name="bad.txt")
  assert got.returncode != 0 and heads == ""
  assert "check...........Failed" in got.stderr
  assert len(runs) == 1 and runs[0].endswith("--files bad.txt")


def test_a_push_of_a_commit_that_an_amend_with_hooks_made_from_one_that_skipped_them_runs_them_again(tmp_path: Path):
  got, heads, runs = pushing(tmp_path, "refs/heads/w/mine", name="bad.txt", amend=True)
  assert got.returncode != 0 and heads == ""
  assert runs[-1].endswith("--files bad.txt c.txt")


def test_a_push_of_a_commit_that_ran_its_hooks_runs_them_no_second_time(tmp_path: Path):
  got, heads, runs = pushing(tmp_path, "refs/heads/w/mine", verify=True)
  assert got.returncode == 0 and heads == "refs/heads/w/mine\n"
  assert len(runs) == 1 and runs[0].startswith("hook-impl")


def test_a_push_of_a_commit_with_no_history_beside_main_goes_through_and_runs_no_hook(tmp_path: Path):
  got, heads, runs = pushing(tmp_path, "refs/heads/w/shots", name="bad.txt", data=True)
  assert got.returncode == 0 and heads == "refs/heads/w/shots\n"
  assert runs == []
