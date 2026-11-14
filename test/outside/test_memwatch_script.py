# ruff: noqa: S603, S607
import contextlib
import os
import shutil
import signal
import subprocess
import sys
import time
from collections.abc import Iterator
from pathlib import Path

import pytest

SCRIPT = Path(__file__).resolve().parents[2] / "script" / "memwatch.sh"
pytestmark = pytest.mark.skipif(sys.platform == "win32", reason="the script runs in sh")
MB = 1024
# os.getuid is no function of Windows, and the import runs before the mark skips the tests there.
MINE = 0 if sys.platform == "win32" else os.getuid()
Row = tuple[int, int, str, int]


def machine(tmp: Path, level: int, rows: list[Row], *, mac: bool = True) -> None:
  """A fake machine: ps lists the given processes as pid, size in KB, command and user, and on macOS sysctl gives the
  given pressure level of the kernel."""
  shim = tmp / "shim"
  shim.mkdir(exist_ok=True)
  (tmp / "table").write_text("".join(f"{uid} {pid} {kb} {c}\n" for pid, kb, c, uid in rows), encoding="utf-8")
  # A process that ended is no longer listed, as with the real ps.
  lister = f'#!/bin/sh\nwhile read -r uid pid rest; do kill -0 "$pid" 2>/dev/null && echo "$uid $pid $rest"; done < {tmp}/table\n'
  scripts = {"ps": lister}
  if mac:
    scripts["sysctl"] = f"#!/bin/sh\necho {level}\n"
  for name, text in scripts.items():
    (shim / name).write_text(text, encoding="utf-8")
    (shim / name).chmod(0o755)


def env(tmp: Path, **more: str) -> dict[str, str]:
  kept = {n: v for n, v in os.environ.items() if not n.startswith("FURB_WATCH_")}
  log = {"FURB_WATCH_LOG": str(tmp / "watch.log")}
  # A test process that runs away holds 4 GB by default, so the fake processes of most tests, which hold less, run with a
  # size of one KB, and a test of the default gives an empty value.
  return {**kept, "PATH": f"{tmp / 'shim'}{os.pathsep}{os.environ['PATH']}", **log, "FURB_WATCH_MIN": "1", **more}


def watch(tmp: Path, script: Path = SCRIPT, **more: str) -> subprocess.CompletedProcess[str]:
  args = ["sh", str(script), "--once"]
  return subprocess.run(args, env=env(tmp, **more), text=True, capture_output=True, timeout=30, check=False)


@contextlib.contextmanager
def fakes(count: int) -> Iterator[list[subprocess.Popen[bytes]]]:
  """Real processes that the fake ps lists, so that the watcher ends a real pid."""
  procs = [subprocess.Popen(["sleep", "60"]) for _ in range(count)]
  try:
    yield procs
  finally:
    for one in procs:
      one.kill()
      one.wait()


WORKER = "python -u -c import sys;exec(eval(sys.stdin.readline()))"


def rows(procs: list[subprocess.Popen[bytes]]) -> list[Row]:
  """A cargo test binary, bun test, a pytest, a daemon of furb, a window, a second bun test, a worker of pytest (the
  largest test process of this user), and a bun test of another user, which is larger still."""
  commands = [
    ("/w/furb/target/debug/deps/furb-1a2b3c4d5e6f7a8b --exact life::test", 800, MINE),
    ("/opt/bun/bin/bun test ./bind/typescript/test/session.test.ts", 2000, MINE),
    ("/w/furb/.venv/bin/python -m pytest -q test/boot", 1000, MINE),
    ("/w/furb/target/debug/furb host --grace 5", 9000, MINE),
    ("/Applications/Terminal.app/Contents/MacOS/Terminal", 6000, MINE),
    ("/opt/bun/bin/bun test ./bind/typescript/test/engine.test.ts", 1500, MINE),
    (WORKER, 2500, MINE),
    ("/opt/bun/bin/bun test ./bind/typescript/test/other.test.ts", 9999, MINE + 1),
  ]
  return [(one.pid, size * MB, command, uid) for one, (command, size, uid) in zip(procs, commands, strict=True)]


