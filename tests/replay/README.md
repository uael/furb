# The records from before the stable cut

Each record here is a life of a build before the stable cut, the major 2 of the schema of the record, which
`docs/promise.md` says. The promise does not hold them: no life opens one, and a reader of a record reads its facts.

- `speaker.jsonl` is a scripted life of the engine `c63db8af`, with two chains. The test
  `a_record_from_before_the_stable_cut_opens_in_no_life` in `src/life.test.rs` proves that no life opens it and that
  the refusal says why. The test `a_record_from_before_the_stable_cut_shows_its_threads_to_a_window_and_says_why` in
  `host/tests/suite/life.rs` proves that a window reads its threads, read only, from the fold of its facts.
- `refused.jsonl` holds two replies, the first of which nothing took. `bind/typescript/test/meaning.test.ts` folds
  its facts into the meaning of each act, with no life.

The records of the promise are in `tests/corpus`, as `tests/corpus/README.md` says.
