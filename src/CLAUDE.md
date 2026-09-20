# The crate

`src/` serves both things: `src/furb/engine.py` is the engine, and the `.rs` files beside it are the crate that
runs that same file in a sandbox and gives its surface to a host that is not python. One file, one engine, so the
engine a host runs and the engine a model reads are never two things.

## The proof

**`script/sanded.py` is the proof of the crate, and no other number is one.**

It runs the suite in `test/`, in this interpreter, under pytest, unedited. What that suite drives is not the
engine of this interpreter: it is a life of `furb_sand`, which is the engine in monty with the Kernel of the
crate, and every verb the suite calls is said as a word in there. The tests, the World doubles and the assertions
all stand here. Nothing of the suite enters the sandbox.

That is the only shape that means anything. The contract has 672 sentences and the suite holds one test to each,
so a crate that passes them is a crate that carries the engine whole. A rig that runs the suite against the
engine of this interpreter proves the rig and not the crate, whatever it reports.

Three such rigs were written and then removed, because each reported a green number that was not the number:

- `wired.py` ran the suite with the World doubles behind the plain boundary, engine in this interpreter.
- `kerneled.py` ran the suite with the Kernel of the crate under it, engine in this interpreter.
- `inside.py` ran the suite with no pytest, so that it could be carried into a sandbox, which it never was.

They are gone. `script/sanded.py` is what stands.

`tests/life.rs` stays: it drives one life of the real engine end to end in monty, on a World written small, and it
is what a reader of the crate opens first. It is 19 scenarios and it is no proof of the contract.

## How the proof reaches the engine

`boot` is the seam. `conftest.life` calls `engine.boot(record, kernel=..., probe=..., world=...)`, and the rig
answers it with a life of the binding. The harness hands over the generators its doubles make, and each double
stands in the frame of its generator: the World the life asks, and the gate the Kernel reads a word by, which the
harness's `Py` answers as it stands.

A World double reads the engine while it answers, which is `engine.cwd(on=...)` in the middle of a read and
`id in engine.acts` before that. Nothing may call into a life that stands waiting for it, so the double is driven
on a thread of its own: it blocks where it reads, the boundary is told the word, and the value wakes it. What a
World says when nothing asked it goes through the Voice.

## What the boundary owes the contract

The World the contract declares is a generator beside the engine, so it reads the engine where it answers. A
World of a host is not beside it, so the boundary must give it that same reach, and `Reply::Reads` is how: one
word, run in the names of the engine, handed back the way an ask is. It is no new reach into the sandbox, since
the word runs in the names of the engine and not in the globals of a chain.

When the suite fails against the crate and the engine is right, look here first: the fault is usually that the
boundary is narrower than the World the contract declares.
