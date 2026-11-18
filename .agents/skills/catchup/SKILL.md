---
name: catchup
description: Tells a chain what changed in the files that it read or wrote, at each stand.
metadata:
  furb-word: catchup.py
  furb-life: catchup()
  furb-interface: catchup.pyi
  furb-default: "true"
  furb-plays: root
---
At each stand, as when a later life opens on a record, catchup reads again each file that each chain read or wrote,
once for a file whose last text is the same on several chains, and tells each chain in one paragraph the lines that
changed. It plays on the root alone. Its contract is `catchup.pyi`.
