# Audit a record

Someone gives you a record of furb and says: this is what my agent did. The audit lets you check that claim on your
own machine, with no model, no network, and no part of this repository: the command `furb audit` does it.

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
3. Lets furb make the life again from the record, from its first entry. The engine runs each word of the record again,
   and it stops at the first act that does not agree with the record. At each checkpoint of the record, the life is
   born again, and the checkpoint has to hold what the life made before it.
4. Hashes each answer of a model and each word that the life ran after the last checkpoint, as the life made them
   again, and compares each hash with the hash of the same answer or word in the record. It also compares each act
   that you or the life made after the last checkpoint with the act that the record holds, where you is the operator
   of that life.

The audit cannot make an answer of a model again, or what a command printed, since no model and no command runs. The
sha256 of step 1 binds those answers. The replay of step 3 proves that the life made each act, and each word that it
ran, from those answers alone.

## Run it

The record names the build that made it in its fact `made`: the engine, as a digest of `src/furb/engine.py` and
`src/furb/kernel.py`, the schema of the record, the release of furb that made it, as `0.4.2`, and the texts of that
engine and of its Kernel. A build replays each segment of the record on the texts that its fact `made` holds. A build
refuses before any replay only a record of another major of the schema, and the audit says so. A record of a build
before the texts holds no texts. It replays on the engine of the build that audits it, and the replay stops at the
first act that does not agree with the record. So install the release that the fact `made` names, in a clean virtual
environment. A record from a build before the release was in the fact names no release: then install the latest
release first, and an older one when the replay stops:

```sh
python3.14 -m venv clean
clean/bin/pip install furb==<release>
```

A release before the command `furb audit` holds the audit as the script `script/bench/audit.py` at its tag, which
takes the same words: run it with `clean/bin/python -I audit.py` in place of `clean/bin/furb audit`.

Then run the audit, with the network off if you want to see that it needs none. On macOS, `sandbox-exec` turns the
network off for one command:

```sh
sandbox-exec -p '(version 1)(allow default)(deny network*)' \
  clean/bin/furb audit <record> <sha256> --report report.json
```

On Linux, `unshare -rn` does the same: `unshare -rn clean/bin/furb audit <record> <sha256>`.

The audit exits 0 when everything agrees, and 1 when something does not.

## The report

The audit writes its report as JSON on stdout, and in the file that `--report` names:

- `sha256`: the hash of the bytes of the record.
- `engine`: the engine that made the record, from its fact `made`.
- `entries` and `acts`: how many entries the record holds, and how many of them are acts.
- `checkpoints`: how many checkpoints the record holds, each proven as the life was born again at it.
- `answers`: for each reply of a model after the last checkpoint, the hash of its answer in the record and in the
  replay.
- `words`: for each word that the life ran after the last checkpoint, the hash of the word in the record, and the hash
  of the word that the replay ran.
- `wrong`: each thing that does not agree, and `agrees`: whether that list is empty.

## The hashes

A hash is the sha256 of a text, as hex. The text of an answer or of an act is its JSON as the line of the record
holds it: a tuple and a whole float keep their marks there, so an answer that the replay makes again with a list in
the place of a tuple does not agree. The text of a word is the word as the Kernel runs it: the Python that the model
wrote, with each quote made a string.
