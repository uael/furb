//! One life of the engine, as the operator holds it: the World of this machine it stands on, the record it keeps,
//! and what the operator hears of it.

use std::{
  future::{Future, poll_fn},
  pin::Pin,
  task::Poll,
};

use furb::{
  Act, Engine, Fact, Fault, Object,
  engine::OPERATOR,
  fact,
  life::Opening,
  skill::{self, Extension},
  verbs,
  wire::outward,
  world::Catalog,
};

use crate::{console, error::CliError};

/// One life, as the operator holds it: its engine, its root, the facts of the record it was made again from, and what
/// the operator hears of its root.
pub(crate) struct Life {
  pub(crate) engine: Engine,
  pub(crate) root: String,
  held: Vec<Fact>,
}

impl Life {
  /// A life for one command of the operator, on the console of the terminal, which comes
  /// before the ears of the crate. Each ear is heard under the name every host hears it by,
  /// so the record of one opens in another.
  ///
  /// The journal says the whole record again before boot returns, so the life stands whole on its record here, and a
  /// life whose record drifted is refused, since it would keep nothing more.
  pub(crate) async fn lived(opening: Opening) -> Result<Self, CliError> {
    let (engine, held) = opening.open([("console", console::terminal())]).await?;
    let root = engine.root().to_owned();
    let held: Vec<Fact> =
      held.iter().filter_map(|entry| Fact::of(*entry.as_ref().items()?.first()?)).collect();
    Ok(Self { engine, root, held })
  }

  /// The extensions that the life runs, in the order it enabled them.
  pub(crate) fn extensions(&mut self) -> Result<Vec<Extension>, CliError> {
    let root = self.engine.transcript(verbs::Transcript { on: Some(self.root.clone()), of: None });
    Ok(skill::enabled(&root?))
  }

  /// The name of the prompt the record already holds for this message: of the operator, on the root, of this shape
  /// and to this actor; and nothing when it holds none.
  ///
  /// The engine matches nothing the operator says again, so a life stood up on its own record would open a second
  /// prompt beside the one that record stands on, and ask a model for what it was answered once.
  pub(crate) fn again(&self, shape: &str, message: &str, to: &str) -> Option<String> {
    let wanted = [OPERATOR, self.root.as_str(), shape, message, to];
    self.held.iter().find_map(|a| {
      let words =
        a.0.as_ref().items()?.iter().map(furb::ObjectRef::as_str).collect::<Option<Vec<_>>>()?;
      (words.len() == 7 && a.kind() == "prompt" && words[2..] == wanted)
        .then(|| a.about().to_owned())
    })
  }

  /// Whether a pause stands over an act of the operator on the root, or over the root, as the
  /// engine reads it.
  pub(crate) fn paused(&mut self, id: &str) -> Result<bool, CliError> {
    paused(&mut self.engine, id)
  }

  /// What an act of the operator on the root came to, awaited until it is done, as the operator waits on its own
  /// loop; or why it never will be: a pause over the act or over the root, which no one wakes while one command
  /// runs, or work that an earlier life left and that the act awaits alone, which no wake runs while one command
  /// runs.
  pub(crate) async fn settled(&mut self, id: &str) -> Result<Object, CliError> {
    let Self { engine, root, .. } = self;
    poll_fn(|cx| {
      if let Poll::Ready(got) = Pin::new(&mut Act::<Object>::of(engine, id)).poll(cx) {
        return Poll::Ready(got.map_err(CliError::from));
      }
      match paused(engine, id) {
        Ok(false) => Poll::Pending,
        Ok(true) => Poll::Ready(
          refused(engine, root)
            .and_then(|refused| Err(CliError::Paused { id: id.to_owned(), refused })),
        ),
        Err(no) => Poll::Ready(Err(no)),
      }
    })
    .await
  }
}

/// Whether the engine reads a pause over an act.
fn paused(engine: &mut Engine, id: &str) -> Result<bool, CliError> {
  let paused = engine.verb("paused", vec![Object::string(id)], vec![])?;
  Ok(outward(paused.as_ref()) == serde_json::Value::Bool(true))
}

/// Why a reply of the root was refused last, which says why the World paused a chain.
fn refused(engine: &mut Engine, root: &str) -> Result<Option<String>, CliError> {
  let done = verbs::Transcript { on: Some(root.to_owned()), of: Some("done".to_owned()) };
  let facts = engine.transcript(done)?;
  let mut why = facts.iter().rev().filter(|a| fact::named(a.about(), "reply"));
  Ok(
    why
      .find_map(|a| a.word(0).and_then(Fault::of))
      .map(|no| no.message())
      .filter(|why| !why.is_empty()),
  )
}

/// The shape of a prompt of the operator that names none: a str, so the model works until it closes the prompt with
/// its report.
pub(crate) const SHAPE: &str = "str";

/// An actor as the roster names it, and its model.
pub(crate) struct Actor {
  /// The actor, as provider:model/effort.
  pub(crate) name: String,
  /// The model of the actor, as provider:model, when the catalog knows one by its name.
  pub(crate) model: Option<String>,
}

/// An actor as the roster names it, and its model: the model the catalog finds by the name, with
/// the effort moved to the nearest one that model takes; or the name as it is, and no model, when
/// the catalog knows no model by it, the operator among them.
pub(crate) async fn actor(to: &str) -> Actor {
  match Catalog::loaded().await.rostered(Some(&[]), Some(to)).await {
    Ok((models, actor)) => Actor {
      name: actor.unwrap_or_else(|| to.to_owned()),
      model: models.first().map(|one| one.name().to_owned()),
    },
    // A name that the catalog does not know is no error here: it names an actor of no model.
    Err(_) => Actor { name: to.to_owned(), model: None },
  }
}
