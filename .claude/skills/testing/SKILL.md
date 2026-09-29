---
name: testing
description: Choose the proportional checks for a change of furb, read a failure, and send a proof that needs a real model or another system to the place that owns it.
license: Adapted from the skill openclaw-testing of openclaw at 9a75b9b, under the terms in LICENSE.txt
---

# Testing furb

Prove the changed contract with the smallest check that means something, run the gates the change needs, and stop.
Run more, or run again, only for a new change, a failure, or a risk that stays open. Do not add a test that only
mirrors a change of the implementation that can be undone at low cost; use the `test-audit` skill when you write or
review a test.

`CLAUDE.md` gives every command, and says what each one runs. Read it first. `.github/workflows/gates.yml` says what
CI runs, and on which systems.

## Select the proof

Each gate runs locally. A gate of the crate after a change of the crate needs the wheels built again: run
`uv sync --reinstall-package furb-monty --reinstall-package furb-cli` before the suite, or the suite runs the old
crate.

| Change | Start with |
| --- | --- |
| A defect of the runtime | Reproduce it narrowly; run that proof again after the repair, and the tests beside it |
| `src/furb/engine.py` | `uv run pytest -q`, which runs the suite on both engines and the hygiene laws, whose wall of tokens the engine must stay under |
| `src/furb/engine.pyi` | The test of each sentence you changed, then `uv run pytest -q test/test_hygiene.py` and `uv run ty check --error-on-warning` |
| An extension | `uv run pytest -q extensions/<name>`, and the hygiene laws |
| The crate, `src/*.rs` | `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` with no feature, with `--features python`, with `--features typescript` and with both, then the wheels and `uv run pytest -q` |
| A door | The gates of the crate, then `uv run pytest -q test/outside` for python, and `bun run build && bun run check && bun run test` for TypeScript |
| The command line, `cli/` | `cargo test -p furb-cli`, which runs `furb` as a process on a claude command line that answers from a script |
| The TUI, `tui/` | `bun run check`, `bun run lint`, `bun run test`; after a change that a screen shows, `bun run screenshots` and read each capture |
| The contract of a value that crosses a door | The gates of both doors, since each door gives the same host API |
| The workflow | `git diff --check`, then read the run of the workflow on the pull request |
| The documents alone | Their links and their format, and `git diff --check`; no test of the runtime |

## Boundaries of source and state

Never open a life on a real record, and never run the engine against the record, the config or the files of the
owner. A replay of a record is a live run: its ears write files and run commands. Give a test a directory of its
own, as `Sand` and `tmp_path` do.

`uv run python script/smoke.py`, `script/play.py` and `script/deepswe.py` ask a real model. Each spends prompts, so
run one only when the owner asks, or when the change touches the provider and no scripted model can prove it.
`script/CLAUDE.md` says how to run the rig of DeepSWE.

The suite runs on Linux and on macOS, and the gates of TypeScript run on Windows too. A proof on one system does not
prove the others. When CI fails on a system you do not have, read its log before you change anything.

## CI failures

```bash
gh run list --branch <branch> --limit 10 --json databaseId,headSha,status,conclusion,url
gh run view <run-id> --json status,conclusion,headSha,url,jobs
gh run view <run-id> --job <failed-job-id> --log
```

Bind the diagnosis to the exact sha and job. A run that was cancelled may be one that a newer run of the same branch
replaced. Fetch the failed log once and use it again. Tell a failure of the product from a failure of the harness,
of the runner, or of a cache before you run a job again. A branch that is behind `main` fails for reasons that come
before its own change: bring it up to `main` before you verify it.

Fix the failures that belong to the change, and run the proof they touch again. Report a failure that does not
belong to the change with its evidence, rather than make the task wider.
