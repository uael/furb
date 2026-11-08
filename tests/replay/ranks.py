"""Write tests/replay/ranks.jsonl: a scripted life of the engine 5f66c73c, from no words of a person.

The engine 5f66c73c, of the commit 8f4d10c9b, says the first started of an act of the outside again as soon as a later
life makes the act, before each fact that the record holds before that started. The root runs a command. The operator
pauses an act that is over before the command starts. The word then reads the order of that pause and of that started
in its transcript, and runs a second command that names the order. With no adapter, the replay says the started first,
so the second command differs and the life drifts at bash2. tests/replay/README.md says the rule.

Usage: python3 tests/replay/ranks.py
"""

import hashlib
import json
import subprocess
from pathlib import Path

COMMIT = "8f4d10c9b"
DIGEST = "5f66c73c214963e47e4c1048ef2286d80e02537c56152a456191e81b6b8fc1c4"
WORD = (
  "b = await bash('echo a')\n"
  "kinds = [x[0] for x in transcript() if x[0] in ('pause', 'started') and x[1] in ('stand1', 'bash1')]\n"
  "await bash('echo ' + ' '.join(kinds))\n"
  "close(1)"
)


def text(path: str) -> str:
  return subprocess.run(["git", "show", f"{COMMIT}:{path}"], capture_output=True, text=True, check=True).stdout  # noqa: S603, S607


def exited(name: str, out: str) -> dict:
  return {
    "is": "Exit",
    "code": 0,
    "stdout": {"is": "Text", "path": f"{name}/stdout", "content": out},
    "stderr": {"is": "Text", "path": f"{name}/stderr", "content": ""},
  }


def bash(name: str, maker: str, command: str) -> list:
  return ["bash", name, maker, "chain1", command, "", {"is": "float", "args": ["120"]}, None]


def main() -> None:
  engine, kernel = text("src/furb/engine.py"), text("src/furb/kernel.py")
  assert hashlib.sha256(engine.encode() + b"\0" + kernel.encode() + b"\0").hexdigest() == DIGEST
  facts = [
    ["made", "chain1", "store", DIGEST, None, "1.0", "0.4.2", engine, kernel],
    ["chain", "chain1", "operator", "", "root", ""],
    ["stand", "stand1", "operator", "chain1"],
    ["done", "stand1", "provider", [[["operator", [], 200000], ["m", ["low"], 200000]], "/srv/furb-ranks", "m/low"]],
    ["prompt", "prompt1", "operator", "chain1", "int", "", ""],
    ["reply", "reply1", "rung1", "chain1", "m/low"],
    ["started", "reply1", "provider"],
    ["done", "reply1", "provider", {"is": "tuple", "args": [["assistant", WORD, None, []]]}],
    ["gate", "gate1", "chain1", "chain1", WORD],
    ["done", "gate1", "gate", []],
    bash("bash1", "rung1", "echo a"),
    ["pause", "stand1", "operator"],
    ["started", "bash1", "bash"],
    ["merged", "merged1", "bash", "chain1", "bash1"],
    ["out", "bash1", "bash", "a\n", "stdout"],
    ["done", "bash1", "bash", exited("bash1", "a\n")],
    bash("bash2", "rung1", "echo pause started"),
    ["started", "bash2", "bash"],
    ["merged", "merged2", "bash", "chain1", "bash2"],
    ["out", "bash2", "bash", "pause started\n", "stdout"],
    ["done", "bash2", "bash", exited("bash2", "pause started\n")],
  ]
  out = Path(__file__).with_suffix(".jsonl")
  out.write_text("".join(json.dumps([one], ensure_ascii=False) + "\n" for one in facts), encoding="utf-8")


if __name__ == "__main__":
  main()
