# Examples of furb as a framework

Each door of furb has one example: a program that you run as it stands. Each program opens a life of the engine on
a record, and shows these five things, in this order:

1. A durable life. The program starts itself again as a second process. That process opens a record, gives the
   model a task, and ends while the model waits for the operator. The first process then opens the same record. The
   record answers each word that the model wrote before, so the model is asked nothing again. The program wakes the
   root, the operator answers, and the task closes with its answer.
2. A tree of chains. The word of the model on the root opens the chain `reviewer` and prompts a model on it, in one
   line: `await prompt(str, "Review the plan.", on=reviewer)`.
3. A prompt to the operator. The word of the model asks `await prompt(str, "Ship it?", to=OPERATOR)`. An ear of the
   host, written in the language of the door, answers it.
4. A grant. `grant(usd=0.5)` puts a ceiling on the chain `budget`. Each answer of the model costs 0.3 dollars, so
   the chain pauses after two answers.
5. A door. The model on the chain `door` waits. The operator writes the door of its prompt, `promptN`, with a new
   plan. The chain cancels the old word and runs the new plan, which closes the prompt.

A scripted model answers in place of a real one, through the `answer` of the opening of the life, so an example
costs nothing and asks no network. To ask a real model, remove `answer`, and name the model in the actor.

| Door | Example | Run it | Its test |
| --- | --- | --- | --- |
| Rust, the crate `furb` | [`rust.rs`](rust.rs) | `cargo run --example framework` | `cargo test` runs it as a program |
| TypeScript, the package `@furb/engine` | [`typescript.ts`](typescript.ts) | `bun run build`, then `bun docs/examples/typescript.ts` | `bind/typescript/test/examples.test.ts` |
| python, the packages `furb` and `furb-monty` | [`python.py`](python.py) | `uv run python docs/examples/python.py` | `test/outside/test_examples.py` |

The python example runs on the engine of this interpreter. With `FURB_ENGINE=monty`, it runs the same program on
the engine of the crate, which the package `furb-monty` gives, and its test runs it both ways.

Each program asserts each claim that it shows, and fails when one does not hold. Each test runs its program as a
process, as you run it, so the gates of CI prove each claim on each change.