def test_a_tight_memory_ends_the_largest_test_process_of_the_user_and_no_other_process(tmp_path: Path) -> None:
  with fakes(8) as procs:
    machine(tmp_path, 2, rows(procs))
    got = watch(tmp_path)
    assert procs[6].wait(timeout=10) == -signal.SIGKILL
    assert [one.poll() for one in procs if one is not procs[6]] == [None] * 7
  assert got.returncode == 0
  assert got.stdout == f"memwatch: pressure level 2, ended pid {procs[6].pid} (2500 MB): {WORKER}\n"


def test_a_memory_that_is_not_tight_ends_nothing(tmp_path: Path) -> None:
  with fakes(8) as procs:
    machine(tmp_path, 1, rows(procs))
    got = watch(tmp_path)
    assert [one.poll() for one in procs] == [None] * 8
  assert (got.returncode, got.stdout) == (0, "")


def test_a_machine_that_gives_no_reading_is_never_tight(tmp_path: Path) -> None:
  with fakes(1) as procs:
    machine(tmp_path, 4, [(procs[0].pid, 500 * MB, "bun test ./a.test.ts", MINE)])
    (tmp_path / "shim" / "sysctl").write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
    got = watch(tmp_path, FURB_WATCH_FREE="0")
    assert procs[0].poll() is None
  assert got.stdout == ""


def test_a_tight_memory_with_no_test_process_ends_nothing_and_says_so(tmp_path: Path) -> None:
  with fakes(2) as procs:
    machine(
      tmp_path,
      4,
      [(procs[0].pid, 9000 * MB, "/w/furb/target/debug/furb host", MINE), (procs[1].pid, 5000 * MB, "Mail", MINE)],
    )
    got = watch(tmp_path)
    assert [one.poll() for one in procs] == [None] * 2
  assert got.stdout == "memwatch: pressure level 4, and no test process to end\n"


@pytest.mark.skipif(sys.platform != "linux", reason="the share of the memory is the rule of Linux")
@pytest.mark.parametrize(("floor", "ended"), [("101", True), ("0", False)])
def test_on_linux_the_memory_is_tight_under_its_share_of_free_memory(
  tmp_path: Path, floor: str, *, ended: bool
) -> None:
  with fakes(1) as procs:
    machine(tmp_path, 0, [(procs[0].pid, 500 * MB, "bun test ./a.test.ts", MINE)], mac=False)
    got = watch(tmp_path, FURB_WATCH_FREE=floor)
    assert ("% free, ended pid" in got.stdout) == ended, got.stdout
    # The kill is a signal, which the process takes when it next runs, so an ended process is waited for, not polled.
    if ended:
      assert procs[0].wait(timeout=10) == -signal.SIGKILL
    else:
      assert procs[0].poll() is None


def test_the_loop_ends_one_test_process_at_each_look_while_the_memory_stays_tight(tmp_path: Path) -> None:
  with fakes(2) as procs:
    commands = ["bun test ./a.test.ts", "bun test ./b.test.ts"]
    machine(
      tmp_path, 2, [(one.pid, size * MB, c, MINE) for one, size, c in zip(procs, (900, 300), commands, strict=True)]
    )
    loop = subprocess.Popen(
      ["sh", str(SCRIPT)], env=env(tmp_path, FURB_WATCH_POLL="0.05", FURB_WATCH_AFTER="0.05"), stdout=subprocess.PIPE
    )
    try:
      deadline = time.monotonic() + 15
      while any(one.poll() is None for one in procs) and time.monotonic() < deadline:
        time.sleep(0.05)
      assert [one.poll() for one in procs] == [-signal.SIGKILL] * 2
    finally:
      loop.kill()
      loop.communicate()


