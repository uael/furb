# Examples

These five short programs show furb as a library: a life that your own program opens, asks, and steers. Each one
shows one idea, in each of the three languages of furb: Python, with the package `furb`; TypeScript, with the package
`furb`; and Rust, with the crate `furb`. The three read the same, since each gives the same names:

- `open` opens a life on its record, or goes on with it.
- `prompt` asks a model, and gives a thread that you await for its answer.
- `chain` opens a chain, a conversation with its own state.
- `questions` gives each question that a model asks you.
- `grant` puts a dollar ceiling on a chain.
- A thread has its `plan`, the Python that its model will run, which `edit` changes.

## Run them

Each example asks the default model, which is the claude command line when your machine has it, as
[Models](../models.md) says. Run an example in a directory of its own, since it keeps its record there. From a clone
of this repository:

- Python: `uv run python docs/examples/python/durable.py`
- TypeScript: `bun run build`, then `bun docs/examples/typescript/durable.ts`
- Rust: `cargo run --example durable`

A program that runs again on its record makes its prompts, chains and grants in the same order, so each one finds
the act that it made before, done or still at work. A program that changed between two runs makes a new act where it
no longer agrees with the record, and the act of the record goes on as work that waits.

The tests run each example as a process, on a claude command line that answers from a script:
`test/outside/test_examples.py`, `bind/typescript/test/examples.test.ts` and `tests/examples.rs`.

## A durable life

Run it, end the process while the model works, and run it again: the second run finds its prompt in the record and goes on, with no word of the model asked twice.

```python
import asyncio

import furb


async def main() -> None:
  life = await furb.open("release.jsonl")
  plan = await life.prompt(str, "Plan the release of version 2.")
  print(plan)


asyncio.run(main())
```

```ts
import { open } from "furb";

await using life = await open("release.jsonl");
console.log(await life.prompt("str", "Plan the release of version 2."));
```

```rust
use std::error::Error;

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("release.jsonl").await?;
    let plan = life.prompt::<String>("Plan the release of version 2.")?;
    println!("{}", life.wait(plan).await?);
    Ok(())
  })
}
```

## A tree of chains

A reviewer on a chain of its own, with a model of its own, reviews the draft of the root.

```python
import asyncio

import furb


async def main() -> None:
  life = await furb.open("notes.jsonl", roster=["claude-cli:opus", "claude-cli:sonnet"])
  reviewer = life.chain("reviewer", model="claude-cli:sonnet")
  draft = await life.prompt(str, "Write the release notes of version 2.")
  review = await reviewer.prompt(str, f"Review these release notes:\n{draft}")
  print(review)
  for chain in life.chains():
    print(chain.label, "under", chain.parent.label)


asyncio.run(main())
```

```ts
import { open } from "furb";

await using life = await open("notes.jsonl", { roster: ["claude-cli:opus", "claude-cli:sonnet"] });
const reviewer = life.chain("reviewer", { model: "claude-cli:sonnet" });
const draft = await life.prompt("str", "Write the release notes of version 2.");
console.log(await reviewer.prompt("str", `Review these release notes:\n${draft}`));
for (const chain of life.chains()) console.log(chain.label, "under", chain.parent?.label);
```

```rust
use std::error::Error;

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("notes.jsonl").roster(["claude-cli:opus", "claude-cli:sonnet"]).await?;
    let reviewer = life.chain("reviewer")?;
    life.model(&reviewer, "claude-cli:sonnet")?;
    let draft = life.prompt::<String>("Write the release notes of version 2.")?;
    let draft = life.wait(draft).await?;
    let review = life.prompt_on::<String>(&reviewer, &format!("Review these release notes:\n{draft}"))?;
    println!("{}", life.wait(review).await?);
    for chain in life.chains()? {
      let parent = life.parent(&chain)?.expect("a chain under the root");
      println!("{} under {}", life.label(&chain)?, life.label(&parent)?);
    }
    Ok(())
  })
}
```

## A prompt to the operator

