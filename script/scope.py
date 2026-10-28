"""Say which jobs of the gates a change needs, from the paths that it changes.

The jobs on Linux always run: they hold the whole proof of the logic. A job on macOS or on Windows proves the system
alone, and a runner of macOS waits in a queue for minutes, so a change runs those jobs only when one of its paths can
behave in another way there. The paths come one on each line from the standard input, and the answer is the lines
`key=value` that a step of GitHub takes into its outputs. With `--all`, which each run but a pull request takes, every
job runs.
"""

import json
import re
import sys

LINUX = ["ubuntu-latest"]
EVERY = ["ubuntu-latest", "macos-latest", "windows-latest"]

# A path that no job needs on another system: prose, the tools of the repository, and the workflows that hold their
# own proof.
INERT = re.compile(
  r"(.*\.md|docs/(?!examples/).*|LICENSE|\.gitignore|\.furb/.*|\.agents/.*|\.claude/.*|script/.*|reserve/.*"
  r"|\.github/(ISSUE_TEMPLATE/.*|budgets\.json|workflows/(publish|release|titles)\.yml)"
  r"|release-please-config\.json|\.release-please-manifest\.json)"
)
# A path that only the suite of python reads.
PYTHON = re.compile(r"(test/.*|bind/python/.*|pyproject\.toml|uv\.lock|docs/examples/python/.*)")
# A path that only the bindings of TypeScript, the TUI and the app read.
TYPESCRIPT = re.compile(
  r"((tui|gui|bind/typescript)/.*|package\.json|bun\.lock|biome\.json|tsconfig.*\.json|docs/examples/typescript/.*)"
)
# A path that the crate builds, which every job reads.
RUST = re.compile(
  r"((src|cli|tests|extensions|\.cargo)/.*|Cargo\.(toml|lock)|build\.rs|rust-toolchain\.toml|docs/examples/rust/.*)"
)
APP = re.compile(r"gui/.*")


def scope(paths: list[str]) -> dict[str, object]:
  """The systems of the suite and of the bindings, and whether the crate on Windows and on macOS and the app run."""
  kinds = {"python": False, "typescript": False, "rust": False, "other": False, "app": False}
  for path in paths:
    if INERT.fullmatch(path):
      continue
    if PYTHON.fullmatch(path):
      kinds["python"] = True
    elif TYPESCRIPT.fullmatch(path):
      kinds["typescript"] = True
      kinds["app"] |= bool(APP.fullmatch(path))
    elif RUST.fullmatch(path):
      kinds["rust"] = True
    else:
      kinds["other"] = True
  every = kinds["rust"] or kinds["other"]
  return {
    "suite_os": EVERY if every or kinds["python"] else LINUX,
    "ts_os": EVERY if every or kinds["typescript"] else LINUX,
    "crate": every,
    "gui": every or kinds["app"],
  }


def lines(answer: dict[str, object]) -> str:
  """The answer as the lines of an output of a step."""
  return "".join(f"{key}={json.dumps(value, separators=(',', ':'))}\n" for key, value in answer.items())


def main() -> None:
  # A run that is no pull request has no change to read, and a path that no rule names runs every job.
  paths = ["every"] if "--all" in sys.argv[1:] else [line.strip() for line in sys.stdin if line.strip()]
  sys.stdout.write(lines(scope(paths)))


if __name__ == "__main__":
  main()
