"""pr puts a flag of its labels after the dash of the body with a space, whether the str of flags starts with one or not."""

import asyncio
import re
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)


@dataclass
class Text:
  path: str
  content: str = ""


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Refused(Exception):
  pass


def command_of(labels: str | list[str]) -> str:
  commands: list[str] = []

  async def bash(command: str, **_: object) -> Exit:
    commands.append(command)
    return Exit(0, Text("out", "https://github.com/uael/furb/pull/1\n"), Text("err"))

  ns: dict = {
    "bash": bash,
    "wait": None,
    "clock": lambda: 0.0,
    "Exit": Exit,
    "Refused": Refused,
    "span": lambda lo, hi: (lo, hi),
    "HIDDEN": "hidden",
  }
  exec(compile(IMPORT.sub("", (HERE / "pr.py").read_text(encoding="utf-8")), "pr.py", "exec"), ns)  # noqa: S102
  asyncio.run(ns["pr"]("fix(pr): Take flags", "why", ["x"], ["checked"] * 6, labels=labels))
  return commands[0]


def test_a_str_of_flags_gets_a_space_with_or_without_one_of_its_own() -> None:
  for labels in ("--label w:x", " --label w:x", "  --label w:x "):
    command = command_of(labels)
    assert "--body-file - --label w:x; fi" in command
    assert "--body-file - --add-label w:x >/dev/null" in command
    assert "---" not in command


def test_a_list_of_labels_gives_one_flag_for_each_and_a_str_of_none_gives_none() -> None:
  assert "--body-file - --label 'w:x' --label 'it'\\''s'; fi" in command_of(["w:x", "it's"])
  assert "--body-file -; fi" in command_of("")
