//! The opening of a life, which every host shares: the directory it stands on, the record it opens on and keeps, the
//! ears of the World that the crate writes, the provider of its models among them, and the extensions it runs. A
//! host adds its own ears, such as the console of its operator, which come first in the order of boot, so each takes
//! a question in the place of an ear of the crate, or wraps it.
//!
//! Every life hears the ear of the extensions and the ear of each official extension, whatever it enables, since a
//! record is made again by running its words: a life whose host turns the
//! extensions off enables nothing new, and runs what its record enables all the same. The ears are named as every host names them, so the record of one
//! host opens in another.

#![expect(
  clippy::clone_on_ref_ptr,
  reason = "debt: a clone of an Arc or an Rc that hides its kind"
)]
#![expect(clippy::match_bool, reason = "debt: a match on a bool")]
#![expect(clippy::too_many_lines, reason = "debt: a function longer than 60 lines")]

use std::{
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};

use crate::{
  Ear, Engine, Fault, Object, checkpoint,
  ear::{hear, hearing, lasting},
  extension::{self, Places, skills::skills},
  world::{self, Catalog, Retries, Scratch, Writes, mcp::mcp},
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// What a life waits for before it boots, as [`Opening::prepared`] gives it.
struct Prepared {
  places: Places,
  extensions: Vec<extension::Extension>,
  provider: world::Provider,
  /// The checkpoint that carries the tail of another build, which a life that keeps no record
  /// holds in memory alone, since it writes nothing.
  crossed: Option<Object>,
}

/// A function of the host that answers each request in place of the models: it is given the request and a function
/// that tells what it writes as it writes it, and it gives the turn.
#[derive(Clone)]
pub struct Answer(pub world::Hosted);

/// A function of the host that is told what a model writes, as it writes it: the rung it writes for, the chain of
/// that rung, and what it added to its text and to its thought.
#[derive(Clone)]
pub struct Stream(pub Writes);

/// A function of the host that is told each retry of a call of a model, before its wait: the rung the reply is for,
/// the chain of that rung, the attempt that comes next, the most attempts, the seconds of the wait, and why the
/// attempt before it failed. What the model wrote for that rung before the retry is void.
#[derive(Clone)]
pub struct Retrying(pub Retries);

/// A function of the host that is told each report of the life that no model reads, one line each, such as a file
/// of the record that the scratch directory leaves out. The host shows it in its log.
#[derive(Clone)]
pub struct Log(pub Arc<dyn Fn(&str) + Send + Sync>);

/// What a life is opened on, which every host gives the same: the directory it stands on, the record it opens on and
/// whether it keeps what it says there, whether it does work or only inspects its record, whether it enables the
/// extensions that the configs turn on, the config directory of the user, and what its provider offers. Each part
/// that is unsaid takes the default it names.
#[derive(Default, Clone)]
#[cfg_attr(feature = "typescript", napi_derive::napi(object, object_to_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::FromPyObject), pyo3(from_item_all))]
pub struct Opening {
  /// The directory the life stands on, which each chain stands in until it goes elsewhere, and whose config turns
  /// extensions on.
  pub directory: String,
  /// The record the life opens on; none when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub record: Option<String>,
  /// Whether the life keeps what it says to its record, under the lease of the store; true when unsaid. A life that
  /// keeps nothing reads its record with no lease, and has a scratch directory of its own in place of the one of its
  /// record.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub keeps: Option<bool>,
  /// Whether the life only inspects its record; false when unsaid. Such a life keeps nothing, enables no new
  /// extension and asks no model, and the ears that do work, the files, the commands and time, are silent under
  /// their names, so it runs no command and touches no file.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub inspecting: Option<bool>,
  /// Whether the life enables at its tip the extensions that the configs turn on; true when unsaid. A life runs what
  /// its record enables either way.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub extensions: Option<bool>,
  /// The config directory of the user, which the configs and the official extensions read; the one of this process
  /// when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub config: Option<String>,
  /// The actor a prompt goes to when it names none, as the catalog names a model, and an effort after a slash, at its
  /// level as [`world::Model::at`] moves it; the first model of the roster when unsaid, and
  /// the first when the catalog lacks the model of the actor.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub actor: Option<String>,
  /// The models the provider offers beside the model of the actor, as the catalog names them;
  /// a name that the catalog lacks is dropped, and each chain is told so once.
  /// When it is unsaid, the
  /// provider offers every model that the catalog offers, after the model of the actor; a roster that names none
  /// offers the model of the actor alone, and, with no actor, the operator alone.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub roster: Option<Vec<String>>,
  /// The claude command line to run, in place of the one that `FURB_CLAUDE_BIN` names or this machine holds.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub claude: Option<String>,
  /// How many seconds a turn of the claude command line may go with no progress.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub stall: Option<f64>,
  /// The directory of the images that a turn names, where a read of an image keeps it; when unsaid, the folder
  /// `<record>.images` beside the record that the life keeps, and none for a life that keeps no record.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub images: Option<String>,
  /// A function of the host that answers each request in place of the models.
  #[cfg_attr(
    feature = "typescript",
    napi(
      ts_type = "(request: { actor: string; chain: string; messages: unknown[]; settings: Record<string, unknown> }, write: (delta: { text?: string; thinking?: string }) => void) => Promise<unknown>"
    )
  )]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub answer: Option<Answer>,
  /// A function of the host that is told what a model writes, as it writes it.
  #[cfg_attr(
    feature = "typescript",
    napi(ts_type = "(rung: string, chain: string, text: string, thinking: string) => void")
  )]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub stream: Option<Stream>,
  /// A function of the host that is told each retry of a call of a model, before its wait.
  #[cfg_attr(
    feature = "typescript",
    napi(
      ts_type = "(rung: string, chain: string, attempt: number, most: number, seconds: number, why: string) => void"
    )
  )]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub retrying: Option<Retrying>,
  /// A function of the host that is told each report of the life that no model reads, one line each; the stderr of
  /// this process when unsaid.
  #[cfg_attr(feature = "typescript", napi(ts_type = "(line: string) => void"))]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub log: Option<Log>,
}

