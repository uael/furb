---
name: memory
description: Tells a chain the AGENTS.md or CLAUDE.md files of the project that apply to the files that it reads.
metadata:
  furb-word: memory.py
  furb-life: await recall()
  furb-interface: memory.pyi
  furb-default: "true"
---
memory reads the memory of the user, then the memory of each folder from the root down to the working directory,
and the memory of each folder on the way to a file that the chain reads. Its contract is `memory.pyi`.
