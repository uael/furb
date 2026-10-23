# The corpus of the versions

Each record here is a life that a build of the current version made, `VERSION` in `src/world/store.rs`. The test
`each_record_of_the_corpus_names_this_version_and_replays_on_this_build_with_no_drift` in `src/life.test.rs` replays
each record in a life that only inspects it, on each build, in `cargo test`. So it is the gate that keeps the versions
few.

- A change that replays each record to the same acts keeps the version. A life that a build of the version made goes
  on with the new build, with no switch.
- A change that makes a record drift adds one to `VERSION`, and makes the corpus again in the same change:

  ```
  cargo test -p furb --lib life::test::make_the_corpus -- --ignored
  ```

  A life on a record of an older version is refused before any replay, with both versions. The command line runs the
  build of that version in its place when the store of builds of the machine holds it, and
  `script/builds.sh <version>` puts it there. Once that build writes a checkpoint at the tip of the record, as the
  command line has it do for an older version, the life goes on from the checkpoint on the new build.

`work.jsonl` is the life that `make_the_corpus` runs, on a model that answers from a script. Its root writes, reads,
runs a command, waits, reads the clock and chance, and asks a chain of its own. A second life on the record takes
one prompt more, so the record holds two openings. Each official extension runs on it.
