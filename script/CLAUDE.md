# The DeepSWE rig

`script/deepswe.py` puts one life of the engine against one task of the DeepSWE collection and grades what the life
left behind. The collection is at https://github.com/datacurve-ai/deep-swe, and the rig fetches it once.

The one law of the rig is that no rule of it reaches the model. The system prompt is the engine, minified, and
nothing else. The message is the instruction of the task as the task wrote it, and one line that asks the model to
close with a float from 0 to 1 for how sure it is that the work is done. Under 0.9 the operator says the level is
too low and asks it to go on, on the same chain. That is every word the model ever reads.

## Before the first run

    uv run python script/deepswe.py tools

This installs the reporters that the verifiers of the tasks open by absolute path, which their images bake in and
this host does not hold. It needs `npm`. It says which of `go-ctrf-json-reporter` and `cargo-nextest` are missing,
without which a go task and a rust task cannot be graded.

    uv run python script/deepswe.py validate <task>

This grades the task's own reference solution and is the control. A rig that scores that solution under 1 is
measuring itself and not the life, so a task that does not validate says nothing about the engine. Validate a task
once, before you spend anything on it.

## The verbs

- `tools` installs the reporters, and needs no task.
- `seed <task>` stands the workroot up and stops before any model is asked, so a run that costs money starts warm.
- `validate <task>` grades the reference solution of the task.
- `run <task>` seeds, lets one life work the task, freezes what it left, grades it, and archives the whole of it.
- `freeze <task>` takes the submission from the checkout as it stands, as one diff against the base.
- `grade <task>` grades what the checkout holds now, or the submission a run already froze.
- `turns <task>` prints every turn of the root of a run, as the model read them, which is how a friction is found.

`run` takes four more things:

- `--to model/effort` is the actor the task is put to, and `opus/low` is the default.
- `--ceiling` is the dollars the life may spend, 8 by default, and 0 is no ceiling. It is a grant on the root chain,
  so the chain pauses when an answer carries the ledger to it.
- `--timeout` is the seconds the run may take, 1800 by default, and 0 is no cap.
- `--resume` goes on with the checkout and the record that stand, instead of seeding a fresh one.

`turns` takes `--to`, the actor the folded life stands on, since it opens a life on the record to fold it.

## Where everything stands

- The collection is cached at `$DEEPSWE_DIR`, or under the temporary directory of the host as `deep-swe`.
- Each task works under `$DEEPSWE_WORK/<task>`, or under the temporary directory as `furb-deepswe/<task>`. `base`
  is the pristine checkout with the dependencies of the task, `app` is the checkout the life works in, and `.run`
  holds the record, the numbers, the frozen submission and the reward.
- The reporters stand under `~/.cache/furb-deepswe` when the path a task names is not this host's to write.
- Every graded run is archived in `traces/`, as the record it kept and a result beside it. The archive is ignored
  by git.
- `$DEEPSWE_GRADE` says how the verifier runs: `docker`, `unshare` or `local`. Unset, the rig takes a mount
  namespace if this host gives one, then docker if docker answers, and plain paths otherwise.
- `$DEEPSWE_NODE_HOME` puts a node of your choosing first on the PATH of a grade.
- `$FURB_CLAUDE_BIN` names the claude command line to ask, and the `claude` on PATH is taken when it is unset.

## What a run does to itself

Two bounds end a run that is going nowhere, since a grant bounds the money and not the work. The rig keeps a mark
at the last thing the life did, and ends a run that has bought 25 answers since that mark. Every step of the rig
itself ends at half an hour, so an install that waits on a prompt it will never be given holds nothing.

The life works with its process standing in the checkout, since a word of a model runs in that process, and one
that joins a path onto the working directory of python would otherwise write outside the tree that is graded.

## Reading a run

The result beside the record in `traces/` holds the reward, how many of the hidden tests passed, what the run cost,
how many answers it bought and what the model closed with. `turns` prints what the model actually read, turn by
turn, which is where a friction of the engine shows itself.
