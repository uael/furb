"""The audit of a record: a replay with no model and no network, which checks each word and each answer of a record.

An outsider runs it on an environment that holds `pip install furb` alone, at the version whose engine made the
record, as docs/audit.md says:

    python audit.py <record> <sha256> [--report <path>]

The audit checks first that the record hashes as the one who gives it says: the sha256 of its bytes. Then it opens a
life that only inspects a copy of the record. That life keeps nothing, enables no new extension and asks no model, and
the files, the commands, time and the web are silent, so it runs no command, touches no file and reaches no network.
The journal makes the record again from its words, and the life stops at the first act that does not agree with the
record. Last, the audit hashes each answer of a model and each word that the life gave the gate or ran, as the life
made them again, and checks each against the hash of the same answer and the same word in the record.

The audit writes its report as JSON, and exits 0 when everything agrees, and 1 when something does not, or when the
life refuses the record, as when the engine that made it is not the engine of this build.
"""

import argparse
import asyncio
import hashlib
import json
import re
import sys
import tempfile
from pathlib import Path

import furb_monty
from furb import engine, python, sheet
from furb.kernel import kernel
from furb.world import entries
from furb_monty import _monty

ACT = re.compile(r"\w+?\d+")
"""ACT matches the name of an act, which a maker of an act of the life is, or the operator."""


def digest(text: str) -> str:
  """The sha256 of a text, as hex."""
  return hashlib.sha256(text.encode()).hexdigest()


def plain(value: object) -> str:
  """The one form of a value that the audit hashes: its JSON with sorted keys, where a tuple is a list and a value
  that JSON does not hold, as a text, an exit or a refusal, is its repr."""
  return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), default=repr)


def said(record: list[tuple]) -> tuple[dict[str, str], dict[str, str]]:
  """What the record says: the hash of the answer of each reply that came to a turn, by its reply, and the hash of the
  word of that answer as the gate reads it, by the rung that asked for it."""
  makers = {str(fact[1]): str(fact[2]) for (fact,) in record if fact[0] == "reply"}
  answers, words = {}, {}
  for (fact,) in record:
    match fact:
      case ("done", str(reply), _, (_, str(word), *_) as turn) if engine.question(("reply", reply)):
        answers[reply] = digest(plain(turn))
        words[makers[reply]] = digest(engine.unquoted(word))
  return answers, words


def chains() -> list[str]:
  """Every chain of the life, from the root."""
  found, todo = [], [engine.ROOT]
  while todo:
    one = todo.pop(0)
    found.append(one)
    todo += [str(fact[1]) for fact in engine.transcript(one) if fact[0] == "chain" and fact[1] != one]
  return found


def made(record: list[tuple]) -> tuple[dict[str, str], dict[str, str], dict[str, str], list[str]]:
  """What the life made again: the hash of the outcome of each reply, by its reply; the hash of each word that the
  life ran, by its rung; the gate that read each word, by the hash of that word; and each act of the record that the
  life did not make again as the record holds it."""
  answers, words, gated = {}, {}, {}
  for one in chains():
    for fact in engine.transcript(one):
      match fact:
        case ("reply", str(reply), *_) if engine.question(("reply", reply)) and engine.over(reply):
          answers[reply] = digest(plain(engine.peek(reply)))
        case ("run", str(run), _, _, rung, str(word), _) if engine.question(("run", run)):
          words[str(rung)] = digest(word)
        case ("gate", str(gate), _, _, str(word)) if engine.question(("gate", gate)):
          gated.setdefault(digest(word), gate)
  # An act that an ear of the outside made, as the merged of a command, is of the World, which a life that inspects
  # holds silent, so the life makes again only the acts of the operator and of its own acts.
  missed = [
    str(fact[1])
    for (fact,) in record
    if engine.question(fact)
    and (fact[2] == engine.OPERATOR or ACT.fullmatch(str(fact[2])))
    and plain(engine.get(str(fact[1]))) != plain(fact)
  ]
  return answers, words, gated, missed


async def replayed(record: Path, directory: Path) -> tuple[dict[str, str], dict[str, str], dict[str, str], list[str]]:
  """The life that inspects the record, on the ears of the crate, the Kernel and the gate, and what it made again.
  The journal says the whole record again before boot returns."""
  stored, pairs = _monty.opened(
    directory=str(directory), record=str(record), inspecting=True, extensions=False, roster=[]
  )
  ears = dict(pairs)
  try:
    held = entries(stored)
    engine.boot(held, kernel=kernel(vars(python)), gate=sheet.gating(vars(python), furb_monty.gate), **ears)
    await asyncio.sleep(0)
    return made(held)
  finally:
    for one in ears.values():
      one.dispose()


def audited(path: Path, sha256: str) -> dict[str, object]:
  """The report of the audit of the record at the path, which the one who gives it says hashes as sha256."""
  held = path.read_bytes()
  report: dict[str, object] = {"record": str(path), "sha256": hashlib.sha256(held).hexdigest()}
  wrong: list[str] = []
  report["wrong"] = wrong
  if report["sha256"] != sha256.strip().lower():
    wrong.append(f"the record hashes as {report['sha256']}, not as {sha256}")
    return report
  with tempfile.TemporaryDirectory() as tmp:
    # The life reads a copy, so the audit holds no lease of the record and leaves nothing beside it.
    copy = Path(tmp) / "record.jsonl"
    copy.write_bytes(held)
    record = entries(_monty.kept(str(copy)))
    report["engine"] = next((str(fact[3]) for (fact,) in record if fact[0] == "made"), None)
    report["entries"] = len(record)
    report["acts"] = sum(1 for (fact,) in record if engine.question(fact))
    answers, words = said(record)
    try:
      again, ran, gated, missed = asyncio.run(replayed(copy, Path(tmp)))
    except Exception as no:
      # A record that this build refuses, or an act that does not agree with it, stops the life before it makes
      # what the audit checks.
      wrong.append(f"the replay stops: {type(no).__name__}: {no}")
      return report
  report["answers"] = [{"reply": k, "record": v, "replay": again.get(k)} for k, v in answers.items()]
  report["words"] = [{"rung": k, "record": v, "run": ran.get(k), "gate": gated.get(v)} for k, v in words.items()]
  wrong += [f"the life did not make {one} again as the record holds it" for one in missed]
  wrong += [f"the answer of {k} does not hash as the record says" for k, v in answers.items() if again.get(k) != v]
  # A word that the gate refused has a gate and no run, and a word that ran has both.
  wrong += [
    f"the word of {k} does not hash as the record says"
    for k, v in words.items()
    if ran.get(k) not in (None, v) or (k not in ran and v not in gated)
  ]
  return report


def main() -> int:
  """Audit the record, say the report, and keep it where --report names."""
  whole = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  whole.add_argument("record", type=Path, help="The record, a file of lines that the store of furb kept.")
  whole.add_argument("sha256", help="The sha256 of the record, as hex, as the one who gives the record says.")
  whole.add_argument("--report", type=Path, help="Where to write the report of the audit, as JSON.")
  args = whole.parse_args()
  report = audited(args.record, args.sha256)
  report["agrees"] = not report["wrong"]
  text = json.dumps(report, indent=2) + "\n"
  if args.report:
    args.report.write_text(text, encoding="utf-8")
  sys.stdout.write(text)
  return 0 if report["agrees"] else 1


if __name__ == "__main__":
  sys.exit(main())
