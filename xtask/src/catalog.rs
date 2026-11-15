//! The snapshot of the catalog of models that the crate carries, made again from models.dev and
//! from the latest release of pi-ai. It keeps each provider that pi-ai serves, under the client of
//! rig that asks it, the names of its credential and its address, and of each model the fields that
//! the crate reads. models.dev lists every provider but two, whose models come from the data that
//! pi-ai publishes.

use std::{
  collections::BTreeSet,
  fs,
  io::Read,
  path::Path,
  process::{Command as Process, ExitCode},
  time::{SystemTime, UNIX_EPOCH},
};

use fancy_regex::Regex;
use flate2::read::GzDecoder;
use serde_json::{Map, Value, json};

use crate::Fault;

/// The public catalog of models.dev.
const URL: &str = "https://models.dev/api.json";
/// The registry entry of pi-ai, whose data holds the providers that models.dev does not list. Each
/// run takes its latest release and keeps the exact version in the sources, so a build of the
/// snapshot is repeatable.
const PI: &str = "https://registry.npmjs.org/@earendil-works/pi-ai";
/// The file that the crate carries, and the file that says where it came from: the version of each
/// source and the time of its fetch, written only when the snapshot or the version of pi-ai
/// changed.
const SNAPSHOT: &str = "src/world/provider/catalog.json";
const SOURCES: &str = "src/world/provider/catalog.sources.json";
/// The most that a cheap model costs, in dollars for a million tokens that it reads.
const CHEAP: f64 = 1.0;
/// Each alias of a model that the claude command line serves.
const ALIASES: [&str; 4] = ["fable", "haiku", "opus", "sonnet"];
/// What the crate reads of a model: `provider` names the package and the address of a model that
/// its provider serves apart.
const FIELDS: [&str; 8] = [
  "reasoning",
  "reasoning_options",
  "limit",
  "cost",
  "modalities",
  "status",
  "last_updated",
  "provider",
];

/// Each provider of models.dev that the crate serves: the client of rig that asks it, the names of
/// its credential that pi-ai reads before those of models.dev, or `only` those, the names whose
/// credential goes as a bearer, its address where models.dev gives none or pi-ai gives another, and
/// the name that pi-ai reads before a name of models.dev in an address. `chat` is the chat
/// completions of `OpenAI` at the address of the provider, and `gateway` is that of Cloudflare,
/// which takes the credential of Cloudflare alone.
fn served() -> Value {
  serde_json::from_str(SERVED).unwrap_or_default()
}

const SERVED: &str = r#"{
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
      "api": "https://gateway.ai.cloudflare.com/v1/${CLOUDFLARE_ACCOUNT_ID}/${CLOUDFLARE_GATEWAY_ID}/compat"
    },
    "github-copilot": {"rig": "copilot", "only": ["COPILOT_GITHUB_TOKEN"]},
    "azure": {
      "rig": "azure",
      "env": ["AZURE_OPENAI_API_KEY", "AZURE_API_KEY"],
      "api": "https://${AZURE_RESOURCE_NAME}.openai.azure.com",
      "aliases": {"AZURE_RESOURCE_NAME": "AZURE_OPENAI_RESOURCE_NAME"}
    },
    "amazon-bedrock": {"rig": "bedrock", "only": ["AWS_PROFILE", "AWS_ACCESS_KEY_ID", "AWS_BEARER_TOKEN_BEDROCK", "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", "AWS_CONTAINER_CREDENTIALS_FULL_URI", "AWS_WEB_IDENTITY_TOKEN_FILE"]},
    "google-vertex": {"rig": "vertex", "only": ["GOOGLE_CLOUD_API_KEY", "GOOGLE_APPLICATION_CREDENTIALS"]}
  }"#;

/// Each provider that models.dev does not list, whose models the data of pi-ai holds; `pi` is the
/// protocol of pi-ai itself.
fn pis() -> Value {
  json!({
    "ant-ling": {"rig": "chat", "env": ["ANT_LING_API_KEY"], "api": "https://api.ant-ling.com/v1"},
    "radius": {"rig": "pi", "env": ["RADIUS_API_KEY"], "api": "https://radius.pi.dev/v1"}
  })
}

/// A provider as the snapshot holds it: its head, then its models.
type Provider = (String, Map<String, Value>, Map<String, Value>);

