# ruff: noqa: S603, S607
import contextlib
import os
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
  return {**kept, "PATH": f"{tmp / 'shim'}{os.pathsep}{os.environ['PATH']}", **more}


def watch(tmp: Path, **more: str) -> subprocess.CompletedProcess[str]:
  args = ["sh", str(SCRIPT), "--once"]
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
  """A cargo test binary, bun test, a pytest, a daemon of furb, a window, a capture of the gallery, a worker of pytest
  (the largest test process of this user), and a bun test of another user, which is larger still."""
  commands = [
    ("/w/furb/target/debug/deps/furb-1a2b3c4d5e6f7a8b --exact life::test", 800, MINE),
    ("/opt/bun/bin/bun test ./tui/test/walk.test.ts", 2000, MINE),
    ("/w/furb/.venv/bin/python -m pytest -q test/boot", 1000, MINE),
    ("/w/furb/target/debug/furb host --grace 5", 9000, MINE),
    ("/Applications/Terminal.app/Contents/MacOS/Terminal", 6000, MINE),
    ("/opt/bun/bin/bun /w/furb/tui/script/screenshots.ts", 1500, MINE),
    (WORKER, 2500, MINE),
    ("/opt/bun/bin/bun test ./tui/test/other.test.ts", 9999, MINE + 1),
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
    assert (procs[0].poll() is not None) == ended
  assert ("% free, ended pid" in got.stdout) == ended


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
