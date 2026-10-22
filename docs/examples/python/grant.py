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
