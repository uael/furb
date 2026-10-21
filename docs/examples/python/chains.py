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
