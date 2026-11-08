# The promise of furb

This page says what furb promises within a major release, and which test proves each part. The rules come from the
issue #988: R1 to R4, and P2 to P8. R3 replaces P1. The promise binds from 1.0.0, the first stable release (P6). Before
it, at 0.x, each change that the page lists as a break may land, and the page says its fate.

## What a life can do on each build of its major

Each of these acts holds on each build of the major that made the record, with no drift (R1, R3):

- Resume: a life that a stop cut opens at the exact state of its tip and goes on (R2).
- Reopen: a life that ended opens again from its checkpoint or from its first entry, at the same state (R2).
- Read: a reader reads the record and shows each chain and each value, with no life.
- Share: a person attaches to a life that a daemon holds, or forks it into a new life.

A record replays on the engine that it holds, under the names that it holds. A record of another record is not
imported. A drift on the same engine is a defect, never a known limit (R1). Within a major, a later build of the
engine never makes a record drift. Where a later build fixes a defect of replay at its root, an adapter of the old
engine holds the record, in the one home of adapters, `src/furb/adapters.py`, keyed by the digest of the engine (R3).

## What is frozen within a major

| Part | Where | Version now |
| --- | --- | --- |
| The contract: the engine and the interface of each official kit | `src/furb/engine.pyi`, `.agents/skills/<name>/<name>.pyi` | frozen at 1.0.0, as now with the breaks that the last section lists |
| The schema of the record | `RECORD_SCHEMA` in `src/world/store/schema.rs` | 1.1 |
| The schema of a checkpoint: pure data, which `seed` of `src/furb/engine.py` reads (P7) | `CHECKPOINT_SCHEMA` in `src/world/store/schema.rs` | 2.0 |
| The protocol of the daemon | `PROTOCOL` in `host/src/protocol.rs`, `host/protocol.schema.json` | 9 |
| The nodes of a view, a closed set that only grows (P5) | `host/protocol.schema.json` | with the protocol |

A schema only adds within a major, and each reader skips each kind of fact and each field that it does not know. So a
record that a later build of the major made opens on an earlier build of it (P8).

What a minor may change: the default of each policy that the contract marks, WINDOW, TIMEOUT, LARGEST, HEAD and TAIL
(P3), and the inside of the engine, the Kernel, the journal, the gate and the doctrine, while each record of the corpus
replays with no drift.

What only a major may change: a name, a signature, a type or the meaning of a verb of the contract (P2); a schema that
an earlier build of the major cannot read; a node or a field of a view that goes; a drift of a record of the corpus
that no adapter holds. What holds across majors is decided at the first major after 1.0.0 (P4).

## The proofs that a machine runs (R4)

R1, no drift on the same engine:

- `test/test_boot.py::test_a_replay_makes_the_same_acts_in_the_same_order_and_gives_them_the_same_ids`
- `test/test_boot.py::test_a_fact_that_the_journal_says_again_carries_the_speaker_that_the_record_holds`
- `test/test_drift.py::test_an_act_whose_words_are_not_the_ones_the_record_holds_is_a_drift`
- `src/engine.test.rs::a_life_whose_act_holds_the_address_of_an_object_in_its_words_replays_with_no_drift`
- `src/life.test.rs::each_record_of_the_replays_replays_on_its_own_engine_with_no_drift`
- `src/life.test.rs::a_config_that_turns_the_gate_off_runs_a_word_that_the_gate_refuses_and_its_replay_with_the_gate_on_holds_the_same_facts`

R2, the exact tip, with a checkpoint and without one (resume and reopen):

- `src/life.test.rs::a_record_that_another_engine_went_on_with_after_a_checkpoint_replays_its_tail_to_the_exact_state_at_its_tip`
- `src/life.test.rs::a_life_goes_on_from_a_checkpoint_with_the_names_the_module_and_the_open_prompt_of_each_chain`
- `src/life.test.rs::a_life_that_opens_on_a_record_that_a_stop_cut_asks_the_reply_again_and_ends_the_command_with_no_wake`
- `tests/promise.rs::a_life_born_again_three_times_opens_from_its_first_entry_to_the_state_of_its_last_checkpoint`
- `host/tests/daemon.rs::a_life_that_ended_opens_again_from_its_checkpoint`
- `test/test_gathered.py::test_the_definitions_of_a_chain_take_no_text_that_the_chain_heard`

R3, no drift within a major, through the one home of adapters:

