# Audit a record

Someone gives you a record of furb and says: this is what my agent did. The audit lets you check that claim on your
own machine, with no model, no network, and no part of this repository but one file, `script/bench/audit.py`.

A record holds every act of a life, in order: each word that a model wrote, each answer of a model, and each answer
of the World, such as what a command printed. furb can make a life again from its record. The audit uses this to
prove that the record holds together.

## What the audit proves

Give the audit a record and its sha256, as the person who made the record gives it. The audit then:

1. Hashes the record, and stops when the hash is not the one that you gave. So you read the record that its maker
   gave, to the last byte.
2. Opens a life that only inspects a copy of the record. That life keeps nothing, enables no new extension, and asks
   no model. The files, the commands, time and the web are silent in it, so it runs no command, touches no file, and
   reaches no network.
3. Lets furb make the life again from the record. The engine runs each word of the record again, and it stops at the
   first act that does not agree with the record.
4. Hashes each answer of a model and each word that the life gave the gate or ran, as the life made them again, and
   compares each hash with the hash of the same answer or word in the record. It also compares each act that you or
   the life made with the act that the record holds.

The audit cannot make an answer of a model again, or what a command printed, since no model and no command runs. The
sha256 of step 1 binds those answers. The replay of step 3 proves that the life made each act, and each word that it
gave the gate and ran, from those answers alone.

## Run it

The engine that made the record must be the engine that audits it. The record names that engine in its fact `made`,
as a digest of `src/furb/engine.py` and `src/furb/kernel.py`. Another engine refuses the record before any replay,
and the audit says so. So install the release of furb that made the record, in a clean virtual environment:

```sh
python3.14 -m venv clean
clean/bin/pip install furb==<version>
curl -sLO https://raw.githubusercontent.com/uael/furb/v<version>/script/bench/audit.py
```

> [!NOTE]
> The first release that holds the audit, 0.2.0, waits for its release. Until then, build the wheels of `furb`,
> `furb-monty` and `furb-cli` from one commit, and install them with
> `clean/bin/pip install --no-index --only-binary ":all:" --find-links <wheels> furb`.

Then run the audit, with the network off if you want to see that it needs none. On macOS, `sandbox-exec` turns the
network off for one command:

```sh
sandbox-exec -p '(version 1)(allow default)(deny network*)' \
  clean/bin/python -I audit.py <record> <sha256> --report report.json
```

The audit exits 0 when everything agrees, and 1 when something does not.

## The report

The audit writes its report as JSON on stdout, and in the file that `--report` names:

- `sha256`: the hash of the bytes of the record.
- `engine`: the engine that made the record, from its fact `made`.
- `entries` and `acts`: how many entries the record holds, and how many of them are acts.
- `answers`: for each reply of a model that came to a turn, the hash of its answer in the record and in the replay.
- `words`: for each rung that a model answered, the hash of its word in the record, the hash of the word that the
  replay ran, and the gate that read the word in the replay. A word that the gate refused has a gate and no run.
- `wrong`: each thing that does not agree, and `agrees`: whether that list is empty.

## The hashes

A hash is the sha256 of a text, as hex. The text of an answer is its JSON with sorted keys, where a tuple is a list,
and a value that JSON does not hold, such as a text, an exit or a refusal, is its repr. The text of a word is the word
as the gate reads it: the Python that the model wrote, with each quote made a string.
