//! The bench of a loop: furb that triages support tickets, against the claude command line on the
//! same model and the same server.
//!
//! `make` writes the tickets from a seed, with a known label for each. Three arms run on a life of
//! furb: `furb` runs one word that asks the model one prompt per ticket, on a chain of its own;
//! `furb-own` gives the life the task of the arm `claude` as one prompt of the operator, so its
//! model writes its own words; `furb-batch` runs one word that asks for `BATCH` labels at a time.
//! The arm `claude` gives every ticket to the claude command line in one session, through a
//! wrapper that points it at the same server. Each arm reads `BRIEF`. The server measures each arm:
//! the bench reads the totals of its `/metrics` before and after the arm, so run each arm alone on
//! the server. `score` compares each label with the known one.

use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode, Stdio},
  time::{Duration, Instant},
};

use clap::{Args, ValueEnum};
use serde_json::{Map, Value, json};
use wait_timeout::ChildExt;

use crate::{Fault, deepswe, field};

/// Each label, and what it means, which each arm reads.
const LABELS: [(&str, &str); 8] = [
  (
    "billing",
    "a question or a problem about a charge, an invoice, a payment method or a price, with no request for money back",
  ),
  ("refund", "the customer asks for money back"),
  ("shipping", "where an order is, a late, lost or damaged delivery, or the address of a delivery"),
  (
    "account",
    "a login, a password, a code of two-factor sign-in, a locked account or the email of the account",
  ),
  ("bug", "the app or the site shows an error, stops, or does not work as it should"),
  ("cancel", "the customer wants to end a subscription or to close the account"),
  (
    "product",
    "a question about what a product does, its features, its sizes or what it works with",
  ),
  ("feedback", "praise, a suggestion, or a complaint with no request"),
];
/// The number of tickets that one prompt of the arm furb-batch asks a label for.
const BATCH: usize = 50;
/// The word of the arm furb: it loops on the tickets and asks one prompt of the shape str for each.
const FURB: &str = r#"import json
tickets = json.loads(read("tickets.json", HIDDEN).content)
brief = BRIEF
labels = {}
for one in tickets:
  got = await prompt(str, brief + "\n\nThe ticket:\n" + one["text"] + "\n\nClose with its label alone.", on=chain(one["id"]))
  labels[one["id"]] = str(got)
  write(Text("labels.json", json.dumps(labels, indent=1) + "\n"))
len(labels)"#;
/// The word of the arm furb-batch: one prompt of the shape dict for each batch, all on one chain.
const BATCHED: &str = r#"import json
tickets = json.loads(read("tickets.json", HIDDEN).content)
brief = BRIEF
labels = {}
here = chain("batches")
for start in range(0, len(tickets), SIZE):
  listed = "\n".join(one["id"] + ": " + one["text"] for one in tickets[start:start + SIZE])
  listed = "\n\nThe tickets, each after its id:\n" + listed + "\n\nClose with a dict that maps each id to its label."
  got = await prompt(dict, brief + listed, on=here)
  assert isinstance(got, dict)
  labels.update({str(k): str(v) for k, v in got.items()})
  write(Text("labels.json", json.dumps(labels, indent=1) + "\n"))
