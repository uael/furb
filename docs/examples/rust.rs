//! furb as a framework, from Rust: a durable life, a tree of chains, a prompt to the operator, a grant and a door.
//!
//! Run it as it stands, as `cargo run --example framework`, and `cargo test` runs it too. A scripted model answers
//! in place of a real one, so it costs nothing.

use std::{
  env,
  process::Command,
  sync::{Arc, Mutex},
};

use furb::{
  Act, Ear, Engine, Fact, Object, Text,
  ear::{ear, hear, say},
  life::{Answer, Opening},
  verbs,
  world::Hosted,
};
use futures::executor::block_on;
use serde_json::Value;

const PLAN: &str = "Plan the release.";
const REVIEW: &str = "Review the plan.";
const COUNT: &str = "Count the files.";
const WAIT: &str = "Wait for a new plan.";
const WORDS: [(&str, &str); 4] = [
  (
    PLAN,
    "reviewer = chain(\"reviewer\")\nreview = await prompt(str, \"Review the plan.\", on=reviewer)\n\
     ship = await prompt(str, \"Ship it?\", to=OPERATOR)\nclose(f\"{review}, and the operator says {ship}\")",
  ),
  (REVIEW, "close(\"looks good\")"),
  (COUNT, "files = 3"),
  (WAIT, "await wait(3600)"),
];
const DOLLARS: f64 = 0.3;

type Asked = Arc<Mutex<Vec<String>>>;

fn check(holds: bool, claim: &str) {
  assert!(holds, "the example fails: {claim}");
}

fn model(asked: Asked) -> Answer {
  let hosted: Hosted = Arc::new(move |request: Value, _| {
    let said = request["messages"].to_string();
    let (task, word) = WORDS.into_iter().find(|(task, _)| said.contains(task)).unwrap_or(("", ""));
    asked.lock().expect("the list of tasks").push(task.to_owned());
    let usage =
      [Object::int(100), Object::int(10), Object::int(0), Object::int(0), Object::float(DOLLARS)];
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string(word),
      Object::tuple(usage),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  });
  Answer(hosted)
}

fn console(answer: Option<&'static str>) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let fact = hear(&mut co).await;
      let to = fact.word(3).and_then(|one| one.as_str().map(str::to_owned));
      if fact.question() && fact.kind() == "prompt" && to.as_deref() == Some("operator") {
        let saying = match answer {
          Some(text) => Fact::says("done", fact.about(), [Object::string(text)]),
          None => Fact::says("started", fact.about(), []),
        };
        say(&mut co, saying);
      }
    }
  })
}

fn life(record: &str, answer: Option<&'static str>, asked: &Asked) -> Engine {
  let opening = Opening {
    directory: env::current_dir().expect("a directory").display().to_string(),
    record: Some(record.to_owned()),
    extensions: Some(false),
    actor: Some("claude-cli:opus".to_owned()),
    answer: Some(model(Arc::clone(asked))),
    ..Opening::default()
  };
  opening.boot([("console", console(answer))]).expect("the life opens").0
}

fn until(engine: &mut Engine, on: &str, holds: impl Fn(&mut Engine) -> bool) {
  while !holds(engine) {
    let wait = verbs::Wait { seconds: Some(0.01), on: Some(on.to_owned()) };
    block_on(engine.wait(&[], wait).expect("a wait")).expect("the wait ends");
  }
}

fn facts(engine: &mut Engine, on: &str) -> Vec<Fact> {
  engine.transcript(verbs::Transcript { on: Some(on.to_owned()) }).expect("a transcript")
}

fn first(record: &str) {
  let asked = Asked::default();
  let mut engine = life(record, None, &asked);
  let root = engine.root().to_owned();
  let with =
    verbs::Prompt { message: Some(PLAN.to_owned()), on: Some(root.clone()), ..Default::default() };
  let plan = engine.prompt(Object::string("str"), with).expect("a prompt").id().to_owned();
  let asks_operator = |one: &Fact| {
    one.kind() == "prompt"
      && one.word(3).and_then(|to| to.as_str().map(str::to_owned)).as_deref() == Some("operator")
  };
  until(&mut engine, &root, |engine| facts(engine, &root).iter().any(asks_operator));
  println!("{plan}");
}

fn second(record: &str, plan: &str) {
  let asked = Asked::default();
  let mut engine = life(record, Some("yes"), &asked);
  let root = engine.root().to_owned();
  let on = |chain: &str| Some(chain.to_owned());
  engine.wake(&root).expect("a wake");
  let got = block_on(Act::<String>::of(&mut engine, plan)).expect("the plan");
  check(got == "looks good, and the operator says yes", "the life resumes");
  check(
    asked.lock().expect("the tasks").is_empty(),
    "the record answers each word of the first life",
  );
  let reviewer =
    facts(&mut engine, &root).into_iter().find(|one| one.kind() == "chain").expect("a chain");
  let label = engine
    .get(reviewer.about())
    .expect("a view")
    .and_then(|one| one.word(1).and_then(|label| label.as_str().map(str::to_owned)));
  check(label.as_deref() == Some("reviewer"), "the model opens a chain of its own");

  let budget = engine
    .chain(verbs::Chain { label: on("budget"), on: on(&root), ..Default::default() })
    .expect("a chain")
    .id()
    .to_owned();
  drop(
    engine
      .grant(verbs::Grant { usd: Some(0.5), on: on(&budget), ..Default::default() })
      .expect("a grant"),
  );
  let with =
    verbs::Prompt { message: Some(COUNT.to_owned()), on: on(&budget), ..Default::default() };
  drop(engine.prompt(Object::string("int"), with).expect("a prompt"));
  until(&mut engine, &root, |engine| {
    engine.verb("paused", vec![Object::string(&budget)], vec![]).expect("a view").py_repr()
      == "True"
  });
  check(
    *asked.lock().expect("the tasks") == [COUNT, COUNT],
    "the grant pauses the chain at its ceiling",
  );

  let door = engine
    .chain(verbs::Chain { label: on("door"), on: on(&root), ..Default::default() })
    .expect("a chain")
    .id()
    .to_owned();
  let with = verbs::Prompt { message: Some(WAIT.to_owned()), on: on(&door), ..Default::default() };
  let waiting = engine.prompt(Object::string("int"), with).expect("a prompt").id().to_owned();
  until(&mut engine, &root, |engine| facts(engine, &door).iter().any(|one| one.kind() == "wait"));
  engine
    .write(&Text::new(&waiting, "close(4)"), verbs::Write { on: on(&door) })
    .expect("a write of the door");
  let got = block_on(Act::<Object>::of(&mut engine, &waiting)).expect("the prompt");
  check(got.py_repr() == "4", "a write of the door of a prompt changes the plan of its ladder");
}

fn main() {
  if let Some(record) = env::args().nth(1) {
    first(&record);
    return;
  }
  let yard = env::temp_dir().join(format!("furb-example-{}", std::process::id()));
  std::fs::create_dir_all(&yard).expect("a yard");
  let record = yard.join("life.jsonl").display().to_string();
  let ran = Command::new(env::current_exe().expect("this program"))
    .arg(&record)
    .output()
    .expect("the first process");
  check(ran.status.success(), "the first process ends well");
  let plan = String::from_utf8_lossy(&ran.stdout).lines().last().unwrap_or_default().to_owned();
  println!("the first process ended while the operator was asked; {plan} waits in {record}");
  second(&record, &plan);
  std::fs::remove_dir_all(&yard).expect("the yard goes");
  println!("the second process resumed it from the record, and every claim holds");
}
