"""The snapshot of the catalog of models that the crate carries, made again from models.dev.

Run it from the root of the repository, as `uv run python script/catalog.py`. It keeps the providers that rig
serves, each under the client of rig that asks it, and of each model the fields that the crate reads. The crate reads
the models of these providers again from the cache of furb, when a life refreshed it from the network.
"""

import json
import urllib.request
from pathlib import Path

URL = "https://models.dev/api.json"
"""URL is the public catalog of models.dev."""
SNAPSHOT = Path(__file__).parents[1] / "src" / "world" / "provider" / "catalog.json"
"""SNAPSHOT is the file that the crate carries."""
RIG = {
  "anthropic": "anthropic",
  "openai": "openai",
  "google": "gemini",
  "openrouter": "openrouter",
  "groq": "groq",
  "xai": "xai",
  "mistral": "mistral",
  "deepseek": "deepseek",
  "togetherai": "together",
  "moonshotai": "moonshot",
  "zai": "zai",
  "minimax": "minimax",
  "huggingface": "huggingface",
  "xiaomi": "xiaomi",
  "fireworks-ai": "chat",
  "nvidia": "chat",
}
"""RIG is each provider the crate serves, and the client of rig that asks it: `chat` asks the chat completions of
OpenAI at the address of the provider."""
FIELDS = ("reasoning", "reasoning_options", "limit", "cost", "modalities", "status")
"""FIELDS is what the crate reads of a model."""


def main() -> None:
  """The snapshot, written again, one model on each line."""
  # The catalog refuses the agent of urllib, so the script names itself as the crate does.
  asked = urllib.request.Request(URL, headers={"User-Agent": "furb"})
  with urllib.request.urlopen(asked, timeout=60) as got:  # noqa: S310
    catalog = json.load(got)
  lines = ["{"]
  for at, (name, rig) in enumerate(RIG.items()):
    provider = catalog[name]
    head = {"rig": rig, "env": provider["env"], **({"api": provider["api"]} if provider.get("api") else {})}
    lines.append(f'{json.dumps(name)}: {json.dumps(head)[:-1]}, "models": {{')
    models = sorted(provider["models"].items())
    lines.extend(
      f"{json.dumps(named)}: {json.dumps({key: model[key] for key in FIELDS if key in model})}"
      + ("," if n + 1 < len(models) else "")
      for n, (named, model) in enumerate(models)
    )
    lines.append("}}" + ("," if at + 1 < len(RIG) else ""))
  lines.append("}")
  SNAPSHOT.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
  main()
