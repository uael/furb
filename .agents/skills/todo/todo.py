import json

from furb.engine import OPERATOR, Act, act, chain, close, controlled, over, peek, prompt, scope

todo = """todo_list() keeps a list of tasks that the operator and each model share, in a view in the footer of each
window, on a chain of its own labelled todo. todo_add(text) adds a task to the list, and todo_check(text) checks the
task of that text, or unchecks it. A click on a task in the view checks it or unchecks it, and a task written in the
input of the view adds it. The skill plays on the root alone."""


def todo_drawn(tasks: list) -> str:
  head = {"id": "head", "node": "text", "runs": [{"text": "Tasks", "tone": "muted"}]}
  rows = [
    {"id": f"task{n}", "node": "button", "label": ("✓ " if done else "○ ") + text}
    for n, (text, done) in enumerate(tasks)
  ]
  given = {"id": "add", "node": "input", "value": "", "hint": "a task"}
  tree = {"id": "todo", "node": "box", "axis": "column", "gap": 0, "border": True, "children": [head, *rows, given]}
  return json.dumps({"view": 1, "slot": "footer", "tree": tree})


def todo_changed(tasks: list, node: object, value: object) -> list:
  if node == "add" and isinstance(value, str) and value.strip():
    return [*tasks, [value.strip(), False]]
  at = int(node[4:]) if isinstance(node, str) and node[:4] == "task" and node[4:].isdigit() else -1
  return [[t, d != (n == at)] for n, (t, d) in enumerate(tasks)]


def todo_checked(tasks: list, text: str) -> list:
  return [[t, d != (t == text)] for t, d in tasks]


def todo_gesture(value: object) -> dict:
  try:
    gesture = json.loads(value) if isinstance(value, str) else {}
  except ValueError:
    gesture = {}
  return gesture if isinstance(gesture, dict) else {}


def todo_list(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    side = chain("todo", on=scope(id))
    tasks = []
    while True:
      drawing = prompt(str, todo_drawn(tasks), to=OPERATOR, on=side)
      moved = False
      while not moved:
        said = yield {drawing, "todo_add", "todo_check"}
        moved = said[0] in ("todo_add", "todo_check") or over(drawing)
        if said[0] == "todo_add":
          tasks = [*tasks, [str(said[4]), False]]
        elif said[0] == "todo_check":
          tasks = todo_checked(tasks, str(said[4]))
        elif moved:
          gesture = todo_gesture(peek(drawing))
          tasks = todo_changed(tasks, gesture.get("node"), gesture.get("value"))
      if not over(drawing):
        close(json.dumps({"node": None, "value": None}), drawing)

  return act("todo_list", on, controlled(ear))


def todo_add(text: str, on: str = "") -> Act:
  def ear(id):
    yield "done", id, text

  return act("todo_add", on, ear, text)


def todo_check(text: str, on: str = "") -> Act:
  def ear(id):
    yield "done", id, text

  return act("todo_check", on, ear, text)
