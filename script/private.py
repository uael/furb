"""The words and the traces of a person stay private: no record of the repository holds a prompt of the owner, his
name or account, a session id or a path of his home, so each proof of a replay runs on scripted words.

`uv run python script/private.py` reads each record that git tracks, and names each line that holds a trace: a path of the
home of a person, a checkout under ~/src of an account, the handle of the owner (the public repositories uael/furb and
uael/monty stand), the id of a session or of its scratch folder, and a prompt of the operator that is not scripted. It
exits 1 when one does. test/outside/test_private_script.py runs it on each build, in CI.

`uv run python script/private.py --scrub <record>...` writes each record again with each trace put to its stand-in, the same
in each place that it stands, so that a replay of it reads the same life.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# The files that hold the record of a life, its log, or what its modules held.
KINDS = ("*.jsonl", "*.jsonl.zst", "*.modules", "*.log")
# Each trace of a person, with the stand-in that a scrub writes in its place, in the order of the scrub.
TRACES: list[tuple[str, re.Pattern[str], str]] = [
  (
    "a scratch folder of a home",
    re.compile(r"/Users/[^/\s\"'\\]+/Library/Caches/furb/scratch/[0-9a-f]{16}"),
    "/home/example/.cache/furb/scratch/" + "0" * 16,
  ),
  ("a path of a home", re.compile(r"/Users/[^/\s\"'\\]+/|/home/(?!example/)[^/\s\"'\\]+/"), "/home/example/"),
  (
    "a checkout of an account",
    re.compile(r"~/src/github\.com/(?!example/)[^/\s\"'\\]+/furb[\w-]*"),
    "~/src/github.com/example/furb",
  ),
  ("the handle of the owner", re.compile(r"\buael\b(?!/(?:furb|monty)(?![\w-]))"), "example"),
  (
    "the id of a session",
    re.compile(r"(?<![0-9a-f-])(?!0{8}-)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}(?![0-9a-f-])"),
    "00000000-0000-0000-0000-000000000000",
  ),
  ("the scratch folder of a session", re.compile(r"(?<=scratch/)(?!0{16}\b|1{16}\b)[0-9a-f]{16}\b"), "0" * 16),
]
# The words of a person that a record held, each with the scripted words that stand for it.
SPOKEN = {
  "Are all other furb lifes than you dead ?": "Are the other lives of the sample stopped?",
  "And now?": "Check them again.",
  "kill all others lifes": "Stop the other lives of the sample.",
  "Can you see images now ? can you take care if finishing https://github.com/uael/furb/pull/555 ?": (
    "Read the sample image, then finish the sample change."
  ),
  "Wait.": "Hold on.",
}
# The place of the message in a prompt: ("prompt", id, by, on, shape, message, to).
MESSAGE = 5
# The prompts of the operator that a script wrote: the stand-ins above, the messages of the record of the conductor that
# tui/script/scripted.ts writes, and those of the bench of script/deepswe.py, its task and its nudge.
HARNESS = re.compile(r"confidence level [0-9.]+ is too low, continue|- Update the `display` style property to accept")


def scripted_of_the_tui() -> set[str]:
  """The messages of the operator that tui/script/scripted.ts scripts, read from its list of string literals."""
  text = (ROOT / "tui" / "script" / "scripted.ts").read_text(encoding="utf-8")
  listed = text.split("export const SCRIPTED", 1)[1].split("];", 1)[0]
  return {json.loads(one) for one in re.findall(r'^\s*("(?:[^"\\]|\\.)*"),?\s*$', listed, re.M)}


def operator_prompts(line: str) -> list[str]:
  """The messages of the prompts of the operator in a line of a record, at any depth of it."""
  try:
    entry = json.loads(line)
  except ValueError:
    return []
  found: list[str] = []
  stack = [entry]
  while stack:
    one = stack.pop()
    if isinstance(one, list):
      if len(one) > MESSAGE and one[0] == "prompt" and one[2] == "operator" and isinstance(one[MESSAGE], str):
        found.append(one[MESSAGE])
      stack.extend(one)
    elif isinstance(one, dict):
      stack.extend(one.values())
  return found


def text_of(path: Path) -> str:
  if path.suffix == ".zst":
    from compression import zstd  # noqa: PLC0415  # python 3.14, which the venv of the repository holds

    return zstd.decompress(path.read_bytes()).decode("utf-8")
  return path.read_text(encoding="utf-8")


def records(root: Path = ROOT) -> list[Path]:
  command = ["git", "ls-files", "-z", *KINDS]
  listed = subprocess.run(command, cwd=root, capture_output=True, text=True, check=True)  # noqa: S603
  return [root / one for one in listed.stdout.split("\0") if one]


def traces(path: Path, scripted: set[str]) -> list[str]:
  """Each trace of a person in a record, as its line and what it is."""
  found = []
  for n, line in enumerate(text_of(path).splitlines(), 1):
    found += [f"{path}:{n}: {name}: {hit[0][:80]}" for name, pattern, _ in TRACES if (hit := pattern.search(line))]
    found += [
      f"{path}:{n}: a prompt of the operator that no script wrote: {said[:80]!r}"
      for said in operator_prompts(line)
      if said and said not in scripted and not HARNESS.match(said)
    ]
  return found


def scrubbed(text: str) -> str:
  for said, stand in SPOKEN.items():
    for form in {said, json.dumps(said)[1:-1]}:
      text = text.replace(form, stand)
  for _, pattern, stand in TRACES:
    text = pattern.sub(stand, text)
  return text


def main(argv: list[str]) -> int:
  if argv[:1] == ["--scrub"]:
    for one in map(Path, argv[1:]):
      made = scrubbed(text_of(one)).encode("utf-8")
      if one.suffix == ".zst":
        from compression import zstd  # noqa: PLC0415

        made = zstd.compress(made)
      one.write_bytes(made)
    return 0
  scripted = {*SPOKEN.values(), *scripted_of_the_tui()}
  found = [one for path in (map(Path, argv) if argv else records()) for one in traces(path, scripted)]
  said = f"private: {len(found)} traces of a person in the records" if found else "private: no record holds a trace"
  sys.stdout.write("".join(f"{one}\n" for one in [*found, said]))
  return 1 if found else 0


if __name__ == "__main__":
  sys.exit(main(sys.argv[1:]))
