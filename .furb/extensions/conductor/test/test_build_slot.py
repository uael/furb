import os
import subprocess
import sys
import time
from pathlib import Path
from typing import IO

import pytest

SCRIPT = Path(__file__).resolve().parents[4] / "script" / "build-slot.sh"
pytestmark = pytest.mark.skipif(sys.platform == "win32", reason="the script runs in sh")


def env(tmp: Path, **more: str) -> dict[str, str]:
  kept = {n: v for n, v in os.environ.items() if not n.startswith(("FURB_BUILD_", "CARGO_BUILD_JOBS"))}
  return {**kept, "FURB_BUILD_DIR": str(tmp / "slots"), "FURB_BUILD_POLL": "0.05", **more}


def start(tmp: Path, *command: str, err: IO[str] | int = subprocess.PIPE, **more: str) -> subprocess.Popen[str]:
  pipe = subprocess.PIPE
  return subprocess.Popen(["sh", str(SCRIPT), *command], env=env(tmp, **more), text=True, stdout=pipe, stderr=err)


def run(tmp: Path, *command: str, **more: str) -> subprocess.CompletedProcess[str]:
  got = subprocess.run(["sh", str(SCRIPT), *command], env=env(tmp, **more), text=True, capture_output=True, timeout=60)
  return got


def test_a_call_with_no_command_exits_2_and_says_its_use(tmp_path: Path) -> None:
  got = run(tmp_path)
  assert got.returncode == 2 and "usage: sh script/build-slot.sh" in got.stderr


@pytest.mark.parametrize(("given", "seen"), [(None, "4"), ("2", "2"), ("14", "4")])
def test_cargo_jobs_stay_at_four_or_the_lower_cap_of_the_caller(tmp_path: Path, given: str | None, seen: str) -> None:
  more = {} if given is None else {"CARGO_BUILD_JOBS": given}
  got = run(tmp_path, "sh", "-c", "echo $CARGO_BUILD_JOBS; exit 7", **more)
  assert (got.returncode, got.stdout) == (7, seen + "\n")


def test_each_slot_goes_back_when_its_command_ends(tmp_path: Path) -> None:
  assert run(tmp_path, "true").returncode == 0
  assert list((tmp_path / "slots").iterdir()) == []


def entered(log: Path) -> int:
  return log.read_text(encoding="utf-8").split().count("in") if log.exists() else 0


def waiting(errs: list[Path]) -> int:
  return sum("for a slot, held by" in e.read_text(encoding="utf-8") for e in errs if e.exists())


def peak_of(tmp: Path, slots: int, **more: str) -> int:
  # Each command holds until the test releases it, so the peak does not depend on how fast the machine starts them: the
  # test waits for the slots to fill and for each extra build to log that the script holds it, then lets all end.
  log, release = tmp / "log", tmp / "release"
  body = f"echo in >> {log}; while [ ! -e {release} ]; do sleep 0.05; done; echo out >> {log}"
  count = slots + 3
  errs = [tmp / f"err{n}" for n in range(count)]
  files = [e.open("w", encoding="utf-8") for e in errs]
  runs = [start(tmp, "sh", "-c", body, err=f, **more) for f in files]
  try:
    deadline = time.monotonic() + 30
    while (entered(log) < slots or waiting(errs) < count - slots) and time.monotonic() < deadline:
      time.sleep(0.05)
    assert entered(log) == slots and waiting(errs) == count - slots
  finally:
    release.touch()
    for f in files:
      f.close()
  for one in runs:
    one.communicate(timeout=60)
  assert [one.returncode for one in runs] == [0] * count
  held = peak = 0
  for line in log.read_text(encoding="utf-8").split():
    held += 1 if line == "in" else -1
    peak = max(peak, held)
  return peak


def test_no_more_commands_run_at_once_than_the_slots(tmp_path: Path) -> None:
  assert peak_of(tmp_path, 2, FURB_BUILD_SLOTS="2") == 2


@pytest.mark.parametrize(("memory", "slots"), [("36", 4), ("27", 3), ("24", 2), ("8", 1)])
def test_the_slots_default_to_one_for_each_9_gb_of_memory_and_at_least_one(
  tmp_path: Path, memory: str, slots: int
) -> None:
  assert peak_of(tmp_path, slots, FURB_BUILD_MEM_GB=memory) == slots


