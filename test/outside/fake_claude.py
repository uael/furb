"""A claude that answers from a script: enough of the stream-json contract of the CLI to test the provider against.

Every shape it emits was copied from a real `claude -p --output-format stream-json` run, so a provider that reads this
one reads the real one. `FURB_FAKE_MODE` picks how the turn goes wrong; the default is a turn that goes right.
"""

import json
import os
import sys
import time
from pathlib import Path

HOME = "FURB_FAKE_HOME"
MODE = "FURB_FAKE_MODE"


def out(said: object) -> None:
  """One line of the stream, flushed, because a provider that waits on a buffer waits forever."""
  sys.stdout.write((said if isinstance(said, str) else json.dumps(said)) + "\n")
  sys.stdout.flush()


def sid_of(args: list[str]) -> str:
  """The conversation this process is running: the one it was given, the one it resumes, or the fork's brand new id."""
  if "--session-id" in args:
    return args[args.index("--session-id") + 1]
  held = args[args.index("--resume") + 1]
  return f"{held}-forked" if "--fork-session" in args else held


def main() -> int:
  args = sys.argv[1:]
  home = Path(os.environ[HOME])
  mode = os.environ.get(MODE, "")
  sid = sid_of(args)
  (home / f"argv-{len(list(home.glob('argv-*.json')))}.json").write_text(json.dumps(args))
  if mode == "boom":
    sys.stderr.write("the fake refused to run\n")
    return 3
  turn = 0
  while raw := sys.stdin.readline():
    turn += 1
    got = json.loads(raw)
    said = " ".join(b.get("text", "") for b in got["message"]["content"] if b.get("type") == "text")
    with (home / "heard.jsonl").open("a", encoding="utf-8") as fh:
      fh.write(json.dumps({"text": said}) + "\n")
    if mode == "hush":
      time.sleep(5)
      continue
    if mode == "flood":
      out("x" * 8192)
      continue
    if mode == "deaf":
      out("this is not a line the stream ever wrote")
      time.sleep(5)
      continue
    if mode == "slow":
      time.sleep(0.3)  # long enough that a second turn arrives while this one is still in flight
    if mode == "junk":
      out("this is not a line the stream ever wrote")
      out({"type": "assistant", "session_id": sid})
      out(
        {"type": "assistant", "session_id": sid, "message": {"content": [{"type": "tool_use", "id": "x", "name": "y"}]}}
      )
      out({"type": "system", "subtype": "init", "session_id": sid})
    if mode == "bare":
      out(
        {
          "type": "result",
          "session_id": sid,
          "subtype": "success",
          "is_error": False,
          "result": "said nothing, ended anyway",
          "stop_reason": "max_tokens",
        }
      )
      continue
    out(
      {
        "type": "assistant",
        "session_id": sid,
        "message": {
          "content": [
            {"type": "thinking", "thinking": f"about {said[:20]}", "signature": "sig"},
            {"type": "text", "text": f"heard: {said}"},
          ],
          "usage": {
            "input_tokens": 7,
            "output_tokens": 1,
            "cache_read_input_tokens": 0,
            "cache_creation_input_tokens": 0,
          },
        },
      }
    )
    if mode == "gone":
      return 0
    out(
      {
        "type": "result",
        "session_id": sid,
        "subtype": "success",
        "is_error": mode == "sour",
        "result": "the fake reports an error" if mode == "sour" else f"heard: {said}",
        "stop_reason": "end_turn",
        "total_cost_usd": round(0.5 * turn, 4),
        "usage": {
          "input_tokens": 10 * turn,
          "output_tokens": 5,
          "cache_read_input_tokens": 2,
          "cache_creation_input_tokens": 1,
        },
      }
    )
  return 0


if __name__ == "__main__":
  sys.exit(main())
