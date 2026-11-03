---
name: catchup
description: Tells a chain what changed in the files that it read or wrote, at each stand.
metadata:
  furb-word: catchup.py
  furb-life: catchup()
  furb-interface: catchup.pyi
  furb-default: "true"
---
At each stand, as when a later life opens on a record, catchup reads again each file that the chain read or wrote,
and tells the chain in one paragraph the lines that changed. Its contract is `catchup.pyi`.
