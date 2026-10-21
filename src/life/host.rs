//! A life as a program holds it: open it on its record, prompt it, open chains, answer its questions.
//!
//! [`Life::open`] opens a life on a record, on the ears of the crate and on a console of its own, which takes each
//! prompt to the operator as a [`Question`]. A record that holds work goes on: the pending work of an earlier life
//! resumes at once. A program that runs again on its record makes its prompts, chains and grants in the same order,
//! so each finds the act of the record that it made before, done or pending, as a word finds its acts again; a call
//! that does not agree with the record at its place makes a new act, and the act of the record goes on as pending
//! work.

use std::{
  collections::VecDeque,
  future::{IntoFuture, Ready, poll_fn, ready},
  marker::PhantomData,
  sync::{Arc, Mutex},
  task::Poll,
};

use super::Opening;
use crate::{
  Act, Engine, Fact, Fault, Object,
  ear::{ear, hear, say},
  engine::Plain,
  value::entry,
  verbs, world,
};

/// The kinds of the acts that a program makes, which a later run of it finds again in the record.
const MADE: [&str; 4] = ["chain", "prompt", "grant", "rung"];

/// A shape that a prompt wants, by the name that the prompt carries.
pub trait Shape: Plain + Unpin {
  /// The name of the shape in python.
  const NAME: &'static str;
}

impl Shape for String {
  const NAME: &'static str = "str";
}

impl Shape for i64 {
  const NAME: &'static str = "int";
}

impl Shape for f64 {
  const NAME: &'static str = "float";
}

impl Shape for Object {
  const NAME: &'static str = "object";
}

/// One chain of a life, by its name: a conversation with a model of its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chain(pub String);

/// One prompt of a chain, which a program waits for with [`Life::wait`]: the name of the prompt, and the door of
/// its ladder, whose plan is the words of the models that answer it.
#[derive(Debug)]
pub struct Thread<T> {
  /// The name of the prompt.
  pub id: String,
  /// The chain of the prompt.
  pub chain: Chain,
  shape: PhantomData<T>,
}

impl<T> Clone for Thread<T> {
  fn clone(&self) -> Self {
    Thread { id: self.id.clone(), chain: self.chain.clone(), shape: PhantomData }
  }
}

/// One prompt to the operator, which the program answers with [`Question::answer`].
#[derive(Debug, Clone)]
pub struct Question {
  /// The name of the prompt.
  pub id: String,
  /// The chain of the prompt.
  pub chain: Chain,
  /// The shape of the answer.
  pub shape: String,
  /// What the model asks.
  pub message: String,
}

impl Question {
  /// Answer the question with a line that the console reads in its shape.
  pub fn answer(&self, life: &mut Life, line: &str) -> Result<(), Fault> {
    let value = world::answered(&self.shape, line)?;
    life.engine.close(value, verbs::Close { id: Some(self.id.clone()) })
  }
}

/// One act of a chain, with named fields.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeAct {
  /// The kind of the act, as prompt.
  pub kind: String,
  /// The name of the act, as prompt1.
  pub id: String,
  /// Who made the act.
  pub by: String,
  /// The chain of the act.
  pub on: String,
  /// Whether the act has what it came to.
  pub done: bool,
}

/// What [`Life::open`] opens a life with, which it opens when it is awaited.
pub struct Open(Opening);

impl Open {
  /// The model that a prompt goes to when it names none, as the catalog names it.
  pub fn model(mut self, model: impl Into<String>) -> Self {
    self.0.actor = Some(model.into());
    self
  }

  /// The models that the life offers beside that one.
  pub fn roster<S: Into<String>>(mut self, models: impl IntoIterator<Item = S>) -> Self {
    self.0.roster = Some(models.into_iter().map(Into::into).collect());
    self
  }

  /// The directory that the life stands on; the directory of the process when unsaid.
  pub fn directory(mut self, directory: impl Into<String>) -> Self {
    self.0.directory = directory.into();
    self
  }

  /// The claude command line to run, in place of the one that `FURB_CLAUDE_BIN` names or this machine holds.
  pub fn claude(mut self, claude: impl Into<String>) -> Self {
    self.0.claude = Some(claude.into());
    self
  }