def test_a_linker_that_writes_a_test_binary_is_no_test_process(tmp_path: Path) -> None:
  link = (
    "/usr/bin/ld -demangle -o /w/target/debug/deps/furb-3bfcea354560c7bc /w/target/debug/deps/furb-3bfcea354560c7bc.o"
  )
  with fakes(2) as procs:
    machine(
      tmp_path,
      2,
      [
        (procs[0].pid, 2500 * MB, link, MINE),
        (procs[1].pid, 600 * MB, "/w/target/debug/deps/furb-3bfcea354560c7bc --test-threads 4", MINE),
      ],
    )
    got = watch(tmp_path)
  assert (
    got.stdout
    == f"memwatch: pressure level 2, ended pid {procs[1].pid} (600 MB): /w/target/debug/deps/furb-3bfcea354560c7bc --test-threads 4\n"
  )


def test_a_hook_or_a_shell_that_names_pytest_is_no_test_process(tmp_path: Path) -> None:
  pytest_run = "/w/furb/.venv/bin/python3 /w/furb/.venv/bin/pytest -q test/boot"
  drivers = ["python3 script/budget.py pytest -- uv run pytest -q", "sh -c uv run pytest -q", "uv run pytest -q"]
  with fakes(4) as procs:
    rows = [
      (one.pid, (3000 - n) * MB, command, MINE) for n, (one, command) in enumerate(zip(procs, drivers, strict=False))
    ]
    machine(tmp_path, 2, [*rows, (procs[3].pid, 400 * MB, pytest_run, MINE)])
    got = watch(tmp_path)
  assert got.stdout == f"memwatch: pressure level 2, ended pid {procs[3].pid} (400 MB): {pytest_run}\n"


def test_each_process_that_it_ends_is_a_line_of_its_log_with_its_time_and_pid(tmp_path: Path) -> None:
  with fakes(1) as procs:
    machine(tmp_path, 2, [(procs[0].pid, 500 * MB, "bun test ./a.test.ts", MINE)])
    before = int(time.time())
    watch(tmp_path)
  when, pid, command = (tmp_path / "watch.log").read_text(encoding="utf-8").rstrip("\n").split(" ", 2)
  assert before <= int(when) <= time.time() and int(pid) == procs[0].pid and command == "bun test ./a.test.ts"


def clone(tmp: Path) -> tuple[Path, Path]:
  """A git clone that holds the script, with a second tree beside it: the script to run, and the second tree."""
  root = (tmp / "clone").resolve()
  (root / "script").mkdir(parents=True)
  shutil.copy(SCRIPT, root / "script" / "memwatch.sh")
  git = ["git", "-C", str(root), "-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"]
  git += ["-c", f"core.hooksPath={os.devnull}"]
  subprocess.run(["git", "init", "-q", str(root)], check=True)
  subprocess.run([*git, "add", "-A"], check=True)
  subprocess.run([*git, "commit", "-qm", "x"], check=True)
  other = (tmp / "other").resolve()
  subprocess.run([*git, "worktree", "add", "-q", str(other), "-b", "other"], check=True)
  return root / "script" / "memwatch.sh", other


def test_a_tight_memory_that_the_work_does_not_hold_ends_no_test_process_and_says_so(tmp_path: Path) -> None:
  with fakes(3) as procs:
    machine(
      tmp_path,
      2,
      [
        (procs[0].pid, 300 * MB, "bun test ./a.test.ts", MINE),
        (procs[1].pid, 20000 * MB, "Chrome", MINE),
        (procs[2].pid, 9000 * MB, "/elsewhere/furb/target/debug/rustc --crate-name y", MINE),
      ],
    )
    got = watch(tmp_path, FURB_WATCH_MIN="")
    assert [one.poll() for one in procs] == [None] * 3
  said = "the work is not the cause (it holds 300 MB, its largest test process 300 MB): none ended"
  assert got.stdout == f"memwatch: pressure level 2, and {said}\n"