len(labels)"#;
const OPENERS: [&str; 8] = [
  "Hi,",
  "Hello there.",
  "Good morning,",
  "Hey support,",
  "Hi team,",
  "",
  "Dear support,",
  "Hello,",
];
const CLOSERS: [&str; 7] = [
  "Thanks.",
  "Thank you!",
  "Regards, {name}",
  "Cheers, {name}",
  "",
  "Thanks in advance.",
  "{name}",
];
const ASIDES: [&str; 8] = [
  "I have been a customer for {years} years.",
  "I am writing from my phone, sorry for typos.",
  "This is my second message about it.",
  "My sister recommended your shop.",
  "I need this sorted before the weekend.",
  "",
  "",
  "",
];
/// The tickets of each label, in the order of `LABELS`, which the seed fills.
const TEMPLATES: [[&str; 7]; 8] = [
  [
    "My invoice for order {order} shows {amount} but the price on the page was lower. Which one is right?",
    "I see two charges of {amount} on my card for order {order}. Why was I charged twice?",
    "How do I change the card that my monthly plan uses? The old one expired.",
    "Can you send me an invoice with my company name on it for order {order}?",
    "Why does my bill include a fee of {amount} that I never saw at checkout?",
    "My payment for the {product} failed three times, but the bank says the card is fine.",
    "Do you take PayPal, or only cards? I want to pay for the {product} that way.",
  ],
  [
    "I returned the {product} two weeks ago and I still have not got my {amount} back. Please refund me.",
    "The {product} broke after a day. I do not want a new one, I want my money back.",
    "Please give me back the {amount} for order {order}, I changed my mind and never opened it.",
    "You charged me twice for order {order}. Please return the extra {amount} to my card.",
    "The {product} I got is not what the photos showed. I want a full refund.",
    "When will the refund for order {order} reach my account? You said 5 days and it is 12 now.",
    "I cancelled order {order} before it shipped, so please refund the {amount}.",
  ],
  [
    "Where is my order {order}? The tracking has not moved since {date}.",
    "The box of order {order} arrived crushed and the {product} inside is scratched. What now?",
    "Can I change the delivery address of order {order}? I moved last week.",
    "The courier says order {order} was delivered on {date}, but nothing came.",
    "Do you ship the {product} to Canada, and how long does it take?",
    "My order {order} was due on {date} and it is still not here.",
    "Only half of order {order} arrived. Where is the {product}?",
  ],
  [
    "I cannot log in. The site says my password is wrong but I just reset it.",
    "My account is locked after too many tries. Please unlock it.",
    "The code of the two-factor sign-in never reaches my phone.",
    "How do I change the email of my account? I lost access to the old one.",
    "I never got the email to reset my password, I checked the spam folder too.",
    "Someone else signed in to my account from another country. Please help me secure it.",
    "I made two accounts by mistake. Can you merge them into one?",
  ],
  [
    "The app closes each time I open my cart.",
    "When I click Pay, the page shows 'Error 500' and nothing happens.",
    "The filter for size does not work: it shows every size anyway.",
    "Your site shows the prices in dollars though I chose euros, and the button to switch does nothing.",
    "The search box gives no result for '{product}', but the {product} is on the home page.",
    "The app on my phone shows a blank screen after the last update.",
    "I add the {product} to my cart and it is gone when I reload the page.",
  ],
  [
    "Please cancel my monthly plan. I do not need it anymore.",
    "I want to close my account and delete my data.",
    "How do I stop the subscription before the next charge on {date}?",
    "Cancel my membership please, I am moving abroad.",
    "I want to end my yearly plan now. Do not renew it.",
    "Please delete my account. I will not use your shop again.",
    "Stop my subscription box, starting this month.",
  ],
  [
    "Does the {product} work with an iPhone?",
    "What is the difference between the {product} and the {product2}?",
    "Is the {product} waterproof, or only water resistant?",
    "Which size of the {product} should I take if I am 180 cm tall?",
    "How long does the battery of the {product} last?",
    "Does the {product} come with a charger in the box?",
    "Can the {product} be used outdoors in winter?",
  ],
  [
    "Just wanted to say the {product} is great and the delivery was fast. Well done!",
    "Your new site is much harder to use than the old one. I liked the old menu better.",
    "It would be nice if you added a dark mode to the app.",
    "The person on the chat yesterday was very kind and patient. Please thank her.",
    "I think the packaging uses too much plastic. You could do better.",
    "Suggestion: let us save more than one address of delivery.",
    "Five stars for the {product}, it is the best I have owned.",
  ],
];
const PRODUCTS: [&str; 7] = [
  "headphones",
  "rain jacket",
  "coffee grinder",
  "desk lamp",
  "running shoes",
  "backpack",
  "smart watch",
];
const NAMES: [&str; 10] =
  ["Ana", "Bruno", "Chloe", "Dev", "Emma", "Farid", "Grace", "Hugo", "Ines", "Jonas"];
