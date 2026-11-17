# The promise of furb

This page says what furb promises within a major release, and which test proves each part. The rules come from the
issue #988: R1 to R4, and P2 to P8. R3 replaces P1. The promise binds from 1.0.0, the first stable release (P6). Before
it, at 0.x, each change that the page lists as a break may land, and the page says its fate.

## What a life can do on each build of its major

Each of these acts holds on each build of the major that made the record, with no drift (R1, R3):

- Resume: a life that a stop cut opens at the exact state of its tip and goes on (R2).
- Reopen: a life that ended opens again from its image or from its first entry, at the same state (R2).
- Read: a reader reads the record and shows each chain and each value, with no life.
- Share: a person attaches to a life that a daemon holds, or forks it into a new life.

A record replays on the engine that it holds, under the names that it holds. A record of another record is not
imported. The promise holds from the stable cut, the major 3 of the schema of the record, where the replay stands in
the engine and a record keeps no checkpoint: each record of it holds the texts of its engine, its Kernel and its
checkpointer, so each later build of the major replays it with no drift. A record from before the cut, of an earlier
major or of no schema, opens in no life, which says why: a window reads its
threads, read only, from the fold of its facts, and a reader of a record reads each fact of it. A drift on the same
engine is a defect, never a known limit (R1). Within a major, a later build of the engine never makes a record drift,
since the record replays on the texts that it holds; a build of the host that makes one drift holds a defect, which
is fixed at its root (R3). A life that keeps its record and ran another engine, Kernel or checkpointer of the
major first reaches the exact state of its tip on them, by its record, then crosses: it writes the fact made of the
build that opens it, and goes on with that build. A later open replays each segment of the record on the texts that
its fact made holds, and carries the state of its tip to the next segment in memory, where nothing keeps it. Each
chain keeps as its system prompt the engine
that it read first, and its turns end with one paragraph that holds the lines of the engine that differ, so a chain
only grows.

## What is frozen within a major

| Part | Where | Version now |
| --- | --- | --- |
| The contract: the engine and the interface of each official kit | `src/furb/engine.pyi`, `.agents/skills/<name>/<name>.pyi` | frozen at 1.0.0, as now with the breaks that the last section lists |
| The schema of the record | `RECORD_SCHEMA` in `src/world/store/schema.rs` | 3.1 |
| The schema of the carry of a crossing, which an open makes in memory: pure data, which `seed` of `src/furb/checkpoint.py` reads (P7) | `CHECKPOINT_SCHEMA` in `src/world/store/schema.rs` | 2.1 |
| The protocol of the daemon | `PROTOCOL` in `crates/wire/src/protocol.rs`, `crates/host/protocol.schema.json` | 49 |
| The nodes of a view, a closed set that only grows (P5) | `crates/host/protocol.schema.json` | with the protocol |

A schema only adds within a major, and each reader skips each kind of fact and each field that it does not know. So a
record that a later build of the major made opens on an earlier build of it (P8).

What a minor may change: the default of each policy that the contract marks, WINDOW, TIMEOUT, LARGEST, HEAD and TAIL
(P3), and the inside of the engine, the Kernel, the journal and the doctrine, while each record of the corpus
replays with no drift.

What only a major may change: a name, a signature, a type or the meaning of a verb of the contract (P2); a schema that
an earlier build of the major cannot read; a node or a field of a view that goes; a drift of a record of the corpus
that no adapter holds. What holds across majors is decided at the first major after 1.0.0 (P4).

## The proofs that a machine runs (R4)

R1, no drift on the same engine:

- `tests/test_boot.py::test_a_replay_makes_the_same_acts_in_the_same_order_and_gives_them_the_same_ids`
- `tests/test_boot.py::test_a_fact_that_the_journal_says_again_carries_the_speaker_that_the_record_holds`
- `tests/test_drift.py::test_an_act_whose_words_are_not_the_ones_the_record_holds_is_a_drift`
- `tests/suite/engine.rs::a_life_whose_act_holds_the_address_of_an_object_in_its_words_replays_with_no_drift`
- `tests/suite/promise.rs::a_record_that_a_kill_tore_in_a_write_or_at_a_crossing_opens_as_the_entries_written_whole`, at a
  random byte of a random line, of the done of the stand of the first word of the root and of the last fact made,
  from a seed that it prints
- `tests/suite/promise.rs::a_life_that_a_replay_opens_holds_the_transcript_that_it_held_live`
- `tests/suite/life.rs::a_life_that_enables_its_skills_at_an_open_opens_again_by_its_record_and_plays_each_one_on_a_new_chain`

R2, the exact tip, by the image and by the record, across each crossing (resume and reopen):

