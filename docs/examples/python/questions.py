import asyncio

import furb


async def main() -> None:
  life = await furb.open("deploy.jsonl")
  deploy = life.prompt(str, "Deploy version 2. Ask me before you deploy.")
  async for question in life.questions(until=deploy):
    question.answer(input(question.message + " "))
  print(await deploy)


asyncio.run(main())
