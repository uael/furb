# The records of the replay

Each record here is a life that a build of the current version made. `VERSION` in `src/world/store.rs` names that
version. The test `each_record_of_the_replays_names_this_version_and_replays_on_this_build_with_no_drift` in
`src/life.test.rs` replays each record. It runs on each build, in `cargo test`. It uses a life that only inspects the
record. So it keeps the number of versions low.

These records are not in the promise of a release. `tests/corpus` holds those records, as `tests/corpus/README.md`
says.

- A change that replays each record to the same acts keeps the version. A life that a build of the version made goes on
  with the new build. It needs no switch.
- A change that makes a record drift adds one to `VERSION`. The same change makes the record again, or cuts it to the
  acts that it proves.

The tests of `src/life.test.rs` also make the life `worked`, on a model that answers from a script, and replay it. Its
root writes, reads, runs a command, waits, reads the clock and chance, and asks a chain of its own. Each official
extension runs on it.

`writing.jsonl` is the head of a life of 10-28. It is cut to 33 entries, and to the acts that make it drift. A build
made it before the writing became a fact. So its record holds each part of a word that ran, as its model wrote it, as a
question: `("writing", writingN, by, chain, rung, part, before)`. No later life makes that question again, because it
asks no model again. So the part ran too late, only at the run of its rung. A rung that the part cancelled in the life
asked its model in the replay. The store now reads each such question as the writing fact. The journal says that fact
again where the life said it. So the Kernel runs the part there.

`refused.jsonl` holds two replies. The first is a reply of the root that nothing took. The life refused it with no
maker, `nothing takes reply`, so the root paused. The second is a reply of a second chain, after the first. The journal
says that refusal again under no site, as the life said it. So the root pauses again, and the second chain asks for the
next reply, as in the life.

`wake.jsonl` holds two commands that a life left running. A later life wakes the root. The World then closes both
commands as interrupted at once, when it hears that wake. A word awaited the first command. That word reads the clock
when both are over, and chance when only its own is over. The journal says those answers again at once, at the wake, as
the World said them. So the word reads the clock again, as in the life.

`handover/checkpoint.jsonl` is a life that ended with a checkpoint of the form before `schema`. That form holds the
entries that the journal replays, in the place of the data. The life of the owner wrote that form at its handover. So
the first build with the data must open it. The test `a_checkpoint_of_the_form_before_its_schema_opens_on_this_build` in
`src/life.test.rs` opens it on a build of another engine. It stands apart from the replays, because its crafted word
says a prefix that no turn shows. It and the reader of that form in `src/checkpoint.rs` go away once each life of that
form has crossed to the data. The next checkpoint of each life does that.

`handover/relife.jsonl` is a life of that form. Its second life, born from the first checkpoint, also ended with a
checkpoint. The record is cut to its last checkpoint. Each text of a turn over 2000 characters is cut to 300. That form
writes the turns of a chain after the stand of the root. So the root seeded itself again at that stand, and the next
open drifted at `gate1`. The reader of that form puts the turns of each chain just after the chain. The test
`a_checkpoint_of_the_form_before_its_schema_that_a_later_life_wrote_opens_and_seeds_no_root_again` in `src/life.test.rs`
proves it.
