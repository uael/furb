# The DeepSWE rig

`script/deepswe.py` puts one life of the engine against one task of the DeepSWE collection and grades what the life
left behind. The collection is at https://github.com/datacurve-ai/deep-swe, and the rig fetches it once.

The one law of the rig is that no rule of it reaches the model. The system prompt is the engine, minified, and
nothing else. The message is the instruction of the task as the task wrote it, and one line that asks the model to
close with a float from 0 to 1 for how sure it is that the work is done. Under 0.9, the operator says that the
level is too low and asks the model to go on, on the same chain. The model reads no other word.

## Before the first run

    uv run python script/deepswe.py tools

This installs the reporters that the verifiers of the tasks open by absolute path. Their images hold them, and
this host does not. It needs `npm`. It says if `go-ctrf-json-reporter` or `cargo-nextest` is missing: without the
first, a go task cannot be graded, and without the second, a rust task cannot be graded.

    uv run python script/deepswe.py validate <task>

This grades the reference solution of the task, which is the control. A rig that scores that solution under 1
measures itself and not the life, so a task that does not validate says nothing about the engine. Validate a task
once, before you spend money on it.

## The verbs

- `tools` installs the reporters, and needs no task.
- `seed <task>` stands the workroot up and stops before any model is asked, so a run that costs money starts warm.
- `validate <task>` grades the reference solution of the task.
- `run <task>` seeds, lets one life work the task, freezes what it left, grades it, and archives the whole of it.
- `freeze <task>` takes the submission from the checkout as it stands, as one diff against the base.
- `grade <task>` grades what the checkout holds now, or the submission a run already froze.
- `turns <task>` prints every turn of the root of a run, as the model read them.

`run` takes four options:

- `--to model/effort` is the actor the task is put to, and `opus/low` is the default.
- `--ceiling` is the dollars the life may spend, 8 by default, and 0 is no ceiling. It is a grant on the root chain,
  so the chain pauses when an answer carries the ledger to the ceiling.
- `--timeout` is the seconds the run may take, 1800 by default, and 0 is no cap.
- `--resume` goes on with the checkout and the record that stand, instead of seeding a fresh one.

`turns` takes `--to`, the actor that the folded life stands on, since `turns` opens a life on the record to fold it.

## Where everything stands

- The collection is cached at `$DEEPSWE_DIR`, or under the temporary directory of the host as `deep-swe`.
- Each task works under `$DEEPSWE_WORK/<task>`, or under the temporary directory as `furb-deepswe/<task>`. `base`
  is the pristine checkout with the dependencies of the task, `app` is the checkout the life works in, and `.run`
  holds the record, the numbers, the frozen submission and the reward.
- The interpreter of a python task is `.venv` in the checkout, at the version of python that the image of the task
  runs, which is the python of its verifier. The rig reads `PYTHON_VERSION` from the config of the image in its
  registry, and `uv` makes the interpreter. So code that runs under one version and fails under the other fails in
  the checkout too, before the grade.
- The rig also reads, from the layers of the image, every distribution that the python of the image holds. The base
  holds one commit and no tag, so a step builds the tree at a version that git makes up. After the steps, a seed
  installs again, from the tree, each distribution that the image built from it, at the version the image holds.
  What a layer holds is kept under `~/.cache/furb-deepswe/layers`, so each layer is read once.
- The pip of the interpreter is the pip of the image. Each `pip install` of a seed takes `-c constraints.txt`, a
  file in the workroot that pins every distribution that the image took from an index to the version that the
  image holds. So the checkout runs the pytest and the libraries that the verifier runs, and not the newest
  releases. The pins are on the command line and not in `PIP_CONSTRAINT`, since pip gives its environment to the
  builds that it isolates, and the builds of the image were free.
- `app`, and the tree that a grade outside docker makes, are copies of the base, and the venv of each copy is its
  own. The rig writes the path of the copy where the venv names the path of the base. So its scripts run the python
  of the copy, an editable package imports the code of the copy, and an install lands in the copy and not in the
  base.
- The reaper of the host, which cleans its temporary directory, takes the files of an old workroot and leaves its
  directories. So a seed writes `.seeded` last, with the commit of the base, and a later seed keeps the base only
  when `.seeded` names the commit and no file of the commit is gone. Otherwise it fetches and installs it again.
- The reporters stand under `~/.cache/furb-deepswe` when the path a task names is not this host's to write.
- Every graded run is archived in `traces/`, as the record it kept and a result beside it. The archive is ignored
  by git.
- `$DEEPSWE_GRADE` says how the verifier runs: `docker`, `unshare` or `local`. Unset, the rig takes a mount
  namespace if this host gives one, then docker if docker answers, and plain paths otherwise.
- `$DEEPSWE_NODE_HOME` puts a node of your choosing first on the PATH of a grade.
- `$FURB_CLAUDE_BIN` names the claude command line to ask, and the `claude` on PATH is taken when it is unset.

## What a run does to itself

A grant bounds the money and not the work, so two more bounds end a run that goes nowhere:

- The rig keeps a mark at the last thing that the life did, and ends a run that has bought 25 answers since that
  mark.
- Each step of the rig itself ends after half an hour, so an install that waits for input it never gets does not
  hold the run.

The commands of the model, the steps of a seed and a grade do not see the venv of furb that runs the rig. The rig
takes it out of the PATH and out of VIRTUAL_ENV before it does anything. In a python task, a command of the model
finds the venv of the checkout first on its PATH, and then the tools of the host.

The process of the life runs in the checkout. A word of a model runs in that process, so a word that joins a path
to the working directory of python writes inside the tree that is graded, and not outside it.

## Reading a run

The result beside the record in `traces/` holds the reward, how many of the hidden tests passed, what the run cost,
how many answers it bought and what the model closed with. `turns` prints what the model read, turn by turn, which
is where a friction of the engine shows.
