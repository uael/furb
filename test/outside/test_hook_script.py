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


def merging(tmp_path: Path, *, conflict: bool) -> tuple[subprocess.CompletedProcess[str], Path, Path]:
  """A merge of a branch that adds bad.txt, run through the hooks, with a stub that lists what the merge changes."""
  stub = tmp_path / "stub.sh"
  seen = tmp_path / "seen.txt"
  stub.write_text(
    "#!/bin/sh\n"
    'for one; do case "$one" in --to-ref=*) to=${one#--to-ref=};; esac; done\n'
    f'[ -n "$to" ] || exit 0\ngit diff --name-only HEAD "$to" >"{seen.as_posix()}"\n'
    f'grep -q bad.txt "{seen.as_posix()}" && echo "nobad...........Failed" && exit 1\nexit 0\n'
  )
  stub.chmod(0o755)
  repo = tmp_path / "repo"
  env = {**CLEAN, "PRE_COMMIT": stub.as_posix(), "TMPDIR": tmp_path.as_posix()}
  config = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", f"core.hooksPath={HOOK.parent.as_posix()}"]

  def git(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *config, *args], cwd=repo, env=env, capture_output=True, text=True, check=check)

  subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True, env=CLEAN)
  (repo / "a.txt").write_text("a\n")
  git("add", ".")
  git("commit", "-q", "--no-verify", "-m", "base")
  git("switch", "-q", "-c", "side")
  (repo / "bad.txt").write_text("x\n")
  (repo / "a.txt").write_text("side\n" if conflict else "a\n")
  git("add", ".")
  git("commit", "-q", "--no-verify", "-m", "side")
  git("switch", "-q", "main")
  (repo / "c.txt").write_text("x\n")
  (repo / "a.txt").write_text("main\n" if conflict else "a\n")
  git("add", ".")
  git("commit", "-q", "--no-verify", "-m", "other")
  merged = git("merge", "--no-ff", "side", "-m", "merge", check=False)
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


def test_hook_gates_a_merge_that_git_makes_with_no_conflict_on_each_file_that_it_changes(tmp_path: Path):
  merged, seen, repo = merging(tmp_path, conflict=False)
  assert merged.returncode != 0
  assert seen.read_text().splitlines() == ["bad.txt"]
  assert "nobad...........Failed" in merged.stderr
  assert parents(repo) == 1


def test_hook_gates_the_commit_of_a_merge_with_a_conflict_on_each_file_that_it_changes(tmp_path: Path):
  merged, seen, repo = merging(tmp_path, conflict=True)
  assert merged.returncode != 0
  assert sorted(seen.read_text().splitlines()) == ["a.txt", "bad.txt"]
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


def pushing(tmp_path: Path, *refs: str) -> tuple[subprocess.CompletedProcess[str], str]:
  """Pushes of a clone with the hooks of the repository, to each ref named, and the branches of origin after them."""
  origin, repo = tmp_path / "origin.git", tmp_path / "repo"
  subprocess.run(["git", "init", "-q", "--bare", "-b", "main", str(origin)], check=True, env=CLEAN)
  subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True, env=CLEAN)
  config = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", f"core.hooksPath={PUSH.parent.as_posix()}"]

  def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *config, *args], cwd=repo, env=CLEAN, capture_output=True, text=True, check=False)

  (repo / "a.txt").write_text("a\n")
  git("add", ".")
  git("commit", "-q", "--no-verify", "-m", "base")
  git("remote", "add", "origin", origin.as_posix())
  got = git("push", "origin", *[f"HEAD:{one}" for one in refs])
  heads = subprocess.run(
    ["git", "for-each-ref", "--format=%(refname)"], cwd=origin, env=CLEAN, capture_output=True, text=True, check=True
  )
  return got, heads.stdout


def test_a_push_to_main_is_refused_and_moves_nothing(tmp_path: Path):
  got, heads = pushing(tmp_path, "refs/heads/w/mine", "refs/heads/main")
  assert got.returncode != 0 and heads == ""
  assert "The push to main of origin is refused: main changes only through a pull request." in got.stderr


def test_a_push_of_a_branch_of_its_own_goes_through(tmp_path: Path):
  got, heads = pushing(tmp_path, "refs/heads/w/mine")
  assert got.returncode == 0 and heads == "refs/heads/w/mine\n"
