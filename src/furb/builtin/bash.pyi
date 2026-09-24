from dataclasses import dataclass
from typing import Final, Literal

from furb.builtin.files import Text, span
from furb.engine import Act, Show

TIMEOUT: Final[float] = 600.0
"""TIMEOUT is the timeout, in seconds, of a command that does not say one."""
TAIL: Final[Show] = span(-250, -1)
"""TAIL is the span of the last 250 lines, which the stdout of a command without a show is told as."""

def bash(
  command: str,
  fed: bool = False,
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to.
  bash is given one show for each stream that bash tells.
  A feed whose text is None closes the stdin of the command.
  The commands of the World run at the same time.
  bash is given a command, a fed flag, a timeout, and a show for each stream, the plain words first, so the record replays it.
  bash gives the command, which is awaited for its exit code and its streams.
  The World runs the command in the working directory that cwd gives on its chain.
  The command runs until it ends, until its timeout, or until a cancel.
  The World ends the command at its timeout.
  The command completes with its exit code and its streams.
  The exit code of the command is None after a timeout.
  The stdin of the command is closed unless bash opened the command fed.
  Without a stderr show, the stderr of the command flows into its stdout.
  The stderr door of a merged command stays empty.
  A command without a timeout has the timeout TIMEOUT that the file names.
  The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it.
  The door bash1/stdin is writable while a fed command runs.
  A write of nothing to bash1/stdin closes the stdin.
  A read of bash1/stdout gives the lines of the command while the command runs.
  A wake on a chain with a source starts no inherited command again, since only the owner starts an act.
  The World starts it, and a later life starts it again only when the record shows it started and not ended, and then only at a wake.
  Its stdin is written while it runs and it is fed, which it says to the World as a feed and answers with the text that landed, and a write of nothing closes it; a write of it takes no word once the command ended, and none at all when the command was not opened fed.
  Without a show of its own, the stderr of it flows into its stdout, and the door of its stderr stays empty; with a hidden show it tells its header and its binding alone, and not its command and not its close.
  It answers a read of a stream and a peek while it runs, and once it has ended it lives on to answer a read of its streams and to refuse a write of its stdin, and nothing else reaches it, so a cancel does not end it, as it ends every other act, and a peek at it once it ended the life answers from its outcomes.
  It runs until it ends, until its timeout or until a cancel: the World is the one that ends it, at the timeout it reads off the act as at a cancel, since the World is the one running it, and the engine says nothing to make it.
  A pause stops no command: it runs on, and its close waits for the wake.
  Its close tells what its stdout shows, and what its stderr shows when that stream has a show of its own that is no hidden one, so a merged command tells no stderr.
  The paragraph of an exited command holds one showing for each text told.
  A show applies to a stream as it applies to a text.
  The stdout of a command without a show is told as TAIL, which is the span of its last 250 lines.
  A command refuses a write to bash1/stdin once it ended or was cancelled.
  The World asks a command whether its stderr flows into its stdout, as it asks cwd where its paths resolve, and the command answers from what its verb was given.
  The World hears the bash itself, with the command, the fed flag and the timeout, and no working directory and no show.
  """

@dataclass
class Exit:
  """What a command came to: its code, and each of its streams as a text, in the order the command keeps them.
  An Exit is the value that a command completes with.
  The streams of an Exit are its stdout and its stderr, each a Text.
  """

  code: int | None
  stdout: Text
  stderr: Text

type Bash = tuple[Literal["bash"], str, str, str, str, bool, float]
"""A bash carries the command, the fed flag and the timeout, and no show and no working directory."""
type Out = tuple[Literal["out"], str, str, str, str]
"""The streams of a command come as out facts while the command runs, which the record keeps.
A later life reads the parts that a command told before the death of the process.
"""
type Exited = tuple[Literal["exited"], str, str, int | None]
"""exited says the code, and the command is done with its Exit of that code and the streams it kept."""
type Feed = tuple[Literal["feed"], str, str, str | None]
"""A write to the stdin door of a fed command hands the World a feed with the text."""
type Merged = tuple[Literal["merged"], str, str, str, str]
"""A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks before it starts the command."""