def test_the_slot_of_a_holder_that_ended_goes_to_the_next_build(tmp_path: Path) -> None:
  gone = subprocess.Popen(["true"])
  gone.wait()
  folder = tmp_path / "slots" / "slot1"
  folder.mkdir(parents=True)
  (folder / "pid").write_text(f"{gone.pid}\n", encoding="utf-8")
  assert run(tmp_path, "true", FURB_BUILD_SLOTS="1", FURB_BUILD_WAIT="5").returncode == 0


def test_the_slot_of_a_live_holder_holds_until_the_wait_ends(tmp_path: Path) -> None:
  folder = tmp_path / "slots" / "slot1"
  release = tmp_path / "release"
  body = f"while [ ! -e {release} ]; do sleep 0.05; done"
  holder = start(tmp_path, "sh", "-c", body, FURB_BUILD_SLOTS="1")
  try:
    deadline = time.monotonic() + 30
    while not (folder / "pid").exists() and time.monotonic() < deadline:
      time.sleep(0.05)
    got = run(tmp_path, "true", FURB_BUILD_SLOTS="1", FURB_BUILD_WAIT="1")
    assert got.returncode == 75 and "no slot of 1 freed in 1s" in got.stderr
    assert folder.is_dir()
  finally:
    release.touch()
    holder.communicate(timeout=60)


def test_the_slot_whose_pid_went_to_another_process_goes_to_the_next_build(tmp_path: Path) -> None:
  folder = tmp_path / "slots" / "slot1"
  folder.mkdir(parents=True)
  (folder / "pid").write_text(f"{os.getpid()}\n", encoding="utf-8")
  assert run(tmp_path, "true", FURB_BUILD_SLOTS="1", FURB_BUILD_WAIT="5").returncode == 0


def test_a_slot_with_no_pid_frees_after_a_minute_and_not_before(tmp_path: Path) -> None:
  folder = tmp_path / "slots" / "slot1"
  folder.mkdir(parents=True)
  assert run(tmp_path, "true", FURB_BUILD_SLOTS="1", FURB_BUILD_WAIT="1").returncode == 75
  old = time.time() - 120
  os.utime(folder, (old, old))
  assert run(tmp_path, "true", FURB_BUILD_SLOTS="1", FURB_BUILD_WAIT="5").returncode == 0


def test_a_build_inside_a_slot_takes_no_second_slot(tmp_path: Path) -> None:
  got = run(tmp_path, "sh", str(SCRIPT), "echo", "inner", FURB_BUILD_SLOTS="1", FURB_BUILD_WAIT="3")
  assert (got.returncode, got.stdout) == (0, "inner\n")


def test_no_build_sticks_when_holders_die_by_kill_9_and_the_system_gives_their_pids_to_other_processes(
  tmp_path: Path,
) -> None:
  slots = tmp_path / "slots"
  others = [subprocess.Popen(["sleep", "300"]) for _ in range(4)]
  release = tmp_path / "release"
  wait = f"while [ ! -e {release} ]; do sleep 0.02; done"
  holders = [start(tmp_path, "sh", "-c", wait, FURB_BUILD_SLOTS="4", FURB_BUILD_WAIT="120") for _ in range(12)]
  try:
    while len(list(slots.glob("slot*/pid"))) < 4:
      time.sleep(0.02)
    held = {int((s / "pid").read_text(encoding="utf-8")): s for s in slots.glob("slot*") if (s / "pid").exists()}
    dead = [p for p in holders if p.pid in held]
    for p in dead:
      p.kill()
      p.wait()
    for p, other in zip(dead, others, strict=True):
      (held[p.pid] / "pid").write_text(f"{other.pid}\n", encoding="utf-8")
    release.touch()
    assert [p.wait() for p in holders if p not in dead] == [0] * 8
    left = {(s / "pid").read_text(encoding="utf-8").strip() for s in slots.iterdir()}
    assert left <= {str(other.pid) for other in others}
  finally:
    for p in [*holders, *others]:
      p.kill()
      p.communicate()
