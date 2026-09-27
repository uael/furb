"""The snapshot of the catalog of models that the crate carries, made again from models.dev and from pi-ai.

Run it from the root of the repository, as `uv run python script/catalog.py`. It keeps each provider that pi-ai serves,
under the client of rig that asks it, the names of its credential, and its address, and of each model the fields that
the crate reads. models.dev lists every provider but two, whose models come from the data that pi-ai publishes. The
crate reads the models of the providers of models.dev again from the cache of furb, when a life refreshed it from the
network, and the newer of the two holds for each provider.
"""

import io
import json
import re
import tarfile
import urllib.request
from pathlib import Path

URL = "https://models.dev/api.json"
"""URL is the public catalog of models.dev."""
PI = "https://registry.npmjs.org/@earendil-works/pi-ai/-/pi-ai-0.87.0.tgz"
"""PI is the package of pi-ai, whose data holds the providers that models.dev does not list."""
SNAPSHOT = Path(__file__).parents[1] / "src" / "world" / "provider" / "catalog.json"
"""SNAPSHOT is the file that the crate carries."""
AWS = [
  "AWS_PROFILE",
  "AWS_ACCESS_KEY_ID",
  "AWS_BEARER_TOKEN_BEDROCK",
  "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
  "AWS_CONTAINER_CREDENTIALS_FULL_URI",
  "AWS_WEB_IDENTITY_TOKEN_FILE",
]
"""AWS is each name under which the credentials of Amazon stand, which the client of Amazon reads itself."""
SERVED: dict[str, dict] = {
  "anthropic": {"rig": "anthropic", "env": ["ANTHROPIC_AUTH_TOKEN"], "bearer": ["ANTHROPIC_AUTH_TOKEN"]},
  "openai": {"rig": "openai"},
  "google": {"rig": "gemini"},
  "openrouter": {"rig": "openrouter"},
  "groq": {"rig": "groq"},
  "xai": {"rig": "xai"},
  "mistral": {"rig": "mistral"},
  "deepseek": {"rig": "deepseek"},
  "togetherai": {"rig": "together"},
  "moonshotai": {"rig": "moonshot"},
  "moonshotai-cn": {"rig": "chat"},
  "zai": {"rig": "zai", "env": ["ZAI_API_KEY"]},
  "zai-coding-plan": {"rig": "chat", "env": ["ZAI_API_KEY"]},
  "zhipuai-coding-plan": {"rig": "chat", "env": ["ZAI_CODING_CN_API_KEY"]},
  "minimax": {"rig": "anthropic"},
  "minimax-cn": {"rig": "anthropic", "env": ["MINIMAX_CN_API_KEY"], "api": "https://api.minimaxi.com/anthropic"},
  "huggingface": {"rig": "huggingface"},
  "xiaomi": {"rig": "xiaomi"},
  "xiaomi-token-plan-ams": {"rig": "chat", "env": ["XIAOMI_TOKEN_PLAN_AMS_API_KEY"]},
  "xiaomi-token-plan-cn": {"rig": "chat", "env": ["XIAOMI_TOKEN_PLAN_CN_API_KEY"]},
  "xiaomi-token-plan-sgp": {"rig": "chat", "env": ["XIAOMI_TOKEN_PLAN_SGP_API_KEY"]},
  "fireworks-ai": {"rig": "chat"},
  "nvidia": {"rig": "chat"},
  "cerebras": {"rig": "chat", "api": "https://api.cerebras.ai/v1"},
  "baseten": {"rig": "chat"},
  "alibaba-token-plan": {"rig": "chat", "env": ["QWEN_TOKEN_PLAN_API_KEY"]},
  "alibaba-token-plan-cn": {"rig": "chat", "env": ["QWEN_TOKEN_PLAN_CN_API_KEY"]},
  "opencode": {"rig": "chat"},
  "opencode-go": {"rig": "chat"},
  "kimi-code-plan-cn": {"rig": "chat"},
  "kimi-code-plan-global": {"rig": "chat"},
  "meta": {"rig": "openai", "env": ["META_API_KEY"]},
  "vercel": {"rig": "chat", "api": "https://ai-gateway.vercel.sh/v1"},
  "cloudflare-workers-ai": {"rig": "chat", "env": ["CLOUDFLARE_API_KEY"]},
  "cloudflare-ai-gateway": {
    "rig": "gateway",
    "env": ["CLOUDFLARE_API_KEY", "CLOUDFLARE_API_TOKEN"],
    "api": "https://gateway.ai.cloudflare.com/v1/${CLOUDFLARE_ACCOUNT_ID}/${CLOUDFLARE_GATEWAY_ID}/compat",
  },
  "github-copilot": {"rig": "copilot", "only": ["COPILOT_GITHUB_TOKEN"]},
  "azure": {
    "rig": "azure",
    "env": ["AZURE_OPENAI_API_KEY", "AZURE_API_KEY"],
    "api": "https://${AZURE_RESOURCE_NAME}.openai.azure.com",
    "aliases": {"AZURE_RESOURCE_NAME": "AZURE_OPENAI_RESOURCE_NAME"},
  },
  "amazon-bedrock": {"rig": "bedrock", "only": AWS},
  "google-vertex": {"rig": "vertex", "only": ["GOOGLE_CLOUD_API_KEY", "GOOGLE_APPLICATION_CREDENTIALS"]},
}
"""SERVED is each provider of models.dev that the crate serves: the client of rig that asks it, the names of its
credential that pi-ai reads before those of models.dev, or `only` those, the names whose credential goes as a bearer,
its address where models.dev gives none or pi-ai gives another, and the name that pi-ai reads before a name of
models.dev in an address. `chat` is the chat completions of OpenAI at the address of the provider, and `gateway` is
that of Cloudflare, which takes the credential of Cloudflare alone."""
PIS: dict[str, dict] = {
  "ant-ling": {"rig": "chat", "env": ["ANT_LING_API_KEY"], "api": "https://api.ant-ling.com/v1"},
  "radius": {"rig": "pi", "env": ["RADIUS_API_KEY"], "api": "https://radius.pi.dev/v1"},
}
"""PIS is each provider that models.dev does not list, whose models the data of pi-ai holds; `pi` is the protocol of
pi-ai itself."""
FIELDS = ("reasoning", "reasoning_options", "limit", "cost", "modalities", "status", "last_updated", "provider")
"""FIELDS is what the crate reads of a model: `provider` names the package and the address of a model that its
provider serves apart."""