- `src/engine.test.rs::a_record_of_an_older_engine_opens_through_its_adapter_and_keeps_no_fact_of_its_record_again`
- `src/engine.test.rs::a_kernel_of_the_major_from_before_it_ended_its_lost_runs_ends_them_through_its_adapter`
- `src/life.test.rs::a_record_of_an_engine_that_an_adapter_names_drifts_on_that_engine_without_the_adapter`
- `src/life.test.rs::a_record_that_names_no_engine_replays_as_before_and_names_none`
- `tests/promise.rs::each_record_of_each_folder_of_the_corpus_opens_gives_back_its_modules_and_takes_one_more_turn`, which reads each record with the reader too

The schemas (P7, P8):

- `src/life.test.rs::a_checkpoint_of_a_later_minor_of_its_schema_with_fields_more_opens_with_each_chain_and_its_prompts`
- `src/life.test.rs::a_checkpoint_of_another_major_of_its_schema_is_refused_with_both_schemas`
- `src/life.test.rs::a_record_of_another_major_of_its_schema_is_refused_with_both_schemas_and_keeps_nothing`
- `tests/promise.rs::a_record_whose_checkpoint_holds_a_field_that_no_reader_knows_still_opens`

The protocol, the nodes of a view and the contract (P2, P5):

- `host/tests/protocol.rs::the_schema_on_disk_is_the_schema_of_the_protocol`
- `host/tests/protocol.rs::each_change_of_the_schema_raises_the_protocol`
- `test/outside/test_contract_diff.py::test_a_node_of_a_view_that_goes_is_a_break`
- `test/outside/test_contract_diff.py::test_a_break_that_the_page_of_the_promise_does_not_name_fails_whatever_the_title`
- `host/tests/daemon.rs::a_life_that_its_open_attaches_has_a_holder_before_a_grace_of_none_passes`
- `host/tests/daemon.rs::a_drift_at_the_open_of_a_life_tells_its_window_why_and_leaves_the_daemon`

`tests/promise.rs::each_proof_that_the_page_of_the_promise_names_stands_in_its_file` fails when a test that this page
names is gone. `script/contract.py` fails on a break of the contract that this page does not list below.

What no machine proves yet, so the promise does not hold there until a proof lands:

- A rollback: `test/outside/test_promise_rollback.py::test_each_record_of_next_opens_on_the_last_release_of_its_major_once_that_release_holds_a_corpus`
  opens each record of the corpus on the last release of its major, but it skips until a release of the major holds a
  folder of the corpus (P8).
- A fork of a life: no test opens a fork at the state of its origin.
- A stop at a random fact, and a replay with random delays and ears that raise (stop-random, replay-chaos).
- A replay across the builds of the major, with a count of the drifts of the corpus (drift-gate).

## The breaks of the contract since v0.4.2

Each break below came before 1.0.0, so no promise covered it (P6). A record that a build before it made replays on the
engine and on the text of each kit that the record holds, so it needs no adapter. A word of a caller that uses the old
form must change, and so these breaks stand for the next major, 1.0.0.

- `src/furb/engine.pyi Show: the alias changes from Callable[[list[str]], list[int]] to tuple | Callable[[list[str]], list[int]]`: a show of the file is plain data, so a replay tells the same; a callable show stays valid.
- `src/furb/engine.pyi stand: its type of return changes from Standing to Act[Standing]`: a word awaits the stand.
- `src/furb/engine.pyi chain: the type of filter changes from Filter | None to Filter | list[str] | None`: a filter may be the ids that it kept; a callable filter stays valid.
- `.agents/skills/compaction/compaction.pyi carry: the def goes`: compaction keeps a chain by supersede.
- `.agents/skills/compaction/compaction.pyi inherit: the def goes`: as carry.
- `.agents/skills/compaction/compaction.pyi imported: the def goes`: as carry.
- `.agents/skills/memory/memory.pyi Memory: the alias goes`: memory reads its files with reads in the word.
- `.agents/skills/ouroboros/ouroboros.pyi LORE: the const goes`: the kit ouroboros was rewritten.
- `.agents/skills/ouroboros/ouroboros.pyi ouroboros: the def goes`: as LORE.
- `.agents/skills/skills/skills.pyi skill: its type of return changes from Text to Act[Text]`: a word awaits the skill.

The sentences of the contract that changed since v0.4.2 stand for 1.0.0 too. After it, a sentence that changes its
meaning is a major, and `script/contract.py` names each one.
