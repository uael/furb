# The records of the replay

Each record here is a life of an earlier build. Its fact made holds the texts of its engine and of its Kernel, so each
later build replays it on them. The test `each_record_of_the_replays_replays_on_its_own_engine_with_no_drift` in
`src/life.test.rs` replays each record. It runs on each build, in `cargo test`. It uses a life that only inspects the
record.

These records are not in the promise of a release. `tests/corpus` holds those records, as `tests/corpus/README.md`
says.

- A change of the engine or of the Kernel changes no record here, since each replays on its own texts.
- A change of the host that makes a record drift is a defect of that change. The host drives each engine of the major.
- A record of a build before the texts takes them once from git with `python3 script/engines.py <record>`.
  The three records at the top of this folder do not name the engine that made them. `refused.jsonl` and
  `writing.jsonl` were made on the engine `461b240e`, which cannot replay them, since each was cut later to prove a
  fix of a build after it. A build of a worktree made `wake.jsonl` on the engine `af2eeb28`, which no commit holds.
  Each names the engine `d67479fe` of the commit `a4e4f6058`, which last changed the three and replayed them with no
  drift, and holds its texts. The records of `handover` replay nothing after their checkpoint, so they hold no texts.

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

`ranks.jsonl.zst` is the tail of the life `5eb81f9c` of 11-05 after its last checkpoint, which the engine `5f66c73c`
made. Its checkpoint is cut: it keeps each chain and each act, with no turns, no values and no long value of an act. It
is kept by zstd with a long window, 10.1 MB plain and 0.19 MB kept, and the harness reads it through zstd. On its own
engine it drifts at `rung3230`, since that engine says the first started or done of an act of the outside before the
facts that the record holds before it. The adapter of that engine in `src/furb/adapters.py` carries the ranks of #1329,
so the record replays with no drift. The test `a_record_of_an_engine_that_an_adapter_names_drifts_on_that_engine_without_the_adapter`
in `src/life.test.rs` proves the drift without the adapter.

`lives.jsonl.zst` is a life of the engine `0a11688b` of the commit `d5550efda`, with the default extensions. Its root
runs a word that makes an act whose ear makes an act of its own, as compaction makes the prompt of its chooser. The
gathered of that engine reads slot 4 of the maker of each open act, which that inner act has not, so it raises
IndexError and the life writes no checkpoint. The adapter of that engine in `src/furb/adapters.py` gives it the lives
of a later engine. The test `a_life_of_an_engine_whose_gathered_raised_on_an_act_that_no_rung_made_writes_its_checkpoint_through_its_adapter`
writes its checkpoint, which carries the life word of compaction.
