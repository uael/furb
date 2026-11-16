# The corpus of the promise

Each folder here holds records. A build made them on a model that answers from a script.

- `next` holds the records of the coming release.
- Each release has its own folder, named by its version, as `1.0.0`.
- `record-<major>` holds the records of the point of the promise of that major of the schema of the record, as
  `next` held them there: `record-3` holds those of the stable cut (commit `eb49c0233`). From its point on, each record
  of the major opens on each later build with no drift, and no file of the folder changes. `record-2` holds those of
  the major before it, which no life opens: the life says why, and a window reads their facts.

The test `make_the_corpus` in `tests/suite/promise.rs` makes the records of `next`. A record keeps no checkpoint:

- `work.jsonl` runs each default kit. Its root writes, reads, runs a command, waits, reads the clock and chance, asks a
  chain of its own, and defines a value and a function. A word of a model waits on a prompt of a chain of its own. That
  word is open when the first life ends. A second life of the same engine opens on the record, takes a prompt, defines
  a name on what the first life held, and leaves a prompt open.
- `crossed.jsonl` is the same life with no kit. Its first life runs on another engine, whose texts its fact made holds:
  the program of the engine of the build that made it, with a line of comment more. The second life crosses to the
  build after it, so a later build replays the first segment there and carries its tip to the second.
- `language.jsonl` holds the definitions that a word uses, with no kit: `def` and `async def`, a class and a dataclass,
  `match`, comprehensions, f-strings and t-strings, a type alias, generics and decorators. A first life defines some of
  them, and a second life on its record defines the others.
- Beside each record, `<name>.modules` shows what the module of each chain held when the build made the record. It lists
  each value, and what each definition gives, as python shows them. It has no name of a kit. A minor release may change
  a kit and its defaults.

The test `each_record_of_each_folder_of_the_corpus_opens_gives_back_its_modules_and_takes_one_more_turn` in
`tests/suite/promise.rs` runs in `cargo test`. It also runs in `.github/workflows/corpus.yml`, on each pull request: on Linux, and on Linux, macOS and Windows when
the pull request changes a line of `Cargo.toml` that names monty, since a move of the pin of monty may change what a
stored definition gives.
For each record of each folder, it does these steps:

1. It opens the record in a life that only inspects it. It gets no refusal and no drift, and the life puts on to the
   World no act that the record answers.
2. It gets back each value of each module, and what each definition gives, as `<name>.modules` shows.
3. It reads the record with the reader of a record, with no life, and finds each chain.
4. It takes one more turn on the model of the script.

A life can open on a record that a stop cut while a reply of its model ran. That life pauses its root. The test wakes the root,
as an operator does.

## The rule

- The corpus only grows. `.github/workflows/release.yml` opens the pull request of a release. It copies `next` into the
  folder of the version of that release.
- After the tag of that version exists, no file of that folder changes. `script/hooks/corpus.py` refuses such a change,
  in the hooks and in `corpus.yml`, and each change of a file of a folder `record-<major>` once it stands.
- `docs/promise.md` says what the corpus promises and which test proves each part. The test
  `test_each_record_of_next_opens_on_the_last_release_of_its_major_once_that_release_holds_a_corpus` in
  `tests/outside/test_promise_rollback.py` installs the program furb of the last release of the major of the build that
  holds a folder, and opens each record of `next` there with no refusal and no drift. It skips a major with no such
  release: 0.4.2 refuses each record of `next` as made by a later version of the record.
- Before 1.0.0, a change with `!` in its title may make `next` again and delete the
  folders of 0.x. Make `next` with an empty home and an empty temporary folder. Then no file of your machine goes into
  a record:

  ```sh
  CARGO_HOME=${CARGO_HOME:-$HOME/.cargo} RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup} HOME=$(mktemp -d) \
    TMPDIR=$(mktemp -d) cargo test -p furb --test suite promise::make_the_corpus -- --ignored
  ```

Each record holds the texts of its engine in its fact made, so it replays on that engine, and no change of the engine
makes a record of `next` again. `tests/replay` holds other records, which each build replays word by word on their
own engines. They are not in the promise.