- `tests/suite/life.rs::a_record_that_another_engine_went_on_with_after_a_crossing_replays_its_tail_to_the_exact_state_at_its_tip`
- `tests/suite/life.rs::a_life_goes_on_across_a_crossing_with_the_names_the_module_and_the_open_prompt_of_each_chain`
- `src/life.test.rs::a_life_that_a_fault_stopped_inside_a_cut_run_opens_at_the_exact_state_of_its_tip`
- `tests/suite/life.rs::a_record_of_another_engine_crosses_to_this_build_at_its_open`
- `tests/suite/life.rs::an_image_of_a_build_of_other_texts_does_not_load`
- `tests/suite/life.rs::a_life_that_opens_on_a_record_that_a_stop_cut_asks_the_reply_again_and_ends_the_command_with_no_wake`
- `tests/suite/promise.rs::a_life_carried_across_three_crossings_opens_at_its_tip`
- `crates/host/tests/suite/daemon.rs::a_life_that_ended_opens_again_at_the_tip_of_its_record`
- `tests/checkpoint/test_gathered.py::test_the_definitions_of_a_chain_take_no_text_that_the_chain_heard`
- `tests/suite/promise.rs::a_life_killed_at_a_random_fact_opens_at_its_tip_across_its_crossings_and_goes_on`, a
  kill of the process of the life at or after a fact that a seed draws, which it prints with the length of
  the record at the kill, so the torn test plays that kill again as a cut

A fork of a life, its record up to a fact in a directory of its own:

- `tests/suite/promise.rs::a_fork_of_a_life_opens_at_the_state_of_its_origin_at_the_fork_and_goes_on_by_its_own_facts`

R3, no drift within a major, on the texts that each record holds:

- `tests/suite/life.rs::each_record_of_the_corpus_opens_by_its_image_to_the_state_of_its_record`
- `tests/suite/promise.rs::each_record_of_each_folder_of_the_corpus_opens_gives_back_its_modules_and_takes_one_more_turn`, which reads each record with the reader too

The schemas (P7, P8):

- `tests/suite/life.rs::a_record_of_another_major_of_its_schema_is_refused_with_both_schemas_and_keeps_nothing`
- `tests/suite/world/schema.rs::a_schema_from_before_the_stable_cut_opens_in_no_life`
- `tests/suite/life.rs::a_record_from_before_the_stable_cut_opens_in_no_life`
- `crates/host/tests/suite/life.rs::a_record_from_before_the_stable_cut_shows_its_threads_to_a_window_and_says_why`

The protocol, the nodes of a view and the contract (P2, P5):

- `crates/host/tests/suite/protocol.rs::the_schema_on_disk_is_the_schema_of_the_protocol`
- `crates/host/tests/suite/protocol.rs::each_change_of_the_schema_raises_the_protocol`
- `tests/outside/test_contract_diff.py::test_a_node_of_a_view_that_goes_is_a_break`
- `tests/outside/test_contract_diff.py::test_a_break_that_the_page_of_the_promise_does_not_name_fails_whatever_the_title`
- `crates/host/tests/suite/daemon.rs::a_life_that_its_open_attaches_has_a_holder_before_a_grace_of_none_passes`
- `crates/host/tests/suite/daemon.rs::a_drift_at_the_open_of_a_life_tells_its_window_why_and_leaves_the_daemon`

`tests/suite/promise.rs::each_proof_that_the_page_of_the_promise_names_stands_in_its_file` fails when a test that this page
names is gone. `cargo xtask contract` fails on a break of the contract that this page does not list below.

What no machine proves yet, so the promise does not hold there until a proof lands:

- A rollback: `tests/outside/test_promise_rollback.py::test_each_record_of_next_opens_on_the_last_release_of_its_major_once_that_release_holds_a_corpus`
  opens each record of the corpus on the last release of its major, but it skips until a release of the major holds a
  folder of the corpus (P8).
- A replay with random delays of the bus and of each ear, and with ears that raise (replay-chaos).
- A replay across the builds of the major, with a count of the drifts of the corpus (drift-gate).
- A life that crossed: its live transcript holds the facts of the word that seeds it from the carry otherwise than its
  open, so the fork proof compares a fork with the open of its origin, not with the live origin.

## The breaks of the contract since v0.4.2

Each break below came before 1.0.0, so no promise covered it (P6). A record that a build before it made replays on the
engine and on the text of each kit that the record holds, so it needs no adapter. A word of a caller that uses the old
form must change, and so these breaks stand for the next major, 1.0.0.

