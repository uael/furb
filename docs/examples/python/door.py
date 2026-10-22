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