impl Opening {
  /// Whether the life does work, which a life that inspects does not.
  fn works(&self) -> bool {
    self.inspecting != Some(true)
  }

  /// The record the life opens on, and the ears of the crate, in the order of boot: the provider,
  /// the extensions, each official extension, the files, the commands and time, and the store and
  /// the log of the life when the life keeps. The ears of the servers of MCP and of
  /// the fetch do work, so a life that inspects hears them silent.
  ///
  /// The files and the commands share the scratch directory of the life: the one of its record when it keeps the
  /// record, and else one of its own, which goes with the life. A new record starts with an empty scratch directory,
  /// and the files of the record are made again in one that is missing.
  pub fn parts(self) -> Result<(Vec<Object>, Ears), Fault> {
    let prepared = self.prepared()?;
    self.parted(prepared)
  }

  /// Whether the life keeps what it says to its record, under the lease of the store.
  pub fn keeps(&self) -> bool {
    self.keeps != Some(false) && self.works() && self.record.is_some()
  }

  /// The directory of the images: the one that the host names, or else the one beside the record that the life keeps.
  fn gallery(&self) -> Option<PathBuf> {
    match (&self.images, &self.record) {
      (Some(images), _) => Some(PathBuf::from(images)),
      (None, Some(record)) if self.keeps() => Some(PathBuf::from(format!("{record}.images"))),
      _ => None,
    }
  }