  /// Whether the life enables the extensions that the configs turn on; true when unsaid.
  pub fn extensions(mut self, enables: bool) -> Self {
    self.0.extensions = Some(enables);
    self
  }

  fn opened(self) -> Result<Life, Fault> {
    let mut opening = self.0;
    if opening.directory.is_empty() {
      let here = std::env::current_dir().map_err(|no| Fault::refused(no.to_string()))?;
      opening.directory = here.display().to_string();
    }
    let asked = Arc::new(Mutex::new(VecDeque::new()));
    let (mut engine, record) = opening.boot([("console", console(Arc::clone(&asked)))])?;
    let made = record
      .iter()
      .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
      .filter(|one| one.by() == "operator" && MADE.contains(&one.kind()) && one.question())
      .filter(|one| one.about() != "chain1")
      .collect();
    for (id, _) in engine.pending()? {
      let chain = engine.word("scope(id)", vec![("id", Object::string(&id))])?;
      engine.wake(chain.as_ref().as_str().unwrap_or_default())?;
    }
    Ok(Life { engine, made, asked })
  }
}

impl IntoFuture for Open {
  type Output = Result<Life, Fault>;
  type IntoFuture = Ready<Result<Life, Fault>>;

  fn into_future(self) -> Self::IntoFuture {
    ready(self.opened())
  }
}