const MONTHS: [&str; 3] = ["March", "June", "October"];
/// What the server counts of each arm.
const COUNTED: [&str; 6] =
  ["requests", "prompt_tokens", "reused", "output_tokens", "prompt_ms", "decode_ms"];

/// A step of the bench.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Step {
  Make,
  Furb,
  FurbOwn,
  FurbBatch,
  Claude,
  Score,
}

/// One step of the bench of a loop, on its folder.
#[derive(Args)]
pub(crate) struct Loop {
  /// The step: make the tickets, run an arm, or score the arms.
  step: Step,
  /// The folder of the bench, which holds the tickets, the known labels and a folder for each arm.
  folder: PathBuf,
  /// The seed of the tickets.
  #[arg(long, default_value_t = 7)]
  seed: u64,
  /// The number of tickets.
  #[arg(long, default_value_t = 200)]
  count: usize,
  /// The actor of the arms of furb, as local:<model>/<effort>.
  #[arg(long)]
  model: Option<String>,
  /// The seconds that an arm may take.
  #[arg(long, default_value_t = 14_400)]
  cap: u64,
  /// The wrapper of the claude command line that points it at the server.
  #[arg(long)]
  claude: Option<PathBuf>,
}

/// The instructions that each arm reads.
fn brief() -> String {
  let listed: Vec<String> =
    LABELS.iter().map(|(name, meaning)| format!("- {name}: {meaning}")).collect();
  format!(
    "You triage the tickets of the support of a web shop. Give each ticket one label of this list, the one that names what the customer wants most:\n{}\nJudge each ticket yourself, as a person of the support reads it. A label is the name alone, in lower case.",
    listed.join("\n")
  )
}

/// What the arms claude and furb-own ask: the instructions, and the files of the tickets and of the
/// labels.
fn whole() -> String {
  brief()
    + "\n\nThe tickets are in tickets.json, a list of objects with an id and a text. Write labels.json: one JSON object that maps the id of each ticket to its label."
}

impl Loop {
  pub(crate) fn run(self, root: &Path) -> Result<ExitCode, Fault> {
    let folder =
      env::current_dir().map(|here| here.join(&self.folder)).unwrap_or(self.folder.clone());
    let cap = Duration::from_secs(self.cap);
    let named =
      self.step.to_possible_value().map(|one| one.get_name().to_owned()).unwrap_or_default();
    match self.step {
      Step::Make => made(&folder, self.seed, self.count)?,
      Step::Score => println!("{}", pretty(&Value::Object(scored(&folder)?))),
      Step::Claude => {
        let claude = self.claude.unwrap_or_else(|| {
          PathBuf::from(env::var_os("HOME").unwrap_or_default())
            .join(".claude")
            .join("strata-claude")
        });
        let mut command = Process::new(claude);
        command.args([
          "-p",
          &whole(),
          "--permission-mode",
          "bypassPermissions",
          "--output-format",
          "json",
        ]);
        arm(&folder, &named, command, cap)?;
      }
      Step::Furb | Step::FurbOwn | Step::FurbBatch => {
        let Some(model) = self.model else {
          return Err(Fault::Unproven(format!("the arm {named} needs --model")));
        };
        let mut command = Process::new(deepswe::furb(root)?);
        let record = folder.join(&named).join("record.jsonl");
        if self.step == Step::FurbOwn {
          command.args(["prompt", &whole(), "--to", &model, "--shape", "str"]);
        } else {
          let brief = format!("{:?}", brief());
          let word = if self.step == Step::Furb {
            FURB.replace("BRIEF", &brief)
          } else {
            BATCHED.replace("BRIEF", &brief).replace("SIZE", &BATCH.to_string())
          };
          command.args(["run", &word]);
        }
        command.arg("--model").arg(&model).arg("--record").arg(record);
        arm(&folder, &named, command, cap)?;
      }
    }
    Ok(ExitCode::SUCCESS)
  }
}