pub(crate) fn refresh(root: &Path) -> Result<ExitCode, Fault> {
  let catalog: Value = serde_json::from_slice(&fetched(URL)?).unwrap_or_default();
  let latest: Value =
    serde_json::from_slice(&fetched(&format!("{PI}/latest"))?).unwrap_or_default();
  let version = latest["version"].as_str().unwrap_or_default().to_owned();
  let package = fetched(latest["dist"]["tarball"].as_str().unwrap_or_default())?;
  let pis = pis();
  let mut providers: Vec<Provider> = Vec::new();
  for (name, served) in served().as_object().into_iter().flatten() {
    providers.push(provided(name, served, &catalog[name])?);
  }
  for (name, served) in pis.as_object().into_iter().flatten() {
    let mut models = Map::new();
    for one in data(&package, name)? {
      let model = pi(&one);
      models.insert(one["id"].as_str().unwrap_or_default().to_owned(), classed(model));
    }
    providers.push((name.clone(), served.as_object().cloned().unwrap_or_default(), models));
  }
  providers.extend(families(&providers));
  let text = written(&providers);
  let read = |path: &str| fs::read_to_string(root.join(path)).unwrap_or_default();
  let held: Value = serde_json::from_str(&read(SOURCES)).unwrap_or_default();
  let names: Vec<&String> = pis.as_object().map(|one| one.keys().collect()).unwrap_or_default();
  let same =
    held["pi-ai"]["version"] == version.as_str() && held["pi-ai"]["providers"] == json!(names);
  if text == read(SNAPSHOT) && same {
    return Ok(ExitCode::SUCCESS);
  }
  let stamp = utc(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs()));
  let sources = json!({
    "models.dev": {"url": URL, "fetched": stamp},
    "pi-ai": {"version": version, "providers": names, "fetched": stamp}
  });
  for (path, text) in
    [(SNAPSHOT, text), (SOURCES, serde_json::to_string_pretty(&sources).unwrap_or_default() + "\n")]
  {
    let path = root.join(path);
    fs::write(&path, text).map_err(|source| Fault::Read { path, source })?;
  }
  Ok(ExitCode::SUCCESS)
}

/// A provider of models.dev as the crate serves it.
fn provided(name: &str, served: &Value, listed: &Value) -> Result<Provider, Fault> {
  let api = served.get("api").or_else(|| listed.get("api")).and_then(Value::as_str);
  let mut models = Map::new();
  for (key, one) in listed["models"].as_object().into_iter().flatten() {
    let kept: Map<String, Value> = FIELDS
      .iter()
      .filter_map(|field| one.get(*field).map(|value| ((*field).to_owned(), value.clone())))
      .collect();
    models.insert(key.clone(), classed(Value::Object(kept)));
  }
  let mut places = placed(api)?;
  for one in models.values() {
    places.extend(placed(one["provider"]["api"].as_str())?);
  }
  let names = |key: &str, value: &Value| -> Vec<String> {
    value[key]
      .as_array()
      .into_iter()
      .flatten()
      .filter_map(Value::as_str)
      .map(str::to_owned)
      .collect()
  };
  let mut env = names("only", served);
  if env.is_empty() {
    env = [names("env", served), names("env", listed)].concat();
    env.retain(|one| !places.contains(one));
  }
  let mut seen = BTreeSet::new();
  env.retain(|one| seen.insert(one.clone()));
  let mut head = Map::new();
  head.insert("rig".to_owned(), served["rig"].clone());
  head.insert("env".to_owned(), json!(env));
  let extra = [
    ("bearer", served.get("bearer").cloned()),
    ("api", api.map(Value::from)),
    ("aliases", served.get("aliases").cloned()),
  ];
  for (key, value) in extra {
    if let Some(value) = value.filter(truthy) {
      head.insert(key.to_owned(), value);
    }
  }
  Ok((name.to_owned(), head, models))
}

/// Whether python reads a value as true: no null, no false, no empty text, list or map.
fn truthy(value: &Value) -> bool {
  match value {
    Value::Null | Value::Bool(false) => false,
    Value::String(one) => !one.is_empty(),
    Value::Array(one) => !one.is_empty(),
    Value::Object(one) => !one.is_empty(),
    _ => true,
  }
}

/// The names of the environment that an address holds a place for.
fn placed(api: Option<&str>) -> Result<BTreeSet<String>, Fault> {
  let place = Regex::new(r"\$\{([^}]+)\}")?;
  let mut found = BTreeSet::new();
  for one in place.captures_iter(api.unwrap_or_default()) {
    found.insert(one?[1].to_owned());
  }
  Ok(found)
}