def test_a_test_process_under_4_gb_is_no_runaway_while_the_work_holds_little(tmp_path: Path) -> None:
  with fakes(1) as procs:
    machine(tmp_path, 2, [(procs[0].pid, 1500 * MB, "/opt/bun/bin/bun test ./a.test.ts", MINE)])
    got = watch(tmp_path, FURB_WATCH_MIN="")
    assert procs[0].poll() is None
  assert "the work is not the cause (it holds 1500 MB, its largest test process 1500 MB): none ended" in got.stdout


def test_a_test_process_that_holds_4_gb_is_ended_by_default(tmp_path: Path) -> None:
  with fakes(1) as procs:
    machine(tmp_path, 2, [(procs[0].pid, 4500 * MB, "bun test ./a.test.ts", MINE)])
    got = watch(tmp_path, FURB_WATCH_MIN="")
    assert procs[0].wait(timeout=10) == -signal.SIGKILL
  assert got.stdout == f"memwatch: pressure level 2, ended pid {procs[0].pid} (4500 MB): bun test ./a.test.ts\n"


def test_a_work_that_holds_the_size_of_its_limit_ends_the_largest_test_process(tmp_path: Path) -> None:
  with fakes(1) as procs:
    machine(tmp_path, 2, [(procs[0].pid, 300 * MB, "bun test ./a.test.ts", MINE)])
    got = watch(tmp_path, FURB_WATCH_MIN="", FURB_WATCH_LANE="1")
    assert procs[0].wait(timeout=10) == -signal.SIGKILL
  assert got.stdout == f"memwatch: pressure level 2, ended pid {procs[0].pid} (300 MB): bun test ./a.test.ts\n"


@pytest.mark.parametrize(("size", "ended"), [(3000, True), (2000, False)])
def test_the_work_is_each_test_process_and_each_process_that_names_a_tree_of_the_clone(
  tmp_path: Path, size: int, *, ended: bool
) -> None:
  script, other = clone(tmp_path)
  names = [
    f"{other}/target/debug/deps/furb-0123456789abcdef --test-threads 4",
    f"{other}/node_modules/.bin/esbuild --serve",
    f"{script.parents[1]}/target/debug/rustc --crate-name x",
  ]
  with fakes(3) as procs:
    machine(tmp_path, 2, [(one.pid, size * MB, name, MINE) for one, name in zip(procs, names, strict=True)])
    got = watch(tmp_path, script, FURB_WATCH_MIN="")
    if ended:
      assert procs[0].wait(timeout=10) == -signal.SIGKILL
      assert [one.poll() for one in procs[1:]] == [None] * 2
    else:
      assert [one.poll() for one in procs] == [None] * 3
  assert ("ended pid" in got.stdout) == ended, got.stdout


@pytest.mark.parametrize(
  ("name", "ended"), [("target/debug/furb host", False), ("target/debug/rustc --crate-name x", True)]
)
def test_a_daemon_of_furb_is_no_part_of_the_work_since_no_end_of_a_test_process_frees_it(
  tmp_path: Path, name: str, *, ended: bool
) -> None:
  script, other = clone(tmp_path)
  names = [
    "bun test bind/typescript/test/engine.test.ts -t beside",
    f"{other}/{name}",
    f"{other}/target/debug/furb turns --record /x/r.jsonl",
  ]
  sizes = (360, 4600, 4100)
  with fakes(3) as procs:
    machine(tmp_path, 2, [(one.pid, size * MB, c, MINE) for one, size, c in zip(procs, sizes, names, strict=True)])
    got = watch(tmp_path, script, FURB_WATCH_MIN="")
    if ended:
      assert procs[0].wait(timeout=10) == -signal.SIGKILL
    else:
      assert [one.poll() for one in procs] == [None] * 3
  assert ("ended pid" in got.stdout) == ended, got.stdout