/// The tickets of a seed, in its folder, and the known labels apart: each label in turn, each
/// filled from the seed, then shuffled.
fn made(folder: &Path, seed: u64, count: usize) -> Result<(), Fault> {
  let mut rng = fastrand::Rng::with_seed(seed);
  let mut rows = Vec::new();
  for n in 0..count {
    let at = n % LABELS.len();
    let product = rng.usize(..PRODUCTS.len());
    let other = (product + 1 + rng.usize(..PRODUCTS.len() - 1)) % PRODUCTS.len();
    let words = [
      ("{order}", format!("#{}", rng.u32(10_000..=99_999))),
      ("{amount}", format!("{}.{:02} EUR", rng.u32(5..=400), rng.u32(0..=99))),
      ("{date}", format!("{} {}", rng.choice(MONTHS).unwrap_or_default(), rng.u32(1..=28))),
      ("{product2}", PRODUCTS.get(other).copied().unwrap_or_default().to_owned()),
      ("{product}", PRODUCTS.get(product).copied().unwrap_or_default().to_owned()),
      ("{name}", rng.choice(NAMES).unwrap_or_default().to_owned()),
      ("{years}", rng.u32(2..=9).to_string()),
    ];
    let parts = [
      rng.choice(OPENERS).unwrap_or_default(),
      rng.choice(ASIDES).unwrap_or_default(),
      TEMPLATES.get(at).and_then(|one| rng.choice(one.iter().copied())).unwrap_or_default(),
      rng.choice(CLOSERS).unwrap_or_default(),
    ];
    let filled: Vec<String> = parts
      .iter()
      .filter(|part| !part.is_empty())
      .map(|part| {
        words.iter().fold((*part).to_owned(), |text, (key, value)| text.replace(key, value))
      })
      .collect();
    rows.push((filled.join(" "), LABELS.get(at).map_or("", |one| one.0)));
  }
  rng.shuffle(&mut rows);
  let (mut tickets, mut known) = (Vec::new(), Map::new());
  for (n, (text, label)) in rows.into_iter().enumerate() {
    let id = format!("t{:03}", n + 1);
    known.insert(id.clone(), json!(label));
    tickets.push(json!({"id": id, "text": text}));
  }
  kept(&folder.join("tickets.json"), &Value::Array(tickets))?;
  kept(&folder.join("answers.json"), &Value::Object(known))
}

/// One arm, run in a copy of the tickets of its own for `cap` at most, and what the server counted
/// for it beside its labels.
fn arm(folder: &Path, name: &str, mut command: Process, cap: Duration) -> Result<(), Fault> {
  let home = folder.join(name);
  let failed = |path: &Path| {
    let path = path.to_owned();
    move |source| Fault::Read { path, source }
  };
  fs::create_dir_all(&home).map_err(failed(&home))?;
  fs::copy(folder.join("tickets.json"), home.join("tickets.json")).map_err(failed(&home))?;
  let out = fs::File::create(home.join("stdout.txt")).map_err(failed(&home))?;
  let err = fs::File::create(home.join("stderr.txt")).map_err(failed(&home))?;
  let (before, start) = (totals()?, Instant::now());
  let mut child = command
    .current_dir(&home)
    .stdin(Stdio::null())
    .stdout(out)
    .stderr(err)
    .spawn()
    .map_err(|source| Fault::Spawn { program: name.to_owned(), source })?;
  let ended = child.wait_timeout(cap).map_err(failed(&home))?;
  if ended.is_none() {
    child.kill().map_err(failed(&home))?;
    child.wait().map_err(failed(&home))?;
  }
  let code = ended.and_then(|status| status.code());
  let (after, wall) = (totals()?, start.elapsed().as_secs_f64());
  let server: Map<String, Value> = COUNTED
    .iter()
    .map(|key| {
      (
        (*key).to_owned(),
        json!(field(&after, key).as_i64().unwrap_or(0) - field(&before, key).as_i64().unwrap_or(0)),
      )
    })
    .collect();
  let run = json!({"code": code, "wall_s": (wall * 10.0).round() / 10.0, "server": server});
  kept(&home.join("run.json"), &run)?;
  println!("{run}");
  Ok(())
}

