---
name: lane
description: Runs each item of the work from A to Z on a chain of its own, with no model where code decides, and lands it.
metadata:
  furb-word: lane.py
  furb-interface: lane.pyi
  furb-plays: root
  furb-config: keys.json
---
`lane_ask` keeps each ask of the owner verbatim as a row of `LANE_ITEMS`, a file that git excludes, with its source and the command that says it is done. `lane_fill` starts `lane_item` on the next row, the hardest that waits for nothing, while fewer rows run than the key `cap` of the skill in the configs, and `lane_item` runs one row from A to Z. A lane takes one item: `lane_open` asks a worker for its reading and one shell command that fails now, `lane_work` holds the worker to that command, the checks of code and the probes of other models until no probe fails, and `lane_land` rebases and pushes with no model. `lane_run` runs each lane on a runner chain of its own, so lanes run at once, and `lane_job` and `lane_overdue` keep the watch of the root. `lane_steer` reads the measures against the rules of the owner. The kit learns by a change of a function and of the sentence of `lane.pyi` that holds it, with its test.

The tools with no model stand beside the word: `measures.sh <tree> <base>` with `kinds.py`, the one home of the kind of a file, gives the measures; `ci-green.sh <tree> <base> <check>...` waits for named checks of CI on the head; `ts-ci.sh <tree> <base>` and `ts-local.sh <tree>` run the tests of the TypeScript side on CI and on this machine, less those that `ts-held.txt` holds.
