---
name: skills
description: Lists the skills of the project and of the user, and loads one.
metadata:
  furb-word: skills.py
  furb-life: await listed()
  furb-default: "true"
  furb-interface: skills.pyi
---
`skills()` lists each skill that the folders of skills hold, and `skill(name)` loads one. `await reload()` loads
again each skill that the chain runs, reads the servers of MCP again, and moves furb to its newest build, as the
key of a reload in a window does. Its contract is `skills.pyi`.