def fetched(url: str) -> bytes:
  """What an address serves. The catalog refuses the agent of urllib, so the script names itself as the crate does."""
  asked = urllib.request.Request(url, headers={"User-Agent": "furb"})  # noqa: S310
  with urllib.request.urlopen(asked, timeout=60) as got:  # noqa: S310
    return got.read()


def placed(api: str | None) -> set[str]:
  """The names of the environment that an address holds a place for."""
  return set(re.findall(r"\$\{([^}]+)\}", api or ""))


def pi(listing: dict) -> dict:
  """A model of the data of pi-ai in the words of models.dev."""
  levels = [level for level, word in (listing.get("thinkingLevelMap") or {}).items() if word is not None]
  cost = listing.get("cost", {})
  return {
    "reasoning": listing.get("reasoning", False),
    "reasoning_options": [{"type": "effort", "values": ["none" if one == "off" else one for one in levels]}]
    if levels
    else [],
    "limit": {"context": listing["contextWindow"], "output": listing["maxTokens"]},
    "cost": {
      "input": cost.get("input", 0),
      "output": cost.get("output", 0),
      "cache_read": cost.get("cacheRead", 0),
      "cache_write": cost.get("cacheWrite", 0),
    },
    "modalities": {"input": listing.get("input", ["text"]), "output": ["text"]},
  }


def main() -> None:
  """The snapshot, written again, one model on each line."""
  catalog = json.loads(fetched(URL))
  with tarfile.open(fileobj=io.BytesIO(fetched(PI))) as package:

    def data(name: str) -> dict:
      held = package.extractfile(f"package/dist/providers/data/{name}.json")
      assert held is not None
      return json.load(held)

    extras = {name: [one for api in data(name).values() for one in api.values()] for name in PIS}
  providers: dict[str, tuple[dict, dict]] = {}
  for name, served in SERVED.items():
    listed = catalog[name]
    api = served.get("api", listed.get("api"))
    models = {key: {field: one[field] for field in FIELDS if field in one} for key, one in listed["models"].items()}
    places = placed(api) | {place for one in models.values() for place in placed(one.get("provider", {}).get("api"))}
    env = served.get("only") or [one for one in served.get("env", []) + listed["env"] if one not in places]
    head = {"rig": served["rig"], "env": list(dict.fromkeys(env))}
    extra = (("bearer", served.get("bearer")), ("api", api), ("aliases", served.get("aliases")))
    head |= {key: value for key, value in extra if value}
    providers[name] = (head, models)
  for name, served in PIS.items():
    providers[name] = (served, {one["id"]: pi(one) for one in extras[name]})
  lines = ["{"]
  for at, (name, (head, models)) in enumerate(providers.items()):
    lines.append(f'{json.dumps(name)}: {json.dumps(head)[:-1]}, "models": {{')
    ordered = sorted(models.items())
    lines.extend(
      f"{json.dumps(named)}: {json.dumps(one)}" + ("," if n + 1 < len(ordered) else "")
      for n, (named, one) in enumerate(ordered)
    )
    lines.append("}}" + ("," if at + 1 < len(providers) else ""))
  lines.append("}")
  SNAPSHOT.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
  main()