  /// The record and the ears of the crate, as [`Opening::parts`] gives them, on what the life
  /// waited for.
  fn parted(self, prepared: Prepared) -> Result<(Vec<Object>, Ears), Fault> {
    let works = self.works();
    let keeps = self.keeps != Some(false) && works;
    let Prepared { places, extensions, provider, crossed } = prepared;
    let (mut record, store) = match (&self.record, keeps) {
      (Some(path), true) => world::store(path).map(|(held, store)| (held, Some(store)))?,
      (Some(path), false) => (world::kept(path)?, None),
      (None, _) => (Vec::new(), None),
    };
    record.extend(crossed);
    // The log reads the record whole, so a life after a checkpoint writes on after the log of the lives before it.
    let logs = self.record.as_ref().filter(|_| keeps).map(|path| world::log(path, &record));
    let record = checkpoint::booted(record);
    let log = self.log.as_ref().map(|log| log.0.clone());
    let told = |line: &str| match &log {
      Some(log) => log(line),
      None => eprintln!("furb: {line}"),
    };
    let scratch = works
      .then(|| -> Result<Scratch, Fault> {
        let kept = self
          .record
          .as_ref()
          .filter(|_| keeps)
          .map(|path| Scratch::of(Path::new(path)))
          .transpose()?
          .flatten();
        let scratch = kept.unwrap_or_else(Scratch::own);
        scratch.remade(&record, &told);
        Ok(scratch)
      })
      .transpose()?;
    let gallery = self.gallery();
    let project = PathBuf::from(&self.directory);
    let provider = provider.ear();
    let working = |made: Option<Box<dyn Ear>>| made.unwrap_or_else(silent);
    let config = places.config.clone();
    // An ear that holds nothing that a fresh one lacks starts again after it raises. A fresh ear of the commands
    // reads no record, since the first one ended each command that the record left open.
    let user = config.clone();
    let servers = world::mcp::Settings::of(project, config.clone(), gallery.clone());
    let mut ears = vec![
      ("provider", provider),
      ("extensions", extension::extensions(extensions, places)),
      ("skills", lasting(skills(config.clone()), move || skills(config.clone()))),
      (
        "mcp",
        working(works.then(|| {
          let again = servers.clone();
          lasting(mcp(servers), move || mcp(again.clone()))
        })),
      ),
      ("fetch", working(works.then(|| lasting(world::fetch(), world::fetch)))),
      (
        "files",
        working(scratch.clone().map(|one| {
          let (again, home) = (gallery.clone(), user.clone());
          lasting(world::files(one.clone(), user.clone(), gallery.clone()), move || {
            world::files(one.clone(), home.clone(), again.clone())
          })
        })),
      ),
      (
        "bash",
        working(scratch.map(|one| {
          lasting(world::bash(one.clone(), &record), move || world::bash(one.clone(), &[]))
        })),
      ),
      ("time", working(works.then(|| lasting(world::time(), world::time)))),
    ];
    ears.extend(store.map(|store| ("store", store)));
    ears.extend(logs.map(|log| ("log", log)));
    Ok((record, ears.into_iter().map(|(name, ear)| (name.to_owned(), ear)).collect()))
  }

  /// What the life waits for before it boots: the tail of another build carried, the extensions
  /// that the configs turn on, and the provider of the models, whose catalog asks the servers and
  /// the command lines of the models. A host with a runtime waits for it on a blocking thread.
  fn prepared(&self) -> Result<Prepared, Fault> {
    let works = self.works();
    let keeps = self.keeps != Some(false) && works;
    // A record of another major of its schema is refused before any build reads it whole or writes
    // on it.
    if let Some(path) = self.record.as_deref().filter(|one| Path::new(one).exists()) {
      world::readable(&world::heads(path, &[world::MADE])?)?;
    }
    // The tail that another build made after the last checkpoint is carried at the tip of the
    // record, before the life takes its lease, so the life goes on from it and replays no fact of
    // another engine.
    let record = self.record.as_deref().map(Path::new);
    let crossed = record.map(checkpoint::crossed).transpose()?.flatten();
    if let (Some(path), Some(one)) = (record.filter(|_| keeps), &crossed) {
      checkpoint::keep(path, one.clone())?;
    }
    let crossed = crossed.filter(|_| !keeps);
    let here = Places::here();
    let places = Places { config: self.config.clone().map_or(here.config, PathBuf::from), ..here };
    let extensions = match self.extensions != Some(false) && works {
      true => extension::configured(&places, self.directory.as_ref())?,
      false => Vec::new(),
    };
    // The standing gives each extension whose word or life word plays, with the file of a word that is not blank.
    let advertised = extensions
      .iter()
      .filter(|one| !(one.word.trim().is_empty() && one.life.is_empty()))
      .map(|one| {
        let file = if one.word.trim().is_empty() { String::new() } else { one.file.clone() };
        (one.name.clone(), file, one.life.clone(), one.root)
      })
      .collect();
    let provider = self.provider(advertised)?;
    Ok(Prepared { places, extensions, provider, crossed })
  }

