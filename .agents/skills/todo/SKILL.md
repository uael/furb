---
name: todo
description: Keeps a list of tasks that the operator and each model share, in a view in the footer of each window.
metadata:
  furb-word: todo.py
  furb-life: todo_list()
  furb-interface: todo.pyi
  furb-default: "false"
  furb-plays: root
---
`todo_list()` draws the list in the footer of each window, on a chain of its own, and keeps it there. A model adds a
task with `todo_add(text)` and checks one with `todo_check(text)`. The operator clicks a task to check it, and writes
a task in the input of the view to add it. The record keeps the list and each change, so a later life shows the same
list. Its contract is `todo.pyi`.
