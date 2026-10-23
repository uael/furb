# The audit of a record

A record of furb holds every act of a life, in the order that the life said it: each word that a model wrote, each
answer of a model, and each answer of the World, such as what a command printed. A later life makes the record again
from its words. `script/bench/audit.py` uses this to let a person who did not run the life check a record, with no
model, no network and no part of this repository but that one file.

## What the audit proves

Give the audit a record and its sha256, as the person who made the record says it. The audit:

1. Hashes the bytes of the record with sha256, and stops when the hash is not the one that you gave. So the record is
   the one that its maker gave, to the last byte.
2. Opens a life that only inspects a copy of the record. That life keeps nothing, enables no new extension and asks
   no model. The files, the commands, time and the web are silent in it, so it runs no command, touches no file and
   reaches no network.
3. Lets the journal make the life again from the record. The engine runs each word of the record again, and it stops
   at the first act that does not agree with the record, as a gate of a word that the record does not hold.
4. Hashes each answer of a model and each word that the life gave the gate or ran, as the life made them again, and
   compares each hash with the hash of the same answer and the same word in the record. It compares each act that the
   operator or the life made with the act that the record holds, too.

The audit cannot compute an answer of a model again, or what a command printed, since no model and no command runs.
The sha256 of step 1 binds these answers. The replay of step 3 proves that the life made each act, and each word that
it gave the gate and ran, from these answers alone.

## How to run it

The build that audits the record must be of the version of the build that made it. The record names that build in
its fact `made`: the engine, as a digest of `src/furb/engine.py` and `src/furb/kernel.py`, and the version of the
build. The audit says the engine in its report. A build of another version refuses the record before any replay, and
the audit says so.

Install furb from its wheels alone, in a clean virtual environment:

    python3.14 -m venv clean
    clean/bin/pip install --no-index --only-binary ":all:" --find-links <wheels> furb

`<wheels>` is a directory that holds the wheels of `furb`, `furb-monty` and `furb-cli` of the same build. With a
release on PyPI, `clean/bin/pip install furb==<version>` does the same. Then run the audit, with the network off if
you want to see that it needs none. On macOS, `sandbox-exec` turns the network off for one command:

    sandbox-exec -p '(version 1)(allow default)(deny network*)' \
      clean/bin/python -I audit.py <record> <sha256> --report report.json

The audit writes its report as JSON on stdout, and in the file that `--report` names:

- `sha256`: the hash of the bytes of the record.
- `engine`: the engine that made the record, from its fact `made`.
- `entries` and `acts`: how many entries the record holds, and how many of them are acts.
- `answers`: for each reply of a model that came to a turn, the hash of its answer in the record and in the replay.
- `words`: for each rung that a model answered, the hash of its word in the record, the hash of the word that the
  replay ran, and the gate that read the word in the replay. A word that the gate refused has a gate and no run.
- `wrong`: each thing that does not agree, and `agrees`: whether that list is empty.

The audit exits 0 when everything agrees, and 1 when something does not.

## The hashes

A hash is the sha256 of a text, as hex. The text of an answer is its JSON with sorted keys, where a tuple is a list
and a value that JSON does not hold, as a text, an exit or a refusal, is its repr. The text of a word is the word as
the gate reads it: the python that the model wrote, with each quote made a string.