- `src/furb/engine.pyi Show: the alias changes from Callable[[list[str]], list[int]] to tuple[int, int] | str | list[str] | Callable[[list[str]], list[int]]`: a show of the file is plain data, so a replay tells the same; a callable show stays valid.
- `src/furb/engine.pyi stand: its type of return changes from Standing to Act[Standing]`: a word awaits the stand.
- `src/furb/engine.pyi chain: the type of filter changes from Filter | None to Filter | list[str] | None`: a filter may be the ids that it kept; a callable filter stays valid.
- `src/furb/engine.pyi gate: the def goes`: no gate reads a word before it runs; a statement that is not python, or that names what nothing binds, raises where it runs.
- `src/furb/engine.pyi Gate: the alias goes`: as gate.
- `src/furb/engine.pyi boot: the type of record changes from Sequence[Entry] to Iterable[Entry]`: boot says each entry again as it reads it, so a host gives the lines of a record as they come.
- `src/furb/engine.pyi Entry: the alias changes from tuple[Fact] to tuple[Fact, int, bool] | tuple[Fact]`: an entry holds the place of its fact and whether the life rested when it said it, so a replay says it at that place; a record of an earlier major opens in no life.
- `src/furb/engine.pyi Entry: the alias changes from tuple[Fact, int, bool] | tuple[Fact] to tuple[Fact, int, bool, int] | tuple[Fact, int, bool] | tuple[Fact]`: an entry holds the count of the facts that waited for their ears as the life said it, so a replay says each answer of the outside where the life said it; a record of the schema 3.0 replays on the texts that it holds, and each listed text of the major gets a dispatch that says such an answer where its life said it.
- `src/furb/engine.pyi Drift: the base Exception goes`: Drift is a BaseException, so no `except Exception` of a word or of an ear holds a drift, which ends what the life keeps; a word that held one ran on with a journal that keeps nothing.
- `src/furb/engine.pyi grant: the def goes`: grant is the official kit grant, off by default; a word loads it with `await skill("grant")`, and the grant of each door, and the `Grant` door of the host, load it in the word that they run.
- `src/furb/engine.pyi Grant: the alias goes`: as grant; the contract of the kit holds it.
- `.agents/skills/compaction/compaction.pyi carry: the def goes`: compaction keeps a chain by supersede.
- `.agents/skills/compaction/compaction.pyi supersede: its type of return changes from str to Act[str]`: supersede gives the jump of the chain, which an await gives as the chain that it jumped to, and each name of the chain names that chain after the jump.
- `.agents/skills/compaction/compaction.pyi compact: its type of return changes from str to Act[str]`: compact gives the jump of the chain, which an await gives as the chain that it jumped to, and each name of the chain names that chain after the jump.
- `.agents/skills/compaction/compaction.pyi condense: its type of return changes from str to Act[str]`: condense gives the jump of the chain, which an await gives as the chain that it jumped to, and each name of the chain names that chain after the jump.
- `.agents/skills/compaction/compaction.pyi inherit: the def goes`: as carry.
- `.agents/skills/compaction/compaction.pyi imported: the def goes`: as carry.
- `.agents/skills/compaction/compaction.pyi CHOOSER: the const goes`: choice asks on the chain the model that wrote its last reply, which reads the chain from its cache.
- `.agents/skills/compaction/compaction.pyi LISTED: the const goes`: the model of the chain read each act, so the prompt of choice lists none.
- `.agents/skills/compaction/compaction.pyi choice: the parameter to goes`: as CHOOSER.
- `.agents/skills/compaction/compaction.pyi chosen: the parameter to goes`: as CHOOSER.
- `.agents/skills/compaction/compaction.pyi condense: the parameter to goes`: as CHOOSER.
- `.agents/skills/memory/memory.pyi Memory: the alias goes`: memory reads its files with reads in the word.
- `.agents/skills/ouroboros/ouroboros.pyi LORE: the const goes`: the kit ouroboros was rewritten.
- `.agents/skills/ouroboros/ouroboros.pyi ouroboros: the def goes`: as LORE.
- `.agents/skills/ouroboros/ouroboros.pyi extend: the parameter tui goes`: the old window, which loaded the part that this parameter wrote, is removed; a kit of the session names its view with view.
- `.agents/skills/ouroboros/ouroboros.pyi: the contract goes`: the kit ouroboros is the kit furb-kit, with the same names and laws in `.agents/skills/furb-kit/furb-kit.pyi`; a record that enabled ouroboros plays the word that its fact enable holds, and a config turns on `furb-kit` in its place.
- `.agents/skills/skills/skills.pyi skill: its type of return changes from Text to Act[Text]`: a word awaits the skill.

The sentences of the contract that changed since v0.4.2 stand for 1.0.0 too. After it, a sentence that changes its
meaning is a major, and `cargo xtask contract` names each one.