  /// The provider: the models of the roster that the catalog makes, which the function of the host answers when it
  /// gives one, and which a life that inspects asks nothing; and the native search of the web of each of them, which
  /// the provider answers the question search with.
  fn provider(&self, advertised: Vec<world::Advertised>) -> Result<world::Provider, Fault> {
    let works = self.works();
    let gallery = self.gallery();
    let stall = self.stall.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
    let claude = self.claude.as_ref().map(PathBuf::from);
    let config = self.config.as_ref().map(PathBuf::from);
    let directory = self.directory.clone();
    let user = config.clone().unwrap_or_else(|| Places::here().config);
    let catalog = Catalog::load()
      .claude(claude.clone(), stall)
      .configured(Path::new(&directory), config.as_deref())
      .map_err(|no| Fault::refused(no.to_string()))?;
    let asks: world::Hosted =
      Arc::new(|_, _| Box::pin(async { Err("a life that inspects asks no model".to_owned()) }));
    let answer =
      if works { self.answer.as_ref().map(|one| Arc::clone(&one.0)) } else { Some(asks) };
    let kept = catalog.keep(self.roster.as_deref(), self.actor.as_deref());
    let mut provider = catalog
      .provider(directory.clone(), kept.names.as_deref(), kept.actor.as_deref(), answer.clone())
      .map_err(|no| Fault::refused(no.to_string()))?
      .gone(kept.left);
    let names = self.roster.clone();
    let preferred = self.actor.clone();
    provider = provider.refreshing(Arc::new(move || {
      let (claude, config, directory) = (claude.clone(), config.clone(), directory.clone());
      let (names, preferred, answer) = (names.clone(), preferred.clone(), answer.clone());
      Box::pin(async move {
        world::claude::refresh();
        let fresh = Catalog::loaded()
          .await
          .discovered(claude, stall)
          .await
          .configure(Path::new(&directory), config.as_deref())
          .await
          .map_err(|no| no.to_string())?;
        let kept = fresh.keeping(names.as_deref(), preferred.as_deref()).await;
        let (names, actor) = (kept.names.as_deref(), kept.actor.as_deref());
        let candidate =
          fresh.provided(directory, names, actor, answer).await.map_err(|no| no.to_string())?;
        Ok(candidate.into_settings())
      })
    }));
    provider = provider.advertises(advertised).searching(works).config(user);
    if let Some(gallery) = gallery {
      provider = provider.images(gallery);
    }
    if let Some(stream) = &self.stream {
      provider = provider.writes(Arc::clone(&stream.0));
    }
    if let Some(retrying) = &self.retrying {
      provider = provider.retries(Arc::clone(&retrying.0));
    }
    Ok(provider)
  }

  /// The life, booted on the ears of the host and then on the ears of the crate, and the record it
  /// opened on: the entries of its last checkpoint and those after it, as [`checkpoint::booted`]
  /// gives them. A life whose record drifted is refused, since it would keep nothing more.
  /// A life that does work then puts the work that an earlier life left to the outside again, as
  /// [`Engine::resume`] says, so no act of the record stays running with no process.
  pub fn boot<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let prepared = self.prepared()?;
    self.booted(host, prepared)
  }

  /// The life, booted as [`Opening::boot`] boots it, on the runtime of tokio of the thread that awaits it: each ear of
  /// the crate registers its IO on that runtime, and what the life waits for before it boots runs on a blocking thread
  /// of that runtime, so the thread of the life waits on no command and no server. A panic of that wait goes on in the
  /// task that awaits.
  pub async fn open<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let waited = tokio::task::spawn_blocking(move || {
      let prepared = self.prepared();
      (self, prepared)
    });
    let (opening, prepared) = match waited.await {
      Ok(both) => both,
      Err(no) => match no.try_into_panic() {
        Ok(panic) => std::panic::resume_unwind(panic),
        Err(no) => return Err(Fault::refused(format!("the life did not open: {no}"))),
      },
    };
    opening.booted(host, prepared?)
  }

  /// The life booted on the ears of the host, on what it waited for, and then on the ears of the crate.
  fn booted<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
    prepared: Prepared,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let works = self.works();
    let (record, ears) = self.parted(prepared)?;
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear));
    let mut engine = Engine::boot(record.clone(), host.chain(ears))?;
    if let Some(no) = engine.raised() {
      return Err(no.clone());
    }
    // A resume returns to the state at the tip of the record, so the work that the stop cut goes
    // on with no wake.
    if works {
      engine.resume()?;
    }
    Ok((engine, record))
  }
}

/// An ear that takes nothing and says nothing, which stands in the place of an ear that does work.
fn silent() -> Box<dyn Ear> {
  hearing(&[], |mut co| async move {
    loop {
      hear(&mut co).await;
    }
  })
}

mod host;
pub use host::{Chain, Life, LifeAct, Open, Question, Shape, Thread};

#[cfg(test)]
#[path = "life.test.rs"]
mod test;
