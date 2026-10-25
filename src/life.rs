//! The opening of a life, which every host shares: the directory it stands on, the record it opens on and keeps, the
//! ears of the World that the crate writes, the provider of its models among them, and the extensions it runs. A
//! host adds its own ears, such as the console of its operator, which come first in the order of boot, so each takes
//! a question in the place of an ear of the crate, or wraps it.
//!
//! Every life hears the ear of the extensions and the ear of each official extension, whatever it enables, since a
//! record is made again by running its words, or loaded from the snapshot that a run of those words made on the same
//! record, the same build and the same ears, which the life checks against the record: a life whose host turns the
//! extensions off enables nothing new, and runs what its record enables all the same. The ears are named as every host names them, so the record of one
//! host opens in another.

use std::{
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};

use crate::{
  Ear, Engine, Fault, Object, checkpoint,
  ear::{hear, hearing},
  extension::{self, Places, compaction::compaction, memory::memory, skills::skills, web::web},
  snapshot::{self, Keeper},
  world::{self, Catalog, Retries, Scratch, Writes},
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// A function of the host that answers each request in place of the models: it is given the request and a function
/// that tells what it writes as it writes it, and it gives the turn.
pub struct Answer(pub world::Hosted);

/// A function of the host that is told what a model writes, as it writes it: the rung it writes for, the chain of
/// that rung, and what it added to its text and to its thought.
pub struct Stream(pub Writes);

/// A function of the host that is told each retry of a call of a model, before its wait: the rung the reply is for,
/// the chain of that rung, the attempt that comes next, the most attempts, the seconds of the wait, and why the
/// attempt before it failed. What the model wrote for that rung before the retry is void.
pub struct Retrying(pub Retries);

/// A function of the host that is told each report of the life that no model reads, one line each, such as a file
/// of the record that the scratch directory leaves out. The host shows it in its log.
pub struct Log(pub Arc<dyn Fn(&str) + Send + Sync>);

/// What a life is opened on, which every host gives the same: the directory it stands on, the record it opens on and
/// whether it keeps what it says there, whether it does work or only inspects its record, whether it enables the
/// extensions that the configs turn on, the config directory of the user, and what its provider offers. Each part
/// that is unsaid takes the default it names.
#[derive(Default)]
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
  /// level as [`world::Model::at`] moves it; the first model of the roster when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub actor: Option<String>,
  /// The models the provider offers beside the model of the actor, as the catalog names them. When it is unsaid, the
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
  /// The directory of the images that a turn names.
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
  /// Whether the life opens from the snapshot of its record when the snapshot agrees with the record, and keeps one
  /// when it keeps its record; true when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub snapshot: Option<bool>,
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

  /// The record the life opens on, and the ears of the crate, in the order of boot: the provider, the extensions,
  /// each official extension, the files, the commands and time, and the store and the log of the life when the life
  /// keeps. The ears of the web and of the compaction do work, so a life that inspects hears them silent.
  ///
  /// The files and the commands share the scratch directory of the life: the one of its record when it keeps the
  /// record, and else one of its own, which goes with the life. A new record starts with an empty scratch directory,
  /// and the files of the record are made again in one that is missing.
  pub fn parts(self) -> Result<(Vec<Object>, Ears), Fault> {
    self.parted(false).map(|(record, ears, _)| (record, ears))
  }

  /// Whether the life keeps what it says to its record.
  fn keeps(&self) -> bool {
    self.keeps != Some(false) && self.works() && self.record.is_some()
  }

  /// The names of the ears of the crate, in the order of boot.
  fn names(&self) -> Vec<String> {
    let mut names = vec![
      "provider",
      "extensions",
      "memory",
      "skills",
      "web",
      "compaction",
      "files",
      "bash",
      "time",
    ];
    if self.keeps() {
      names.extend(["store", "log"]);
    }
    names.into_iter().map(str::to_owned).collect()
  }

  /// The record and the ears of the crate, as [`Opening::parts`] gives them, with an ear of the log that writes it on
  /// after the lines that it holds when the life is `continued` from a snapshot.
  fn parted(self, continued: bool) -> Result<(Vec<Object>, Ears, bool), Fault> {
    let works = self.works();
    let keeps = self.keeps != Some(false) && works;
    if let Some(path) = self.record.as_deref().filter(|_| keeps) {
      upgraded(Path::new(path), &self.directory, world::stored)?;
    }
    let (record, store) = match (&self.record, keeps) {
      (Some(path), true) => world::store(path).map(|(held, store)| (held, Some(store)))?,
      (Some(path), false) => (world::kept(path)?, None),
      (None, _) => (Vec::new(), None),
    };
    let path = self.record.as_deref().unwrap_or_default();
    let refused = |why: String, then: String| {
      Err(Fault::new(
        "Drift",
        vec![Object::string(format!("{why}, so a replay would drift. {then}"))],
      ))
    };
    // A record with a checkpoint goes on from it on any build, so only the build of its last segment counts.
    let (segment, cut) = (checkpoint::segment(&record), checkpoint::last(&record).is_some());
    match (world::version(segment), world::made(segment)) {
      (Some(theirs), _) if theirs != world::VERSION => {
        let at = world::build(theirs)
          .map_or_else(|| "the store of builds".to_owned(), |at| at.display().to_string());
        return refused(
          format!(
            "the record {path} was made by a build of version {theirs}, and this build is of version {}",
            world::VERSION
          ),
          format!(
            "Open the record with the build of version {theirs}, which stands at {at} once a build of that version ran \
             on this machine, and which script/builds.sh {theirs} makes there. Or start a new life."
          ),
        );
      }
      (None, Some(theirs)) if theirs != world::engine() && !cut => {
        return refused(
          format!(
            "the record {path} was made by the engine {theirs:.12}, and this build runs the engine {:.12}",
            world::engine()
          ),
          "Open the record with the build that made it, or start a new life.".to_owned(),
        );
      }
      _ => {}
    }
    let record = checkpoint::booted(record);
    let log = self.log.as_ref().map(|log| log.0.clone());
    let told = |line: &str| match &log {
      Some(log) => log(line),
      None => eprintln!("furb: {line}"),
    };
    let scratch = works.then(|| {
      let kept =
        self.record.as_ref().filter(|_| keeps).and_then(|path| Scratch::of(Path::new(path)));
      let scratch = kept.unwrap_or_else(Scratch::own);
      scratch.remade(&record, &told);
      scratch
    });
    let here = Places::here();
    let places = Places { config: self.config.clone().map_or(here.config, PathBuf::from), ..here };
    let extensions = match self.extensions != Some(false) && works {
      true => extension::configured(&places, self.directory.as_ref())?,
      false => Vec::new(),
    };
    let logs = self.record.as_ref().filter(|_| keeps).map(|path| match continued && !cut {
      true => world::continued(path, record.len()),
      false => world::log(path, &record),
    });
    let (provider, search) = self.provider()?;
    let working = |made: Option<Box<dyn Ear>>| made.unwrap_or_else(silent);
    let config = places.config;
    let mut ears = vec![
      ("provider", provider),
      ("extensions", extension::extensions(extensions)),
      ("memory", memory(config.clone())),
      ("skills", skills(config)),
      ("web", working(works.then(|| web(search)))),
      ("compaction", working(works.then(compaction))),
      ("files", working(scratch.clone().map(world::files))),
      ("bash", working(scratch.map(|scratch| world::bash(scratch, &record)))),
      ("time", working(works.then(world::time))),
    ];
    ears.extend(store.map(|store| ("store", store)));
    ears.extend(logs.map(|log| ("log", log)));
    Ok((record, ears.into_iter().map(|(name, ear)| (name.to_owned(), ear)).collect(), cut))
  }

  /// The ear of the provider: the models of the roster that the catalog makes, which the function of the host
  /// answers when it gives one, and which a life that inspects asks nothing; and the native search of the web of
  /// each of them, which the ear of the web asks.
  fn provider(self) -> Result<(Box<dyn Ear>, world::Search), Fault> {
    let works = self.works();
    let stall = self.stall.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
    let claude = self.claude.map(PathBuf::from);
    let config = self.config.map(PathBuf::from);
    let directory = self.directory;
    let catalog = Catalog::load()
      .claude(claude.clone(), stall)
      .configured(Path::new(&directory), config.as_deref())
      .map_err(Fault::refused)?;
    let asks: world::Hosted =
      Arc::new(|_, _| Box::pin(async { Err("a life that inspects asks no model".to_owned()) }));
    let answer = if works { self.answer.map(|one| one.0) } else { Some(asks) };
    let (roster, actor) = (self.roster.as_deref(), self.actor.as_deref());
    let mut provider =
      catalog.provider(directory.clone(), roster, actor, answer.clone()).map_err(Fault::refused)?;
    let names = self.roster;
    let preferred = self.actor;
    provider = provider.refreshing(Arc::new(move || {
      let fresh = Catalog::load()
        .claude(claude.clone(), stall)
        .configured(Path::new(&directory), config.as_deref())?;
      let offered = names.as_ref().map(|names| {
        names.iter().filter(|name| fresh.find(name).is_some()).cloned().collect::<Vec<_>>()
      });
      let actor = preferred.as_deref().filter(|name| fresh.roster(Some(&[]), Some(name)).is_ok());
      let candidate =
        fresh.provider(directory.clone(), offered.as_deref(), actor, answer.clone())?;
      Ok(candidate.into_settings())
    }));
    if let Some(images) = self.images {
      provider = provider.images(images);
    }
    if let Some(stream) = self.stream {
      provider = provider.writes(stream.0);
    }
    if let Some(retrying) = self.retrying {
      provider = provider.retries(retrying.0);
    }
    let search = provider.search();
    Ok((provider.ear(), search))
  }

  /// The life, booted on the ears of the host and then on the ears of the crate, and the record it opened on. A life
  /// whose record drifted is refused, since it would keep nothing more.
  ///
  /// A life opens from the snapshot of its record when the snapshot agrees with the record, its log, this build and
  /// these ears, as [`crate::snapshot`] says, and stands again at its tip, as boot does; else it replays the record.
  /// A life that keeps its record keeps its snapshot: after it opens, as it goes on, and when it ends. A snapshot
  /// that agrees and that the ears of this host cannot take, as an ear that speaks at its birth, is dropped, and the
  /// life is refused, so the next life on the record replays.
  pub fn boot<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let host: Vec<(String, Box<dyn Ear>)> =
      host.into_iter().map(|(name, ear)| (name.into(), ear)).collect();
    let mut names: Vec<String> = host.iter().map(|(name, _)| name.clone()).collect();
    names.extend(self.names());
    let build = snapshot::build(&names);
    let (keeps, snapshots) = (self.keeps(), self.snapshot != Some(false));
    let path = self.record.clone().filter(|_| snapshots).map(PathBuf::from);
    let restore = path.as_ref().and_then(|path| snapshot::restore(path, &build));
    let (record, ears, cut) = self.parted(restore.is_some() && keeps)?;
    let ears = host.into_iter().chain(ears);
    let mut engine = match restore {
      // A life that goes on from a checkpoint boots on more entries than its record holds, and keeps no snapshot.
      Some(restore) if restore.entries == record.len() && !cut => {
        let opened =
          Engine::from_snapshot(&record, ears, &restore.table, restore.sand, &restore.past)
            .and_then(|mut engine| engine.verb("stand", vec![], vec![]).map(|_| engine));
        match (opened, &path) {
          (Ok(engine), _) => engine,
          (Err(fault), Some(path)) if keeps => {
            snapshot::dropped(path);
            return Err(fault);
          }
          (Err(fault), _) => return Err(fault),
        }
      }
      Some(_) if !cut => {
        return Err(Fault::refused("the record changed while the life opened on its snapshot"));
      }
      _ => Engine::boot(record.clone(), ears)?,
    };
    if let Some(no) = engine.raised() {
      return Err(no.clone());
    }
    if let Some(path) = path.filter(|_| keeps && !cut) {
      engine.keep(Keeper::new(path, build));
    }
    Ok((engine, record))
  }
}

