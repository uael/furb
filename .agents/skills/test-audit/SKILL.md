---
name: test-audit
description: "Use whenever you write, change, review, or sweep tests of furb. The gate for a new test, and the audit of tests that restate the source, repeat a stronger proof, tie a behavior to its implementation, or keep a seam alive that only a test uses."
license: Adapted from the skill test-audit of openclaw at 9a75b9b, under the terms in LICENSE.txt
---

# Test Audit

Three modes hold one bar of value. The authoring mode gates each new or changed test when you write it. The audit
mode sweeps one part for tests of low value. The campaign mode prunes the whole test surface of one part in one
pull request; read [CAMPAIGN.md](CAMPAIGN.md) before you start one. The aim is confidence, not a count of deleted
tests.

## The suites of furb

`CLAUDE.md` gives the laws of each suite. Read it first, and read `src/furb/CLAUDE.md` for the laws of the engine.

- The contract suite, `test/`, holds one test for each sentence of `src/furb/engine.pyi`, and the suite of each
  extension, `extensions/<name>/test/`, does the same for its contract. `test/test_hygiene.py` holds each suite to
  its laws. A test of these suites is owned by its sentence: you never delete it alone. When its assertions do not
  prove its sentence, repair them. When the sentence is wrong, the owner decides, and the sentence and its test
  change together.
- `test/outside/` proves the World, the Kernel, the door to python, and the switch of the package. The hygiene laws
  do not hold these tests, so this gate holds them.
- The tests of the crate stand beside each module, `src/<name>.test.rs`, and `cli/tests/furb.rs` runs the command
  line as a process.
- The TypeScript tests stand in `bind/typescript/test/` and `tui/test/`.

Each contract has one primary test at its strongest boundary. A sentence of the contract is proved once, by the
contract suite, on both engines. A test of a door proves what the door carries that no sentence says. A test of
the crate proves what the crate adds under the engine. A test of the TUI proves what a person sees and does.

## Authoring gate

Before you add a test, answer these four questions. When an answer is missing, do not add the test.

1. What observable behavior, law, or contract does it protect? In the contract suite, the answer is its sentence.
2. What regression that can occur makes it fail?
3. Why does the coverage that exists not catch that regression? A second layer needs a risk of its own, such as a
   value that only the door carries, or a process that only the command line starts. Extend a case or a shared
   helper of `test/conftest.py` before you write a near copy of a test.
4. Does it need a seam in the product, such as an export, a flag, a wrapper or a hook, that no product caller needs?
   If it does, move the test to the real boundary.

Then check the test against each [junk pattern](#junk-patterns). A match fails the gate unless the
[retention bar](#retention-bar) names the contract that the test alone guards. A test that breaks when a refactor
keeps the behavior asserts the implementation. Write it again at the boundary that owns the behavior.

A regression test must fail on the code before the fix, for the reason it names, and pass after the fix. A
regression test that never failed proves its double, not the fix. One regression at the boundary that owns the bug
covers it; do not play the same scene again at each layer it crosses.

## Junk patterns

The authoring gate refuses a new test that matches one of these, and an audit looks for the tests that match.

- a test with no assertion, which only runs code;
- a value compared with itself, or a copy of what the test gave;
- a copy of a fixture, a list of names, or a list of exports;
- a grep of the exact source, an import, or a string;
- a test of a private predicate or of the shape of a call, which a real boundary proves again;
- a second call of the same contract;
- a test whose only purpose is to keep an export, a global or a wrapper that only tests use;
- product code that only tests call;
- an expected value that the helper or the renderer under test made;
- a double that holds the behavior that the test asserts, or one double that stands in for two different APIs;
- a fixture that gives the order or the answer that the owner must make itself;
- a negative control that passes for another reason, such as a refusal of a different guard;
- a name or a docstring that promises more than the input tries.

These are particular to furb:

- a test of the contract suite that reads a name the contract does not give, or reads a private name of
  `engine.py`, when `engine.<verb>(...)` reaches the same behavior;
- an assertion that is weaker than its sentence, such as `in` where the sentence gives an exact value, or a check
  of the kind of a fact where the sentence gives the fact;
- a test of a door or of the crate that proves a sentence of the contract again, which the contract suite proves on
  both engines.

## Value bar

A test earns its cost when it protects a behavior, a regression that can occur, or a contract that means something
alone. In an audit, a test that must change when a refactor keeps the behavior is a suspect, not a sure deletion.
The authoring gate still refuses a new one.

Before you judge a candidate, read the whole test and the code that owns its behavior: the entry point, the
callers, the callees, the tests that overlap it, the gate of CI that runs it, and its history. When the test
claims a behavior of a dependency, such as monty, rig or napi, read that dependency.

## Discovery

Keep discovery read-only, and report the evidence before you edit. For a wide scope, split the work into lanes:

- the engine and its contract suite, `src/furb/` and `test/`;
- the extensions, `extensions/`;
- the crate and the command line, `src/*.rs` and `cli/`;
- the doors, `src/binding*`, `bind/python` and `bind/typescript`;
- the TUI, `tui/`;
- one sweep across all of them for one pattern.

Outside a campaign, prefer a few candidates you are sure of to a long list of guesses.

## Retention bar

Keep a test when it alone enforces a contract: a sentence of a contract, a hygiene law, the wire and the record, a
default, a byte of the system prompt, the crossing of a value through a door, a process of the command line, or a
screen of the TUI. Also keep:

- a test of an order of calls, when the order is a behavior someone sees;
- a regression test whose failure can occur;
- a check of the source, when it is the cheapest guard that fails when the contract changes and that survives a
  rename, as the hygiene laws are;
- a kept test that fails on the base: take it as a possible bug of the product, reproduce it, and repair the owner.

A test that is slow or that reads the source is not deleted for that. A test that looks like the implementation may
still be the only guard of a contract; prove that it is not before you remove it.

## Candidate evidence

Record each field before you edit. A candidate with a missing field is not ready to delete.

- the name and the place of the test;
- the failure it can detect;
- the callers of the product seam it covers, other than tests;
- the stronger proof that remains at the boundary that owns the behavior, or why no proof is needed;
- the history, and why the test or the seam exists;
- the product code or the test support that its deletion lets go;
- the risk, and the command that proves the change.

## Edit shape

Take one batch that one owner holds. Delete the exports, the globals, the wrappers and the dead product paths that
only tests used, and keep no alias. Move a kept regression to the owner it belongs to. Put repeated assertions of one
contract into one test.

Prefer a change that removes lines of the product. Do not add a test that says the implementation again, and do not
turn a candidate you are not sure of into a cleanup to raise a count.

## Validation

The commands are in `CLAUDE.md`, and the `testing` skill says which one proves which change.

1. Run the tests of the owner and of its siblings: `uv run pytest -q <path>`, `cargo test <filter>`, or
   `bun run test`, which gives the tests the config and the cache of `.furb/tests`.
2. When you remove a check of the source, run the command that owns the real contract.
3. Format, then run `git diff --check`.
4. Run the gates that the change needs, and `uv run pytest -q test/test_hygiene.py` for any change of a suite.
5. Read `git diff --numstat`, and count the product apart from the tests and their support.

## Landing

Commit and push only when the owner asks. Land one coherent pull request at a time. After it lands, start again from
the current `main`, and run the read-only discovery again for the next batch.

## Handoff

Report:

- the categories of low value you removed, and why they stood;
- what the change made simpler in the product;
- the candidates you kept, and why they have value;
- the proof you ran, the focused one and the whole one;
- the lines of the product and the lines of the tests, apart;
- the state of the pull request;
- the work that is left, by name.
