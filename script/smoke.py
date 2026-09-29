"""The smoke of a real life: one model, one record, and the journal answering the same thread from it the second
time.

Run it from the root of the repository, as `uv run python script/smoke.py`. It runs `furb thread` twice on one record,
on the default actor of the crate. It spends the dollars of one turn of one model and no more: the first life asks
the model, and the second life is answered out of the record of the first, so no model is asked again.
"""

import subprocess
import sys
import tempfile
from pathlib import Path

from real import bought, ready, say, spent

MARKDOWN = "How many lines does the file a.txt hold?"
"""MARKDOWN is what the model is asked, of a file it must read to answer."""
HELD = "one\ntwo\nthree\n"
"""HELD is what the file holds."""
STALL = 300.0
"""STALL is the seconds the smoke waits for a command of furb, since a chain the World paused ends the command."""
CEILING = 1.0
"""CEILING is the dollars the first life may spend, ten answers or so, so that a model that loops is paused."""


def furb(yard: Path, *words: str) -> str:
  """What one command of the furb of this venv printed, on the record of the smoke, in its directory; and the end of
  the smoke when the command failed, with what furb said."""
  place = ["--record", str(yard / "record.jsonl"), "--cwd", str(yard)]
  command = [str(Path(sys.executable).with_name("furb")), *words, *place]
  got = subprocess.run(command, capture_output=True, text=True, timeout=STALL, check=False)  # noqa: S603
  if got.returncode:
    say(got.stderr.strip())
    raise SystemExit(1)
  return got.stdout.strip()


def main() -> int:
  """The two lives, in a directory of their own, and what the answer of the model cost."""
  ready()
  yard = Path(tempfile.mkdtemp(prefix="furb-smoke-"))
  record = yard / "record.jsonl"
  (yard / "a.txt").write_text(HELD, encoding="utf-8")
  say(f"the directory of the smoke is {yard}")
  furb(yard, "run", f"grant({CEILING})")
  for life in ("first", "second"):
    got = furb(yard, "thread", MARKDOWN, "--shape", "int")
    say(f"the {life} life gave {got}, and the record holds {len(bought(record))} answer(s) of a model")
    assert got == str(len(HELD.splitlines())), f"the {life} life answered {got}"
  _, word, usage, _ = bought(record)[0][3]
  say(f"the word of the model:\n{word}\nthe usage of the answer: {usage}")
  assert len(bought(record)) == 1, "the second life asked a model for what the record holds"
  say(f"the smoke spent {spent(record):.6f} dollars")
  say("SMOKE OK")
  return 0


if __name__ == "__main__":
  sys.exit(main())
