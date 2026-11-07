# ruff: noqa: S603, S607
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import IO

import pytest

SCRIPT = Path(__file__).resolve().parents[2] / "script" / "heavy.sh"
pytestmark = pytest.mark.skipif(sys.platform == "win32", reason="the script runs in sh")


def machine(tmp: Path, free: int, *, swap: int = 6000, grows: bool = False) -> None:
  """A fake reading of memory_pressure and of sysctl, in files that the test changes while the script runs: grows makes
  the swap grow by 300 MB at each read."""
  shim = tmp / "shim"
  shim.mkdir(exist_ok=True)
  # The swap grows before the free memory changes, and a read of the swap that saw it grow ends before the test writes
  # the swap again and drops the looks that saw it grow.
  stops = not grows and (tmp / "grows").exists()
  if grows:
    (tmp / "grows").touch()
  elif stops:
    (tmp / "grows").unlink()
    time.sleep(0.3)
  (tmp / "swap").write_text(f"{swap}\n")
  if stops:
    (tmp / "slots" / "swap").unlink(missing_ok=True)
  (tmp / "free").write_text(f"{free}\n")
  pressure, sysctl = shim / "memory_pressure", shim / "sysctl"
  pressure.write_text(f'#!/bin/sh\necho "System-wide memory free percentage: $(cat {tmp}/free)%"\n')
  sysctl.write_text(
    "#!/bin/sh\n"
    'case "$2" in hw.memsize) echo 38654705664 ;;\n'
    f"*) s=$(cat {tmp}/swap); [ ! -e {tmp}/grows ] || {{ s=$((s + 300)); echo $s > {tmp}/swap; }}\n"
    '   echo "total = 7168.00M  used = $s.00M  free = 1.00M  (encrypted)" ;;\nesac\n'
  )
  pressure.chmod(0o755)
  sysctl.chmod(0o755)


def env(tmp: Path, **more: str) -> dict[str, str]:
  kept = {n: v for n, v in os.environ.items() if not n.startswith("FURB_HEAVY_") and n != "CI"}
  path = f"{tmp / 'shim'}{os.pathsep}{os.environ['PATH']}"
  return {**kept, "PATH": path, "FURB_HEAVY_DIR": str(tmp / "slots"), "FURB_HEAVY_POLL": "0.05", **more}


def start(tmp: Path, *command: str, err: IO[str] | int = subprocess.PIPE, **more: str) -> subprocess.Popen[str]:
  pipe = subprocess.PIPE
  return subprocess.Popen(["sh", str(SCRIPT), *command], env=env(tmp, **more), text=True, stdout=pipe, stderr=err)


def run(tmp: Path, *command: str, **more: str) -> subprocess.CompletedProcess[str]:
  args = ["sh", str(SCRIPT), *command]
  return subprocess.run(args, env=env(tmp, **more), text=True, capture_output=True, timeout=60, check=False)


@pytest.mark.parametrize(("given", "code"), [({"free": 50}, 0), ({"free": 19}, 75), ({"free": 50, "grows": True}, 75)])
def test_with_no_command_it_says_the_memory_and_whether_a_step_can_start(
  tmp_path: Path, given: dict, code: int
) -> None:
  machine(tmp_path, **given)
  got = run(tmp_path)
  assert got.returncode == code and got.stdout.startswith(f"memory {given['free']}% free, swap ")


def entered(log: Path) -> int:
  return log.read_text(encoding="utf-8").split().count("in") if log.exists() else 0


def waiting(errs: list[Path]) -> int:
  return sum("heavy: waiting" in e.read_text(encoding="utf-8") for e in errs if e.exists())


def test_more_steps_than_the_memory_holds_wait_and_start_only_as_it_frees(tmp_path: Path) -> None:
  # Six steps on four slots, while less than a fifth of the memory is free and then while the swap grows: none starts.
  # Once the memory frees, four start and two wait, and the peak of steps at once never passes four.
  machine(tmp_path, 10)
  log, release = tmp_path / "log", tmp_path / "release"
  body = f"echo in >> {log}; while [ ! -e {release} ]; do sleep 0.05; done; echo out >> {log}"
  errs = [tmp_path / f"err{n}" for n in range(6)]
  files = [e.open("w", encoding="utf-8") for e in errs]
  runs = [start(tmp_path, "sh", "-c", body, err=f) for f in files]
  try:
    deadline = time.monotonic() + 30
    while waiting(errs) < 6 and time.monotonic() < deadline:
      time.sleep(0.05)
    assert (entered(log), waiting(errs)) == (0, 6)
    machine(tmp_path, 50, grows=True)
    time.sleep(1)
    assert entered(log) == 0
    machine(tmp_path, 50)
    while entered(log) < 4 and time.monotonic() < deadline:
      time.sleep(0.05)
    time.sleep(0.5)
    assert entered(log) == 4
  finally:
    release.touch()
    for f in files:
      f.close()
    for one in runs:
      one.communicate(timeout=60)
  assert [one.returncode for one in runs] == [0] * 6
  held = peak = 0
  for line in log.read_text(encoding="utf-8").split():
    held += 1 if line == "in" else -1
    peak = max(peak, held)
  assert peak == 4
  assert list((tmp_path / "slots").glob("slot*")) == []


def test_a_step_inside_a_step_takes_no_second_slot(tmp_path: Path) -> None:
  machine(tmp_path, 50)
  got = run(tmp_path, "sh", str(SCRIPT), "echo", "inner", FURB_HEAVY_SLOTS="1", FURB_HEAVY_WAIT="3")
  assert (got.returncode, got.stdout) == (0, "inner\n")


def test_the_slot_of_a_holder_that_ended_goes_to_the_next_step(tmp_path: Path) -> None:
  machine(tmp_path, 50)
  gone = subprocess.Popen(["true"])
  gone.wait()
  folder = tmp_path / "slots" / "slot1"
  folder.mkdir(parents=True)
  (folder / "pid").write_text(f"{gone.pid}\n", encoding="utf-8")
  assert run(tmp_path, "true", FURB_HEAVY_SLOTS="1", FURB_HEAVY_WAIT="5").returncode == 0


def test_a_step_that_waits_past_its_limit_runs_nothing_and_exits_75(tmp_path: Path) -> None:
  machine(tmp_path, 10)
  mark = tmp_path / "ran"
  got = run(tmp_path, "touch", str(mark), FURB_HEAVY_WAIT="1")
  assert got.returncode == 75 and "no step started in 1s, with memory 10% free" in got.stderr
  assert not mark.exists()


def test_a_step_on_a_runner_of_ci_takes_no_slot_and_waits_for_no_memory(tmp_path: Path) -> None:
  machine(tmp_path, 10)
  got = run(tmp_path, "echo", "ran", CI="true", FURB_HEAVY_WAIT="1")
  assert (got.returncode, got.stdout) == (0, "ran\n")
  assert not (tmp_path / "slots").exists()