/// A record whose last segment an older version made, upgraded at a stand: the build of that version, which the store
/// of builds holds, writes a checkpoint at its tip, so a life of this build goes on from it. A record of this version
/// or of a newer one, and one of a version whose build the store lacks, stay as they are, and the life says why it
/// refuses them.
fn upgraded(
  path: &Path,
  directory: &str,
  stored: impl Fn(i64) -> Option<PathBuf>,
) -> Result<(), Fault> {
  if !path.exists() {
    return Ok(());
  }
  let record = world::kept(path)?;
  let older = world::version(checkpoint::segment(&record)).filter(|one| *one < world::VERSION);
  let Some((theirs, program)) = older.and_then(|one| Some((one, stored(one)?))) else {
    return Ok(());
  };
  let ran = std::process::Command::new(&program)
    .arg("checkpoint")
    .arg("--record")
    .arg(path)
    .arg("--cwd")
    .arg(directory)
    .output();
  match ran {
    Ok(done) if done.status.success() => Ok(()),
    Ok(done) => Err(Fault::refused(format!(
      "the build of version {theirs} at {} wrote no checkpoint of {}: {}",
      program.display(),
      path.display(),
      String::from_utf8_lossy(&done.stderr).trim()
    ))),
    Err(no) => Err(Fault::refused(format!(
      "the build of version {theirs} at {} does not run: {no}",
      program.display()
    ))),
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