/// A model of the data of pi-ai in the words of models.dev.
fn pi(listing: &Value) -> Value {
  let levels: Vec<&str> = listing["thinkingLevelMap"]
    .as_object()
    .into_iter()
    .flatten()
    .filter(|(_, word)| !word.is_null())
    .map(|(level, _)| if level == "off" { "none" } else { level.as_str() })
    .collect();
  let options =
    if levels.is_empty() { json!([]) } else { json!([{"type": "effort", "values": levels}]) };
  let cost = |key: &str| listing["cost"].get(key).cloned().unwrap_or(json!(0));
  json!({
    "reasoning": listing.get("reasoning").cloned().unwrap_or(json!(false)),
    "reasoning_options": options,
    "limit": {"context": listing["contextWindow"], "output": listing["maxTokens"]},
    "cost": {"input": cost("input"), "output": cost("output"), "cache_read": cost("cacheRead"), "cache_write": cost("cacheWrite")},
    "modalities": {"input": listing.get("input").cloned().unwrap_or(json!(["text"])), "output": ["text"]}
  })
}

/// A model with its class, from the price of a million tokens that it reads: cheap at `CHEAP` or
/// less, and dear above it or with no price.
fn classed(mut model: Value) -> Value {
  let cheap = model["cost"]["input"].as_f64().is_some_and(|price| price <= CHEAP);
  if let Some(fields) = model.as_object_mut() {
    fields.insert("class".to_owned(), json!(if cheap { "cheap" } else { "dear" }));
  }
  model
}

/// The families that a program of its own serves, with no price, and no client: a model of `Codex`
/// takes the class of the model of `OpenAI` with its name, and an alias of the claude command line,
/// as haiku, the class of the newest model of Anthropic with that alias in its name.
fn families(providers: &[Provider]) -> Vec<Provider> {
  let models = |source: &str| {
    providers
      .iter()
      .find(|(name, ..)| name == source)
      .map(|(_, _, models)| models.clone())
      .unwrap_or_default()
  };
  let class = |one: &Value| json!({"class": one["class"]});
  let codex = models("openai").iter().map(|(name, one)| (name.clone(), class(one))).collect();
  let anthropic = models("anthropic");
  let mut newest = Map::new();
  for alias in ALIASES {
    let mut best: Option<&Value> = None;
    for (name, one) in &anthropic {
      let later = best.is_none_or(|held| {
        one["last_updated"].as_str().unwrap_or("") > held["last_updated"].as_str().unwrap_or("")
      });
      if name.starts_with(&format!("claude-{alias}-")) && later {
        best = Some(one);
      }
    }
    if let Some(one) = best {
      newest.insert(alias.to_owned(), class(one));
    }
  }
  vec![("claude-cli".to_owned(), Map::new(), newest), ("codex".to_owned(), Map::new(), codex)]
}

/// The text of the snapshot: each provider with its head, then one model on each line, as python
/// writes its json.
fn written(providers: &[Provider]) -> String {
  let mut lines = vec!["{".to_owned()];
  for (at, (name, head, models)) in providers.iter().enumerate() {
    let head = python(&Value::Object(head.clone()));
    let opened =
      if head == "{}" { "{".to_owned() } else { format!("{}, ", &head[..head.len() - 1]) };
    lines.push(format!("{}: {opened}\"models\": {{", python(&json!(name))));
    let mut ordered: Vec<(&String, &Value)> = models.iter().collect();
    ordered.sort_by(|one, other| one.0.cmp(other.0));
    for (n, (named, one)) in ordered.iter().enumerate() {
      let comma = if n + 1 < ordered.len() { "," } else { "" };
      lines.push(format!("{}: {}{comma}", python(&json!(named)), python(one)));
    }
    lines.push(format!("}}}}{}", if at + 1 < providers.len() { "," } else { "" }));
  }
  lines.push("}".to_owned());
  lines.join("\n") + "\n"
}