/// The totals of the server since it started, from its `/metrics` at the address and with the key
/// of `FURB_LOCAL_URL` and `FURB_LOCAL_KEY`.
fn totals() -> Result<Value, Fault> {
  let url = env::var("FURB_LOCAL_URL")
    .map_err(|no| Fault::Unproven(format!("FURB_LOCAL_URL names no server: {no}")))?;
  let base = url.trim_end_matches('/');
  let url = format!("{}/metrics", base.strip_suffix("/v1").unwrap_or(base));
  let key = env::var("FURB_LOCAL_KEY").unwrap_or_default();
  let out = Process::new("curl")
    .args(["-fsS", "--max-time", "30", "-H", &format!("Authorization: Bearer {key}"), &url])
    .output()
    .map_err(|source| Fault::Spawn { program: "curl".to_owned(), source })?;
  if !out.status.success() {
    return Err(Fault::Fetch {
      url,
      stderr: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
    });
  }
  let read: Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
  Ok(field(&read, "totals").clone())
}

/// The share of right labels of each arm, and its tokens and seconds per ticket as the server
/// counted them, also kept in score.json.
fn scored(folder: &Path) -> Result<Map<String, Value>, Fault> {
  let read = |path: &Path| -> Value {
    fs::read_to_string(path)
      .ok()
      .and_then(|text| serde_json::from_str(&text).ok())
      .unwrap_or_default()
  };
  let known: BTreeMap<String, String> = read(&folder.join("answers.json"))
    .as_object()
    .into_iter()
    .flatten()
    .map(|(id, label)| (id.clone(), label.as_str().unwrap_or_default().to_owned()))
    .collect();
  let count = u32::try_from(known.len().max(1)).unwrap_or(1);
  let per = |value: f64, places: f64| (value / f64::from(count) * places).round() / places;
  let mut out = Map::new();
  for step in [Step::Furb, Step::FurbOwn, Step::FurbBatch, Step::Claude] {
    let name = step.to_possible_value().map(|one| one.get_name().to_owned()).unwrap_or_default();
    let home = folder.join(&name);
    if !home.join("run.json").is_file() {
      continue;
    }
    let run = read(&home.join("run.json"));
    let given = read(&home.join("labels.json"));
    let clean: BTreeMap<&str, String> = given
      .as_object()
      .into_iter()
      .flatten()
      .map(|(id, label)| {
        let label = label.as_str().map_or_else(|| label.to_string(), str::to_owned);
        (id.as_str(), label.trim().trim_matches(|one| ".'\"`".contains(one)).to_lowercase())
      })
      .collect();
    let right = known.iter().filter(|(id, label)| clean.get(id.as_str()) == Some(*label)).count();
    let labeled = known.keys().filter(|id| clean.contains_key(id.as_str())).count();
    let server = |key: &str| field(field(&run, "server"), key).as_f64().unwrap_or(0.0);
    out.insert(
      name,
      json!({
        "right": right,
        "labeled": labeled,
        "share": per(f64::from(u32::try_from(right).unwrap_or(0)), 1000.0),
        "requests": field(field(&run, "server"), "requests"),
        "prompt_tokens_per_ticket": per(server("prompt_tokens"), 1.0),
        "reused_per_ticket": per(server("reused"), 1.0),
        "output_tokens_per_ticket": per(server("output_tokens"), 1.0),
        "server_s_per_ticket": per((server("prompt_ms") + server("decode_ms")) / 1000.0, 100.0),
        "wall_s_per_ticket": per(field(&run, "wall_s").as_f64().unwrap_or(0.0), 100.0),
        "code": field(&run, "code"),
      }),
    );
  }
  kept(&folder.join("score.json"), &Value::Object(out.clone()))?;
  Ok(out)
}

fn pretty(value: &Value) -> String {
  serde_json::to_string_pretty(value).unwrap_or_default()
}

fn kept(path: &Path, value: &Value) -> Result<(), Fault> {
  if let Some(folder) = path.parent() {
    fs::create_dir_all(folder).map_err(|source| Fault::Read { path: folder.to_owned(), source })?;
  }
  fs::write(path, pretty(value) + "\n")
    .map_err(|source| Fault::Read { path: path.to_owned(), source })
}