/// The ear that takes each prompt to the operator as a question.
fn console(asked: Arc<Mutex<VecDeque<Question>>>) -> Box<dyn crate::Ear> {
  ear(move |mut co| async move {
    loop {
      let fact = hear(&mut co).await;
      let word = |at: usize| {
        fact.word(at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
      };
      if fact.question() && fact.kind() == "prompt" && word(3) == "operator" {
        let question = Question {
          id: fact.about().to_owned(),
          chain: Chain(word(0)),
          shape: word(1),
          message: word(2),
        };
        asked.lock().expect("the questions").push_back(question);
        say(&mut co, Fact::says("started", fact.about(), []));
      }
    }
  })
}

/// One life of the engine, as a program holds it.
pub struct Life {
  engine: Engine,
  made: VecDeque<Fact>,
  asked: Arc<Mutex<VecDeque<Question>>>,
}

impl Life {
  /// A life on its record, or a new one when the record holds nothing, which the program awaits.
  pub fn open(record: impl Into<String>) -> Open {
    Open(Opening { record: Some(record.into()), ..Opening::default() })
  }

  /// The engine of the life, for every verb of the contract.
  pub fn engine(&mut self) -> &mut Engine {
    &mut self.engine
  }

  /// The root chain of the life.
  pub fn root(&self) -> Chain {
    Chain(self.engine.root().to_owned())
  }

  /// The act of the record that a program made at this place, when it agrees with this one.
  fn again(&mut self, kind: &str, on: &Chain, words: &[Object]) -> Option<String> {
    let one = self.made.front()?;
    let held: Vec<String> =
      (1..=words.len()).map(|at| one.word(at).map(|w| w.py_repr()).unwrap_or_default()).collect();
    let said: Vec<String> = words.iter().map(Object::py_repr).collect();
    if one.kind() != kind || one.on() != on.0 || held != said {
      return None;
    }
    self.made.pop_front().map(|one| one.about().to_owned())
  }

  /// One view of the engine, by a python expression on these names.
  fn view(&mut self, expression: &str, names: Vec<(&str, Object)>) -> Result<Object, Fault> {
    self.engine.word(expression, names)
  }

  /// A thread on the root: the message to the default actor, which answers it with a value of the shape.
  pub fn prompt<T: Shape>(&mut self, message: &str) -> Result<Thread<T>, Fault> {
    let root = self.root();
    self.prompt_on(&root, message)
  }

  /// A thread on a chain: the message to its actor, which answers it with a value of the shape.
  pub fn prompt_on<T: Shape>(&mut self, chain: &Chain, message: &str) -> Result<Thread<T>, Fault> {
    let words = [Object::string(T::NAME), Object::string(message), Object::string("")];
    let id = match self.again("prompt", chain, &words) {
      Some(id) => id,
      None => {
        let with = verbs::Prompt {
          message: Some(message.to_owned()),
          on: Some(chain.0.clone()),
          ..Default::default()
        };
        self.engine.prompt(Object::string(T::NAME), with)?.id().to_owned()
      }
    };
    Ok(Thread { id, chain: chain.clone(), shape: PhantomData })
  }

  /// A new chain on the root.
  pub fn chain(&mut self, label: &str) -> Result<Chain, Fault> {
    let root = self.root();
    self.chain_on(&root, label)
  }

  /// A new chain on a chain.
  pub fn chain_on(&mut self, on: &Chain, label: &str) -> Result<Chain, Fault> {
    let words = [Object::string(label), Object::string("")];
    let id = match self.again("chain", on, &words) {
      Some(id) => id,
      None => {
        let with = verbs::Chain {
          label: Some(label.to_owned()),
          on: Some(on.0.clone()),
          ..Default::default()
        };
        self.engine.chain(with)?.id().to_owned()
      }
    };
    Ok(Chain(id))
  }

  /// The model that the prompts of a chain go to when they name none.
  pub fn model(&mut self, chain: &Chain, model: &str) -> Result<(), Fault> {
    let word = format!("actor = {model:?}");
    if self
      .again("rung", chain, &[Object::string(&word), Object::string(""), Object::string("")])
      .is_none()
    {
      let with = verbs::Rung { word: Some(word), on: Some(chain.0.clone()), ..Default::default() };
      drop(self.engine.rung(with)?);
    }
    Ok(())
  }

  /// A ceiling in dollars on a chain, which pauses the chain when it reaches it.
  pub fn grant(&mut self, chain: &Chain, usd: f64) -> Result<(), Fault> {
    if self.again("grant", chain, &[Object::float(usd), Object::none()]).is_none() {
      drop(self.engine.grant(verbs::Grant {
        usd: Some(usd),
        on: Some(chain.0.clone()),
        ..Default::default()
      })?);
    }
    Ok(())
  }

  /// Whether a pause holds a chain.
  pub fn is_paused(&mut self, chain: &Chain) -> Result<bool, Fault> {
    Ok(self.view("paused(id)", vec![("id", Object::string(&chain.0))])?.py_repr() == "True")
  }

  /// Wait until a pause holds a chain, as a grant says one at its ceiling.
  pub async fn paused(&mut self, chain: &Chain) -> Result<(), Fault> {
    self.until(|life| life.is_paused(chain)).await
  }

  /// Wake a chain, so its work goes on.
  pub fn wake(&mut self, chain: &Chain) -> Result<(), Fault> {
    self.engine.wake(&chain.0)
  }

  /// The dollars of the answers of the models on a chain.
  pub fn spent(&mut self, chain: &Chain) -> Result<f64, Fault> {
    let sum = "repr(float(sum(x[3][2][4] for x in transcript(id) if x[0] == 'done' and question(('reply', x[1])) \
               and isinstance(x[3], tuple) and x[3][2])))";
    let got = self.view(sum, vec![("id", Object::string(&chain.0))])?;
    Ok(got.as_ref().as_str().and_then(|one| one.parse().ok()).unwrap_or_default())
  }

  /// The label that a chain was opened with.
  pub fn label(&mut self, chain: &Chain) -> Result<String, Fault> {
    let got = self.view("str(get(id)[4])", vec![("id", Object::string(&chain.0))])?;
    Ok(got.as_ref().as_str().unwrap_or_default().to_owned())
  }

  /// The chain that a chain was opened on, and none for the root.
  pub fn parent(&mut self, chain: &Chain) -> Result<Option<Chain>, Fault> {
    let got = self.view("str(get(id)[3])", vec![("id", Object::string(&chain.0))])?;
    Ok(got.as_ref().as_str().filter(|one| !one.is_empty()).map(|one| Chain(one.to_owned())))
  }

  /// Every chain under the root, each before the chains under it.
  pub fn chains(&mut self) -> Result<Vec<Chain>, Fault> {
    let mut found = Vec::new();
    let mut open = vec![self.root()];
    while let Some(on) = open.pop() {
      let facts = self.engine.transcript(verbs::Transcript { on: Some(on.0.clone()), of: None })?;
      let children =
        facts.iter().filter(|one| one.kind() == "chain" && one.question() && one.on() == on.0);
      open.extend(
        children.map(|one| Chain(one.about().to_owned())).collect::<Vec<_>>().into_iter().rev(),
      );
      if on != self.root() {
        found.push(on);
      }
    }
    Ok(found)
  }

  /// Every act on a chain, in order, with named fields.
  pub fn acts(&mut self, chain: &Chain) -> Result<Vec<LifeAct>, Fault> {
    let facts =
      self.engine.transcript(verbs::Transcript { on: Some(chain.0.clone()), of: None })?;
    let mut acts = Vec::new();
    for one in facts.iter().filter(|one| one.question() && one.on() == chain.0) {
      let done = self.engine.outcome(one.about())?.is_some();
      let (kind, id, by, on) = (one.kind(), one.about(), one.by(), one.on());
      acts.push(LifeAct { kind: kind.into(), id: id.into(), by: by.into(), on: on.into(), done });
    }
    Ok(acts)
  }

  /// What a thread comes to, once a model answers it.
  pub async fn wait<T: Shape>(&mut self, thread: Thread<T>) -> Result<T, Fault> {
    Act::<T>::of(&mut self.engine, thread.id).await
  }

  /// Whether a thread has what it came to.
  pub fn done<T>(&mut self, thread: &Thread<T>) -> Result<bool, Fault> {
    Ok(self.engine.outcome(&thread.id)?.is_some())
  }

  /// The next question to the operator, or none once the thread named is done.
  pub async fn question<T>(&mut self, until: &Thread<T>) -> Result<Option<Question>, Fault> {
    let asked = Arc::clone(&self.asked);
    self
      .until(|life| Ok(!asked.lock().expect("the questions").is_empty() || life.done(until)?))
      .await?;
    Ok(asked.lock().expect("the questions").pop_front())
  }

  /// The words of the rungs of a thread, as python, in order.
  pub fn plan<T>(&mut self, thread: &Thread<T>) -> Result<String, Fault> {
    let facts =
      self.engine.transcript(verbs::Transcript { on: Some(thread.chain.0.clone()), of: None })?;
    let mut words: Vec<(String, String)> = Vec::new();
    for one in &facts {
      let word =
        |at: usize| one.word(at).and_then(|w| w.as_str().map(str::to_owned)).unwrap_or_default();
      match one.kind() {
        "module" => words.clear(),
        "run" => {
          let whose = if word(3).is_empty() { word(1) } else { word(3) };
          match words.iter_mut().find(|(held, _)| *held == whose) {
            Some(held) => held.1 = word(2),
            None => words.push((whose, word(2))),
          }
        }
        _ => {}
      }
    }
    let mut plan = Vec::new();
    for (whose, word) in words {
      let under = self.engine.verb(
        "under",
        vec![Object::string(&whose), Object::string(&thread.id)],
        vec![],
      )?;
      if under.py_repr() == "True" {
        plan.push(word);
      }
    }
    Ok(plan.join("\n"))
  }

  /// Wait until the plan of a thread changes, as it does when a model answers it with a word.
  pub async fn step<T>(&mut self, thread: &Thread<T>) -> Result<(), Fault> {
    let before = self.plan(thread)?;
    self.until(|life| Ok(life.done(thread)? || life.plan(thread)? != before)).await
  }

  /// Write the door of a thread with a new plan: the chain cancels each word that the plan drops, and runs the new
  /// words.
  pub fn edit<T>(&mut self, thread: &Thread<T>, plan: &str) -> Result<(), Fault> {
    let door = crate::Text::new(&thread.id, plan);
    self.engine.write(&door, verbs::Write { on: Some(thread.chain.0.clone()) }).map(drop)
  }

  /// Wait until a condition of the life holds, which the life checks again each time that the engine moves.
  async fn until(
    &mut self,
    mut holds: impl FnMut(&mut Life) -> Result<bool, Fault>,
  ) -> Result<(), Fault> {
    poll_fn(|cx| {
      if let Err(fault) = self.engine.pump(cx.waker()) {
        return Poll::Ready(Err(fault));
      }
      match holds(self) {
        Ok(false) => Poll::Pending,
        got => Poll::Ready(got.map(drop)),
      }
    })
    .await
  }
}