/// A value as `json.dumps` of python writes it: a space after each comma and colon, each letter
/// past ASCII escaped, and each float as python reads it back.
pub(crate) fn python(value: &Value) -> String {
  match value {
    Value::Number(number) if number.is_f64() => float(number.as_f64().unwrap_or_default()),
    Value::String(text) => {
      let mut out = String::from("\"");
      for one in text.chars() {
        match one {
          '"' => out.push_str("\\\""),
          '\\' => out.push_str("\\\\"),
          '\n' => out.push_str("\\n"),
          '\r' => out.push_str("\\r"),
          '\t' => out.push_str("\\t"),
          '\u{8}' => out.push_str("\\b"),
          '\u{c}' => out.push_str("\\f"),
          one if one.is_ascii() && !one.is_ascii_control() => out.push(one),
          one => {
            let mut units = [0_u16; 2];
            for unit in one.encode_utf16(&mut units) {
              out.push_str("\\u");
              out.extend(
                [12, 8, 4, 0].map(|shift| {
                  char::from_digit(u32::from(*unit >> shift & 0xf), 16).unwrap_or('0')
                }),
              );
            }
          }
        }
      }
      out + "\""
    }
    Value::Array(items) => format!("[{}]", items.iter().map(python).collect::<Vec<_>>().join(", ")),
    Value::Object(fields) => {
      let fields: Vec<String> = fields
        .iter()
        .map(|(key, one)| format!("{}: {}", python(&json!(key)), python(one)))
        .collect();
      format!("{{{}}}", fields.join(", "))
    }
    other => other.to_string(),
  }
}

/// A float as python writes it: its shortest digits, in a decimal form from 1e-4 up to 1e16, and in
/// a form with an exponent of two digits at least out of that span.
fn float(value: f64) -> String {
  let scientific = format!("{value:e}");
  let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
  let exponent: i32 = exponent.parse().unwrap_or(0);
  let (sign, mantissa) = mantissa.strip_prefix('-').map_or(("", mantissa), |rest| ("-", rest));
  let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
  if !(-4..16).contains(&exponent) {
    let mantissa =
      if digits.len() > 1 { format!("{}.{}", &digits[..1], &digits[1..]) } else { digits };
    let mark = if exponent < 0 { '-' } else { '+' };
    return format!("{sign}{mantissa}e{mark}{:02}", exponent.unsigned_abs());
  }
  let point = usize::try_from(exponent + 1).unwrap_or(0);
  let text = if exponent < 0 {
    format!("0.{}{digits}", "0".repeat(usize::try_from(-exponent - 1).unwrap_or(0)))
  } else if digits.len() <= point {
    format!("{digits}{}.0", "0".repeat(point - digits.len()))
  } else {
    format!("{}.{}", &digits[..point], &digits[point..])
  };
  format!("{sign}{text}")
}

/// The models of a provider that the data of pi-ai holds in its package.
fn data(package: &[u8], name: &str) -> Result<Vec<Value>, Fault> {
  let wanted = format!("package/dist/providers/data/{name}.json");
  let mut archive = tar::Archive::new(GzDecoder::new(package));
  let failed = |source| Fault::Read { path: wanted.clone().into(), source };
  for entry in archive.entries().map_err(failed)? {
    let mut entry = entry.map_err(failed)?;
    if entry.path().map_err(failed)?.to_string_lossy() == wanted {
      let mut text = String::new();
      entry.read_to_string(&mut text).map_err(failed)?;
      let apis: Value = serde_json::from_str(&text).unwrap_or_default();
      let models = apis
        .as_object()
        .into_iter()
        .flatten()
        .flat_map(|(_, api)| api.as_object().into_iter().flatten().map(|(_, one)| one.clone()));
      return Ok(models.collect());
    }
  }
  Ok(Vec::new())
}

/// What an address serves. The catalog refuses an agent that it does not know, so the task names
/// itself as the crate does.
fn fetched(url: &str) -> Result<Vec<u8>, Fault> {
  let out = Process::new("curl")
    .args(["-fsSL", "--max-time", "60", "-A", "furb", url])
    .output()
    .map_err(|source| Fault::Spawn { program: "curl".to_owned(), source })?;
  if !out.status.success() {
    return Err(Fault::Fetch {
      url: url.to_owned(),
      stderr: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
    });
  }
  Ok(out.stdout)
}

/// A time of the epoch as `2026-11-15T12:00:00Z`.
fn utc(seconds: u64) -> String {
  let (days, rest) = (i64::try_from(seconds / 86_400).unwrap_or(0), seconds % 86_400);
  // The date of the calendar of a count of days from the epoch, as Howard Hinnant gives it.
  let shifted = days + 719_468;
  let era = shifted.div_euclid(146_097);
  let of_era = shifted - era * 146_097;
  let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
  let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  let point = (5 * of_year + 2) / 153;
  let day = of_year - (153 * point + 2) / 5 + 1;
  let month = if point < 10 { point + 3 } else { point - 9 };
  let year = year_of_era + era * 400 + i64::from(month <= 2);
  format!(
    "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
    rest / 3600,
    rest % 3600 / 60,
    rest % 60
  )
}

#[cfg(test)]
#[path = "catalog.test.rs"]
mod test;
