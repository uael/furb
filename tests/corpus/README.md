# The corpus of the promise

Each folder here holds records. A build made them on a model that answers from a script.

- `next` holds the records of the coming release.
- Each release has its own folder, named by its version, as `1.0.0`.

The test `make_the_corpus` in `tests/promise.rs` makes the records of `next`. Each record has a checkpoint and a tail:

- `work.jsonl` runs each default kit. Its root writes, reads, runs a command, waits, reads the clock and chance, asks a
  chain of its own, and defines a value and a function. A word of a model waits for an hour. That word is open when the
  life keeps its checkpoint. A second life of the same engine goes on from the checkpoint. It takes a prompt, defines a
  name on what the checkpoint holds, and leaves a prompt open.
- `crossed.jsonl` is the same life with no kit. Its tail names another engine.
- `language.jsonl` holds the definitions that a word uses, with no kit: `def` and `async def`, a class and a dataclass,
  `match`, comprehensions, f-strings and t-strings, a type alias, generics and decorators. A checkpoint holds some of
  them, and the tail after it holds the others.
- Beside each record, `<name>.modules` shows what the module of each chain held when the build made the record. It lists
  each value, and what each definition gives, as python shows them. It has no name of a kit. A minor release may change
  a kit and its defaults.

The test `each_record_of_each_folder_of_the_corpus_opens_gives_back_its_modules_and_takes_one_more_turn` in
`tests/promise.rs` runs in `cargo test`. It also runs in `.github/workflows/corpus.yml`, on each pull request: on Linux, and on Linux, macOS and Windows when
the pull request changes a line of `Cargo.toml` that names monty, since a move of the pin of monty may change what a
stored definition gives.
For each record of each folder, it does these steps:

1. It opens the record. It gets no refusal and no drift.
2. It gets back each value of each module, and what each definition gives, as `<name>.modules` shows.
3. It reads the record with the reader of a record, with no life, and finds each chain.
4. It takes one more turn on the model of the script.

A life can go on from a checkpoint that holds a reply of its model. That life pauses its root. The test wakes the root,
as an operator does.

## The rule

- The corpus only grows. `.github/workflows/release.yml` opens the pull request of a release. It copies `next` into the
  folder of the version of that release.
- After the tag of that version exists, no file of that folder changes. `script/hooks/corpus.py` refuses such a change,
  in the hooks and in `corpus.yml`.
- A change that makes a record of a release folder fail breaks the promise of its major version.
- A later build of a major version can write a record that an earlier build of that major opens. Each reader skips each
  field that it does not know. The test `a_record_whose_checkpoint_holds_a_field_that_no_reader_knows_still_opens`
  proves it on this build.
- Before 1.0.0, no promise covers a release. A change with `!` in its title may make `next` again and delete the
  folders of 0.x. Make `next` with an empty home and an empty temporary folder. Then no file of your machine goes into
  a record:

  ```sh
  CARGO_HOME=${CARGO_HOME:-$HOME/.cargo} RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup} HOME=$(mktemp -d) \
    TMPDIR=$(mktemp -d) cargo test -p furb --test promise make_the_corpus -- --ignored
  ```

Each record holds the texts of its engine in its fact made, so it replays on that engine, and no change of the engine
makes a record of `next` again. `tests/replay` holds other records, which each build replays word by word on their
own engines. They are not in the promise.
