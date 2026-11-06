from typing import Final

from furb.engine import Act

naming: str
"""naming is the doctrine of the extension, a quote that says what it does."""
NAME_LONGEST: Final[int]
"""NAME_LONGEST is the number of characters past which a name of the session is cut."""
NAME_BRIEF: Final[str]
"""NAME_BRIEF is what the model that names the session reads before the first message of the operator."""

def name_of(text: str) -> str:
  """name_of(text) gives the name that a text holds: its first line that holds more than quotes, marks and periods, with none of them around it, cut to NAME_LONGEST characters, else an empty str."""

def name_session(on: str = "") -> Act:
  """The name of the session: an act on the root that waits for the first prompt that the operator makes in the life, and that is done with a name of the session that its message gives.
  A prompt that the operator made before the act started counts as the first prompt of the operator.
  The life word of the extension is name_session(), and the extension plays on the root alone, so a life names its session once.
  It asks the name of the actor that automatic() gives, with a prompt of str on a new chain of its own labelled naming, which reads NAME_BRIEF and then the message.
  With no such actor, or when that prompt gives no name, the name is name_of the message.
  The journal keeps the prompt and its answer, so a later life on the same record gives the same name and asks no model.
  """

def rename_session(name: str, on: str = "") -> Act:
  """The name that the operator gives the session: an act on the root that is done at once with the name, less the spaces around it.
  Its value is empty when the name holds only spaces, which gives the session its default name again.
  """
