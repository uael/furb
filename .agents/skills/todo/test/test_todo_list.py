"""todo_list, the list of tasks of the session."""

import json
from pathlib import Path

from conftest import plain, settle
from furb import engine
from furb_monty import _monty
from skills.conftest import tasks, todo, viewed

TODO = next(x for x in _monty.official() if x["name"] == "todo")


def gesture(node: str | None, value: str | None = None) -> str:
  """What a person did to a node of the view, as the answer of its prompt."""
  return json.dumps({"node": node, "value": value})


async def test_todo_list_is_the_list_of_tasks_of_the_session(tmp_path: Path) -> None:
  """The list of tasks of the session: an act on the root that draws the list as a view in the footer of each window, a prompt of str to the operator on a new chain of its own labelled todo, and that lives as long as the life."""
  _, root = todo(tmp_path)
  await settle(300)
  (made,) = [a for a in engine.transcript(root) if a[0] == "chain"]
  assert made[4:6] == ("todo", "") and made[2] == "todo_list1"
  (shown,) = viewed(root)
  assert (shown[3], shown[4], shown[6]) == (made[1], "str", "operator")
  (listed,) = [a for a in engine.transcript(root) if a[0] == "todo_list"]
  assert listed[3] == root and not engine.over(listed[1])
  assert tasks(root) == []


async def test_the_message_of_that_prompt_is_the_view(tmp_path: Path) -> None:
  """The message of that prompt is the view, as JSON: its version 1, the slot footer, and a box that holds the heading Tasks, a button for each task, which shows its text after a mark of whether it is checked, then an input whose id is add."""
  _, root = todo(tmp_path)
  await settle(300)
  await engine.rung("todo_add('buy milk')", on=root)
  await settle(300)
  view = json.loads(str(viewed(root)[-1][5]))
  assert (view["view"], view["slot"], view["tree"]["node"]) == (1, "footer", "box")
  head, task, given = view["tree"]["children"]
  assert (head["node"], head["runs"][0]["text"]) == ("text", "Tasks")
  assert task == {"id": "task0", "node": "button", "label": "○ buy milk"}
  assert (given["id"], given["node"], given["value"]) == ("add", "input", "")


async def test_a_gesture_changes_the_list_and_draws_it_again(tmp_path: Path) -> None:
  """When the prompt of the view is done with a gesture, as JSON with the id of a node and a value, it changes the list and draws it again: a button checks or unchecks its task, and the input adds a task of its value, less the spaces around it."""
  _, root = todo(tmp_path)
  await settle(300)
  engine.close(gesture("add", "  buy milk "), str(viewed(root)[-1][1]))
  await settle(300)
  assert tasks(root) == ["○ buy milk"]
  engine.close(gesture("task0"), str(viewed(root)[-1][1]))
  await settle(300)
  assert tasks(root) == ["✓ buy milk"]
  engine.close(gesture("task0"), str(viewed(root)[-1][1]))
  await settle(300)
  assert tasks(root) == ["○ buy milk"]
  assert len(viewed(root)) == 4


async def test_a_word_that_adds_or_checks_a_task_draws_the_list_again(tmp_path: Path) -> None:
  """When a word adds or checks a task, it closes the prompt of the view that stands with no gesture and draws the list again."""
  _, root = todo(tmp_path)
  await settle(300)
  first = str(viewed(root)[-1][1])
  await engine.rung("todo_add('buy milk')", on=root)
  await settle(300)
  assert engine.peek(first) == gesture(None)
  second = str(viewed(root)[-1][1])
  await engine.rung("todo_check('buy milk')", on=root)
  await settle(300)
  assert engine.peek(second) == gesture(None)
  assert tasks(root) == ["✓ buy milk"] and not engine.over(str(viewed(root)[-1][1]))


async def test_the_journal_keeps_each_view_and_its_answer(tmp_path: Path) -> None:
  """The journal keeps each prompt of the view and its answer, so a later life on the same record draws the same list and asks no one."""
  sand, root = todo(tmp_path)
  await settle(300)
  await engine.rung("todo_add('buy milk')", on=root)
  await settle(300)
  engine.close(gesture("task0"), str(viewed(root)[-1][1]))
  await settle(300)
  record = plain(sand.record)
  later, again = todo(tmp_path, record=record)
  await settle(300)
  assert tasks(again) == ["✓ buy milk"] and len(viewed(again)) == 3
  assert [a for a in later.calls if a[0] == "reply"] == []


async def test_the_life_word_is_todo_list_on_the_root_alone(tmp_path: Path) -> None:
  """The life word of the extension is todo_list(), and the extension plays on the root alone, so the life holds one list."""
  assert TODO["life"] == "todo_list()" and TODO["root"]
  _, root = todo(tmp_path)
  two = engine.chain("two")
  await settle(300)
  await engine.rung("todo_add('buy milk')", on=root)
  await settle(300)
  # The chain of the view and each other chain run the setup of the root with no life word of it.
  chains = [root, two, *(a[1] for a in engine.transcript(root) if a[0] == "chain")]
  assert [a[1] for one in chains for a in engine.transcript(str(one)) if a[0] == "todo_list"] == ["todo_list1"]
  assert [a for a in engine.transcript(two) if a[0] == "todo_add"] == []
  assert tasks(root) == ["○ buy milk"]
