from dataclasses import dataclass

from furb.builtin.files import HIDDEN, Text, span
from furb.engine import Act, Refused, Show, act, bound, covers, ended, pausing, started, told

TIMEOUT, TAIL = 600.0, span(-250, -1)


def bash(
  command: str,
  fed: bool = False,
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  def ear(id):
    streams, mute = {x: Text(x) for x in (f"{id}/stdout", f"{id}/stderr")}, "" if fed else "not fed"
    told(id, "" if show is HIDDEN else command, bound(id, "Exit"))
    while True:
      match (yield):
        case ("merged", qid, _, _, about) if about == id:
          yield "done", qid, show_err is None
        case ("read", qid, _, _, path) if path in streams:
          yield "done", qid, streams[path]
        case ("write", qid, _, _, path, text) if path == f"{id}/stdin":
          took = Refused(mute) if mute else Text(path, text)
          if not mute:
            yield "feed", id, text or None
            mute = "" if text else "closed"
          yield "done", qid, took
        case _ if mute == "ended":
          continue
        case ("peek", qid, _, _, at) if at == id:
          yield "done", qid, Exit(None, *streams.values())
        case ("out", about, _, text, stream) if about == id:
          into = f"{id}/{stream if show_err else 'stdout'}"
          streams[into] = streams[into].grow(text)
        case ("exited", about, _, code) if about == id:
          mute = "ended"
          if show is not HIDDEN:
            shows = zip(streams.values(), (show, show_err), strict=True)
            told(id, f"exited {code}", *[(t.path, t.content, s) for t, s in shows if s not in (None, HIDDEN)])
          yield "done", id, Exit(code, *streams.values())
        case ("cancel" | "close", *_) as a if covers(a, id):
          mute = "ended"
          yield "done", id, ended(a, id)

  return act("bash", on, started(pausing(ear)), command, fed, timeout)


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text