The model asks you before it deploys, and your program answers each question that it puts to the operator.

```python
import asyncio

import furb


async def main() -> None:
  life = await furb.open("deploy.jsonl")
  deploy = life.prompt(str, "Deploy version 2. Ask me before you deploy.")
  async for question in life.questions(until=deploy):
    question.answer(input(question.message + " "))
  print(await deploy)


asyncio.run(main())
```

```ts
import { createInterface } from "node:readline";
import { open } from "furb";

await using life = await open("deploy.jsonl");
const terminal = createInterface({ input: process.stdin });
const lines = terminal[Symbol.asyncIterator]();
const deploy = life.prompt("str", "Deploy version 2. Ask me before you deploy.");
for await (const question of life.questions({ until: deploy })) {
  process.stdout.write(`${question.message} `);
  question.answer((await lines.next()).value ?? "");
}
terminal.close();
console.log(await deploy);
```

```rust
use std::{error::Error, io::Write};

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("deploy.jsonl").await?;
    let deploy = life.prompt::<String>("Deploy version 2. Ask me before you deploy.")?;
    while let Some(question) = life.question(&deploy).await? {
      print!("{} ", question.message);
      std::io::stdout().flush()?;
      let mut line = String::new();
      std::io::stdin().read_line(&mut line)?;
      question.answer(&mut life, line.trim())?;
    }
    println!("{}", life.wait(deploy).await?);
    Ok(())
  })
}
```

## A grant

A ceiling of fifty cents pauses the chain, and a larger grant lets it go on.

```python
import asyncio

import furb


async def main() -> None:
  life = await furb.open("budget.jsonl")
  budget = life.chain("budget")
  budget.grant(usd=0.50)
  count = budget.prompt(int, "Count the lines of python in this directory.")
  await budget.paused()
  print(f"paused at {budget.spent:.2f} dollars")
  budget.grant(usd=2.00)
  budget.wake()
  print(await count)


asyncio.run(main())
```

```ts
import { open } from "furb";

await using life = await open("budget.jsonl");
const budget = life.chain("budget");
budget.grant({ usd: 0.5 });
const count = budget.prompt("int", "Count the lines of python in this directory.");
await budget.paused();
console.log(`paused at ${budget.spent.toFixed(2)} dollars`);
budget.grant({ usd: 2 });
budget.wake();
console.log(await count);
```

```rust
use std::error::Error;

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("budget.jsonl").await?;
    let budget = life.chain("budget")?;
    life.grant(&budget, 0.50)?;
    let count = life.prompt_on::<i64>(&budget, "Count the lines of python in this directory.")?;
    life.paused(&budget).await?;
    println!("paused at {:.2} dollars", life.spent(&budget)?);
    life.grant(&budget, 2.00)?;
    life.wake(&budget)?;
    println!("{}", life.wait(count).await?);
    Ok(())
  })
}
```

## A door

You change the plan of the model while it works: an edit of the door of the thread cancels the old word and runs the new plan.

```python
import asyncio

import furb


async def main() -> None:
  life = await furb.open("door.jsonl")
  release = life.prompt(str, "Release version 2 in one hour.")
  await release.step()
  print(release.plan)
  release.edit(release.plan.replace("await wait(3600)\n", ""))
  print(await release)


asyncio.run(main())
```

```ts
import { open } from "furb";

await using life = await open("door.jsonl");
const release = life.prompt("str", "Release version 2 in one hour.");
await release.step();
console.log(release.plan);
release.edit(release.plan.replace("await wait(3600)\n", ""));
console.log(await release);
```

```rust
use std::error::Error;

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("door.jsonl").await?;
    let release = life.prompt::<String>("Release version 2 in one hour.")?;
    life.step(&release).await?;
    let plan = life.plan(&release)?;
    println!("{plan}");
    life.edit(&release, &plan.replace("await wait(3600)\n", ""))?;
    println!("{}", life.wait(release).await?);
    Ok(())
  })
}
```
