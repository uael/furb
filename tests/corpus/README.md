# The corpus of the promise

Each folder here holds records that a build made, on a model that answers from a script: `next` holds the records of
the coming release, and each release has a folder of its own, named by its version, as `1.0.0`. The test
`make_the_corpus` in `tests/promise.rs` makes the records of `next`. Each record has a checkpoint and a tail:

- `work.jsonl` runs each default kit. Its root writes, reads, runs a command, waits, reads the clock and chance, asks
  a chain of its own, and defines a value and a function. A word of a model that waits an hour is open when the life
  keeps its checkpoint. A second life of the same engine goes on from the checkpoint, takes a prompt, defines on what
  the checkpoint holds, and leaves a prompt open.
- `crossed.jsonl` is the same life with no kit, whose tail names another engine.
- Beside each record, `<name>.modules` holds what the module of each chain held on the build that made the record:
  each value and what each definition gives, as python shows them. A name of a kit is not in it, since a minor may
  change a kit and its defaults.

The test `each_record_of_each_folder_of_the_corpus_opens_gives_back_its_modules_and_takes_one_more_turn` in
`tests/promise.rs` runs in `cargo test`, and in `.github/workflows/corpus.yml` on each pull request, on Linux. For each
record of each folder, it:

1. opens the record with no refusal and no drift,
2. gets back each value of each module and what each definition gives, as `<name>.modules` holds them,
3. reads the record with the reader of a record, with no life, and finds each chain in it, and
4. takes one more turn on the model of the script.

A life that goes on from a checkpoint that holds a reply of its model pauses its root, so the test wakes it, as an
operator does.

## The rule

- The corpus only grows. When `.github/workflows/release.yml` opens the pull request of a release, it copies `next`
  into the folder of the version of that release. Once the tag of that version stands, no file of that folder
  changes again. `script/hooks/corpus.py` refuses a change of each file of a folder that a tag holds, in the hooks
  and in `corpus.yml`.
- A change that makes a record of a folder of a release fail breaks the promise of its major.
- A record of a later build of a major opens on an earlier build of it, since each reader skips each field that it does
  not know: `a_record_whose_checkpoint_holds_a_field_that_no_reader_knows_still_opens` proves it on this build.
- Before 1.0.0, no promise covers a release. A change whose title has `!` may make `next` again and delete the folders
  of 0.x. Make `next` in an empty home and an empty temporary folder, so that no file of your machine goes into a
  record:

  ```sh
  CARGO_HOME=${CARGO_HOME:-$HOME/.cargo} RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup} HOME=$(mktemp -d) \
    TMPDIR=$(mktemp -d) cargo test -p furb --test promise make_the_corpus -- --ignored
  ```

`tests/replay` holds the records that only a build of the same `VERSION` replays word by word. They are not in the
promise.
