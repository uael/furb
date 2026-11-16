from furb.engine import Act

todo: str
"""todo is the doctrine of the extension, a quote that says what it does."""

def todo_list(on: str = "") -> Act:
  """The list of tasks of the session: an act on the root that draws the list as a view in the footer of each window, a prompt of str to the operator on a new chain of its own labelled todo, and that lives as long as the life.
  The message of that prompt is the view, as JSON: its version 1, the slot footer, and a box that holds the heading Tasks, a button for each task, which shows its text after a mark of whether it is checked, then an input whose id is add.
  When the prompt of the view is done with a gesture, as JSON with the id of a node and a value, it changes the list and draws it again: a button checks or unchecks its task, and the input adds a task of its value, less the spaces around it.
  When a word adds or checks a task, it closes the prompt of the view that stands with no gesture and draws the list again.
  The journal keeps each prompt of the view and its answer, so a later life on the same record draws the same list and asks no one.
  The life word of the extension is todo_list(), and the extension plays on the root alone, so the life holds one list.
  """

def todo_add(text: str, on: str = "") -> Act:
  """A task that a word adds to the list: an act that is done at once with its text, which the list hears."""

def todo_check(text: str, on: str = "") -> Act:
  """A task that a word checks, or unchecks when it is checked: an act that is done at once with its text, which the list hears, and which changes each task of that text."""
