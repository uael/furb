# ruff: noqa: S603
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "script" / "private.py"
SPEC = importlib.util.spec_from_file_location("private", SCRIPT)
assert SPEC and SPEC.loader
private = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(private)
# A record that holds each trace of a person that the script names.
TRACED = [
  ["stand", "stand1", "provider", [[["operator", [], 200000]], "/Users/someone/src/furb", "m/low"]],
  ["read", "read1", "rung1", "chain1", "/home/someone/notes.md"],
  ["read", "read3", "rung1", "chain1", "/Users/first.last/src/furb"],
  ["rung", "rung3", "operator/someone", "chain1", "a word that a member typed", "", ""],
  ["read", "read2", "rung1", "chain1", "~/src/github.com/someone/furb-w6/DOCTRINE.md"],
  ["bash", "bash1", "rung1", "chain1", "ls /Users/x/Library/Caches/furb/scratch/3bcaab2537057c07"],
  ["tell", "rung1", "chain1", ["session 0c16d8e3-6726-4650-aac1-a3161707af0e of uael"]],
  ["tell", "rung2", "chain1", ["git log: Someone <someone@mail.org>, by operator/someone"]],
  ["prompt", "prompt1", "operator", "chain1", "None", "Are the other lives of the sample stopped?", ""],
  ["prompt", "prompt2", "operator/tui", "chain1", "None", "a word that no script wrote", ""],
]


# The rows that a scrub makes private: each but the words that no script wrote, which no stand-in replaces.
KEPT = [one for one in TRACED if "a word that" not in json.dumps(one)]


def run(*args: str) -> subprocess.CompletedProcess[str]:
  return subprocess.run([sys.executable, str(SCRIPT), *args], capture_output=True, text=True, timeout=120, check=False)


def test_no_record_of_the_repository_holds_a_trace_of_a_person() -> None:
  got = run()
  assert got.returncode == 0, got.stdout[-3000:]
  assert got.stdout == "private: no record holds a trace\n"


def test_each_trace_of_a_person_in_a_record_is_named(tmp_path: Path) -> None:
  record = tmp_path / "record.jsonl"
  record.write_text("".join(json.dumps(one) + "\n" for one in TRACED), encoding="utf-8")
  got = run(str(record))
  assert got.returncode == 1
  named = {line.split(": ")[1] for line in got.stdout.splitlines() if line.startswith(str(record))}
  assert named == {
    "a path of a home",
    "a checkout of an account",
    "the scratch folder of a session",
    "the id of a session",
    "the handle of the owner",
    "a prompt of the operator that no script wrote",
    "an address of mail",
    "an operator by his login",
  }
  assert "a word that no script wrote" in got.stdout and "a word that a member typed" in got.stdout
  assert "a path of a home: /Users/first.last\n" in got.stdout


def test_a_scrub_puts_each_trace_to_its_stand_in_and_keeps_the_public_repositories(tmp_path: Path) -> None:
  record = tmp_path / "record.jsonl"
  public = ["tell", "rung2", "chain1", ["see https://github.com/uael/furb and uael/monty"]]
  record.write_text("".join(json.dumps(one) + "\n" for one in [*KEPT, public]), encoding="utf-8")
  assert run("--scrub", str(record)).returncode == 0
  text = record.read_text(encoding="utf-8")
  assert run(str(record)).stdout == "private: no record holds a trace\n"
  assert "github.com/uael/furb and uael/monty" in text
  assert [json.loads(line)[0] for line in text.splitlines()] == [one[0] for one in [*KEPT, public]]


def test_a_scrub_numbers_the_stand_ins_so_two_sessions_stay_two(tmp_path: Path) -> None:
  record = tmp_path / "record.jsonl"
  ids = ["0c16d8e3-6726-4650-aac1-a3161707af0e", "9f00aa11-2222-4333-8444-555566667777"]
  record.write_text(
    "".join(json.dumps(["tell", "rung1", "chain1", [one, ids[0]]]) + "\n" for one in ids), encoding="utf-8"
  )
  assert run("--scrub", str(record)).returncode == 0
  first, second = (json.loads(line)[3] for line in record.read_text(encoding="utf-8").splitlines())
  assert first == ["00000000-0000-0000-0000-000000000001"] * 2
  assert second == ["00000000-0000-0000-0000-000000000002", "00000000-0000-0000-0000-000000000001"]
