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

use std::{
  fs, panic,
  path::{Path, PathBuf},
  sync::Arc,
  task::Waker,
  time::Duration,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::task;

use crate::{
  Ear, Engine, Fault, Object, ObjectRef, checkpoint,
  ear::{hear, hearing, lasting},
  engine::OPERATOR,
  image::{self, Image},
  minify,
  skill::{self, Places},
  world::{self, Catalog, Mode, Retries, Scratch, Writes, mcp::mcp, skills::skills},
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// What a life waits for before it boots, as [`Opening::prepared`] gives it.
struct Prepared {
  places: Places,
  played: Vec<skill::Skill>,
  /// The line that tells why each extension that the configs turn on does not play.
  refused: Vec<String>,
  provider: world::Provider,
  /// The last entries of the record, which the boot takes, so an open reads them one time.
  last: Option<world::Last>,
  /// The image of the record, when one agrees with it: the life then opens from it, and reads no
  /// entry.
  image: Option<Image>,
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

/// A stage of the open of a life, in the order that an open passes them. An open skips a stage that
/// it has no work for, and a stage never comes again in one open. Each stage counts its work in a
/// unit of its own, and a stage that counts nothing tells 0 of 0.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
  /// A host tells its readers what the record told before its last checkpoint: the entries that
  /// the record had kept when the life said the fact told last, of its entries as far as that
  /// checkpoint. The crate never tells this stage.
  Recall,
  /// The record is read from its last checkpoint, which the read finds from the end of the record,
  /// for the build of its last life: the bytes passed, of the bytes of the record. It is the first
  /// stage that the crate tells, at once, and the life then reads its settings and the catalog of
  /// its models in it. A life that opens from its image reads no entry, and tells each byte passed
  /// once the image agrees with the record.
  Find,
  /// A record whose last life ran another engine is carried across to this build, by a life that
  /// inspects it and writes its checkpoint. It counts nothing.
  Cross,
  /// The record is read again from its last checkpoint under its lease, when it changed after
  /// the stage find: the bytes passed, of the bytes of the record.
  Read,
  /// The sandbox loads the engine, takes the data of the checkpoint, and the checkpointer makes the
  /// entries of it, or the sandbox loads the session of the image of the life, which then boots on
  /// no entry. It counts nothing.
  Seed,
  /// The life boots: the entries that its journal passed, of the entries that it boots on. It is
  /// the last stage that the crate tells, and a boot that succeeds ends with each entry counted.
  Boot,
  /// A host puts what the open told in the log that its readers read: the events put, of those
  /// events. The crate never tells this stage.
  Log,
}

/// A function of the host that is told how far the open of a life stands, as [`Opening::open`]
/// passes each [`Stage`]: the stage, the work done and the work known. It is told at the start of
/// each stage and then once for each line of the record or each entry, from the thread that does
/// the work, so it must be cheap.
#[derive(Clone)]
pub struct Progress(pub Arc<dyn Fn(Stage, u64, u64) + Send + Sync>);

impl Default for Progress {
  /// A function that tells no one.
  fn default() -> Self {
    Self(Arc::new(|_, _, _| {}))
  }
}

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
  /// Whether the life only inspects its record; false when unsaid. Such a life keeps nothing,
  /// enables no new extension, asks no model and says no new stand, so its standing is the one its
  /// record holds, and the ears that do work, the files, the commands and time, are silent under
  /// their names, so it runs no command and touches no file.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub inspecting: Option<bool>,
  /// Whether the life opens from the first entry of its record, born again at each checkpoint as
  /// each life of the record was, and not from its last checkpoint alone; false when unsaid. Each
  /// checkpoint then has to end the life that wrote it and to hold what a life born on the entries
  /// before it gathers, or the open is refused, as [`checkpoint::reborn`] says. Both opens reach
  /// one state at the tip, so the tests and the gates open this way to prove each checkpoint of a
  /// record.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub whole: Option<bool>,
  /// Whether the life enables at its tip the extensions that the configs turn on; true when unsaid. A life runs what
  /// its record enables either way.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub skills: Option<bool>,
  /// The config directory of the user, which the configs and the official extensions read; the one of this process
  /// when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub config: Option<String>,
  /// The home of the logins of the members who answer as the operator, as the access list of a host
  /// and its local login give them, each of which a prompt can ask with `to="operator/<login>"`;
  /// none when unsaid. A host that changes it changes the roster at the next stand.
  #[cfg_attr(feature = "typescript", napi(ts_type = "Array<string>"))]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub members: Option<world::Members>,
  /// The actor a prompt goes to when it names none, as the catalog names a model, and an effort after a slash, at its
  /// level as [`world::Model::at`] moves it. When unsaid, the default actor of the last stand
  /// that the record answered, so a life opened again goes on with the actor it had, and on a
  /// record with no stand, the first model of the roster; the first when the catalog lacks the
  /// model of the actor.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub actor: Option<String>,
  /// The home of the models the provider offers beside the model of the actor, as the catalog
  /// names them; a name that the catalog lacks is dropped, and each chain is told so once.
  /// When it is unsaid or holds none, the
  /// roster of the last stand that the record answered, less the operator, so a life opened
  /// again goes on with the roster it had, and on a record with no stand, the
  /// provider offers every model that the catalog offers, after the model of the actor; a roster that names none
  /// offers the model of the actor alone, and, with no actor, the operator alone. A host that
  /// puts other names in the home changes the roster at the next stand that reads the catalog
  /// again, with no new life, and the record keeps the answer of that stand.
  #[cfg_attr(feature = "typescript", napi(ts_type = "Array<string>"))]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub roster: Option<world::Roster>,
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
    let untold = Progress::default();
    let prepared = self.prepared(&untold, false)?;
    self.parted(prepared, &untold)
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
  fn parted(self, mut prepared: Prepared, told: &Progress) -> Result<(Vec<Object>, Ears), Fault> {
    let keeps = self.keeps != Some(false) && self.works();
    let (last, store) = match (&self.record, prepared.last.take(), keeps) {
      (Some(path), read, true) => {
        let (path, bytes) = (Path::new(path), size(path));
        let mut reads = |at| (told.0)(Stage::Read, at, bytes);
        let (last, store) = world::stored(path, read, &mut reads)?;
        (last, Some(store))
      }
      (Some(_), Some(read), false) => (read, None),
      (Some(path), None, false) => (world::last(Path::new(path), &mut |_| {})?, None),
      (None, ..) => (world::Last::default(), None),
    };
    // The log takes where the last entries start, so a life after a checkpoint writes on after the
    // log of the lives before it.
    let logs = self.record.as_ref().filter(|_| keeps);
    let logs = logs.map(|path| world::logging(path, &last.entries, last.from));
    let record = checkpoint::booted(last.entries);
    let mut ears = self.eared(prepared, Some(&record), checkpoint::stood(&record))?;
    ears.extend(store.map(|store| ("store".to_owned(), store)));
    ears.extend(logs.map(|log| ("log".to_owned(), log)));
    Ok((record, ears))
  }

  /// The ears of the crate but the store and the log, as [`Opening::parts`] gives them, for a life
  /// that boots on a record, which holds the standing of its root or not, or for a life that opens
  /// from its image and reads no record: its scratch directory then stands as the life before left
  /// it, which the key of the image holds.
  fn eared(
    &self,
    prepared: Prepared,
    record: Option<&[Object]>,
    stood: bool,
  ) -> Result<Ears, Fault> {
    let works = self.works();
    let keeps = self.keeps != Some(false) && works;
    let Prepared { places, played, refused, provider, .. } = prepared;
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
        record.map_or_else(|| scratch.hold(), |record| scratch.remade(record, &told));
        scratch.reap(&places.config, Path::new(&self.directory));
        Ok(scratch)
      })
      .transpose()?;
    let gallery = self.gallery();
    let project = PathBuf::from(&self.directory);
    let provider = provider.mode(Mode::of(works, stood)).ear();
    let working = |made: Option<Box<dyn Ear>>| made.unwrap_or_else(silent);
    let config = places.config.clone();
    // An ear that holds nothing that a fresh one lacks starts again after it raises.
    let user = config.clone();
    let servers = world::mcp::Settings::of(project, config.clone(), gallery.clone());
    let ears = vec![
      ("provider", provider),
      (skill::EAR, skill::telling(played, refused, places)),
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
        working(
          scratch.map(|one| lasting(world::bash(one.clone()), move || world::bash(one.clone()))),
        ),
      ),
      ("time", working(works.then(|| lasting(world::time(), world::time)))),
    ];
    Ok(ears.into_iter().map(|(name, ear)| (name.to_owned(), ear)).collect())
  }

  /// What the life waits for before it boots: the extensions that the configs turn on, and the
  /// provider of the models, whose catalog asks the servers and the command lines of the models. A
  /// host with a runtime waits for it on a blocking thread.
  fn prepared(&self, told: &Progress, images: bool) -> Result<Prepared, Fault> {
    let works = self.works();
    // The first stage is told at once, also for a record that is not there yet, so a host never
    // waits for the first one behind the catalog.
    let bytes = self.record.as_deref().map_or(0, size);
    (told.0)(Stage::Find, 0, bytes);
    let read = |told: &Progress| match self.record.as_deref().map(Path::new) {
      Some(path) if path.exists() => {
        world::last(path, &mut |at| (told.0)(Stage::Find, at, bytes)).map(Some)
      }
      _ => Ok(None),
    };
    // A record of another major of its schema is refused before any build writes on it.
    let image = self.image(images).inspect(|_| (told.0)(Stage::Find, bytes, bytes));
    let mut last = if image.is_some() { None } else { read(told)? };
    if let (Some(held), Some(path)) = (&last, self.record.as_deref()) {
      // The build that wrote the checkpoint names the schema of the record before it, and each
      // build after it the schema of what it wrote.
      world::readable(world::made_before(Path::new(path), held.from)?.as_slice())?;
      world::readable(&held.entries)?;
      // A crossing writes a checkpoint, so the last entries of before it are those of another
      // record.
      if self.keeps() && self.crossed(&held.entries, told)? {
        last = read(&Progress::default())?;
      }
    }
    // The models of a life read the engine that the life runs: the one that its image holds, or
    // the one that the last segment of its record holds.
    let source = match (&image, &last) {
      (Some(image), _) => Some(image.half.source.clone()),
      (None, Some(last)) => Some(world::texts(checkpoint::segment(&last.entries))?.0),
      (None, None) => None,
    };
    let unread = || Fault::refused("this build cannot minify the engine that the life runs");
    let system = source
      .filter(|one| one != crate::ENGINE)
      .map(|one| minify::minified(&one).ok_or_else(unread))
      .transpose()?;
    let here = Places::here();
    let places = Places { config: self.config.clone().map_or(here.config, PathBuf::from), ..here };
    let (played, refused) = match self.skills != Some(false) && works {
      true => skill::configured(&places, self.directory.as_ref())?,
      false => (Vec::new(), Vec::new()),
    };
    // The standing gives each extension whose word or life word plays, with the file of a word that is not blank.
    let advertised = played
      .iter()
      .filter(|one| !(one.word.trim().is_empty() && one.life.is_empty()))
      .map(|one| {
        let file = if one.word.trim().is_empty() { String::new() } else { one.file.clone() };
        (one.name.clone(), file, one.life.clone(), one.root)
      })
      .collect();
    // A life that names no actor or no roster goes on with the ones that its record stood on last.
    let tip = image.as_ref().map(|image| image.half.stood.clone());
    let stood = match (&last, self.record.as_deref()) {
      (Some(last), Some(path)) => match checkpoint::standing(&last.entries) {
        // A checkpoint whose last stand was refused carries no standing, and the record still holds
        // the one that the stand before it gave.
        None => world::stood_before(Path::new(path), last.from)?,
        stood => stood,
      },
      _ => tip,
    };
    let (actor, roster) = stood.as_ref().map_or((None, None), |one| offered(one.as_ref()));
    let actor = self.actor.clone().or(actor);
    let provider = match system {
      Some(one) => self.provider(advertised, actor, roster)?.system(one.into()),
      None => self.provider(advertised, actor, roster)?,
    };
    Ok(Prepared { places, played, refused, provider, last, image })
  }

  /// The image of the record, when the open reads images and one agrees with the record: it holds
  /// the life at its tip, so the open then reads no entry. A life that proves each checkpoint of
  /// its record opens by the record.
  fn image(&self, images: bool) -> Option<Image> {
    let record = self.record.as_deref().filter(|_| images && self.whole != Some(true))?;
    image::read(Path::new(record))
  }

  /// The provider: the models of the roster that the catalog makes, which the function of the host answers when it
  /// gives one, and which a life that inspects asks nothing; and the native search of the web of each of them, which
  /// the provider answers the question search with. Its roster is the one of the home of the
  /// opening at each read of the catalog, else the one that the record stood on last.
  fn provider(
    &self,
    advertised: Vec<world::Advertised>,
    actor: Option<String>,
    stood: Option<Vec<String>>,
  ) -> Result<world::Provider, Fault> {
    let home = self.roster.clone().unwrap_or_default();
    let roster = move || home.get().or_else(|| stood.clone());
    let works = self.works();
    let gallery = self.gallery();
    let stall = self.stall.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
    let claude = self.claude.as_ref().map(PathBuf::from);
    let config = self.config.as_ref().map(PathBuf::from);
    let directory = self.directory.clone();
    let user = config.clone().unwrap_or_else(|| Places::here().config);
    let catalog = Catalog::load(claude.clone())
      .claude(claude.clone(), stall)
      .configured(Path::new(&directory), config.as_deref())
      .map_err(|no| Fault::refused(no.to_string()))?;
    let asks: world::Hosted =
      Arc::new(|_, _| Box::pin(async { Err("a life that inspects asks no model".to_owned()) }));
    let answer =
      if works { self.answer.as_ref().map(|one| Arc::clone(&one.0)) } else { Some(asks) };
    let kept = catalog.keep(roster().as_deref(), actor.as_deref());
    let mut provider = catalog
      .provider(directory.clone(), kept.names.as_deref(), kept.actor.as_deref(), answer.clone())
      .map_err(|no| Fault::refused(no.to_string()))?
      .gone(kept.left);
    let preferred = actor;
    provider = provider.refreshing(Arc::new(move || {
      let (claude, config, directory) = (claude.clone(), config.clone(), directory.clone());
      let (names, preferred, answer) = (roster(), preferred.clone(), answer.clone());
      Box::pin(async move {
        world::claude::refresh(claude.as_deref());
        let fresh = Catalog::loaded(claude.clone())
          .await
          .discovered(claude, stall)
          .await
          .configure(Path::new(&directory), config.as_deref())
          .await?;
        let kept = fresh.keeping(names.as_deref(), preferred.as_deref()).await;
        let (names, actor) = (kept.names.as_deref(), kept.actor.as_deref());
        let candidate = fresh.provided(directory, names, actor, answer).await?;
        Ok(candidate.into_settings())
      })
    }));
    let members = self.members.clone().unwrap_or_default();
    provider = provider.advertises(advertised).config(user).members(members);
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
    let untold = Progress::default();
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear)).collect();
    let prepared = self.prepared(&untold, true)?;
    let (host, prepared) = match self.imaged(host, prepared, &mut |_| true)? {
      Ok(engine) => return Ok((engine, Vec::new())),
      Err(back) => back,
    };
    let prepared = prepared.map_or_else(|| self.prepared(&untold, false), Ok)?;
    self.booted(host, prepared, &untold)
  }

  /// The life, booted as [`Opening::boot`] boots it, on the runtime of tokio of the thread that awaits it: each ear of
  /// the crate registers its IO on that runtime, and what the life waits for before it boots runs on a blocking thread
  /// of that runtime, so the thread of the life waits on no command and no server. A panic of that wait goes on in the
  /// task that awaits. The host is told how far the open stands, stage by stage, from the blocking
  /// thread and then from the thread that awaits.
  ///
  /// Before the life loads its image, `stands` is given the part of the host that the image holds,
  /// as the host of the life before gave it to [`image::keep`], on the thread that awaits: it
  /// says whether the host stands on it, and the life opens by its record when it does not.
  pub async fn open<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
    told: Progress,
    stands: &mut dyn FnMut(&[u8]) -> bool,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear)).collect();
    let (opening, prepared) = self.waited(&told, true).await?;
    if prepared.image.is_some() {
      (told.0)(Stage::Seed, 0, 0);
    }
    let (host, prepared) = match opening.imaged(host, prepared, stands)? {
      Ok(engine) => return Ok((engine, Vec::new())),
      Err(back) => back,
    };
    let (opening, prepared) = match prepared {
      Some(prepared) => (opening, prepared),
      None => opening.waited(&told, false).await?,
    };
    opening.booted(host, prepared, &told)
  }

  /// What the life waits for before it boots, as [`Opening::prepared`] gives it, from a blocking
  /// thread of the runtime.
  async fn waited(self, told: &Progress, images: bool) -> Result<(Self, Prepared), Fault> {
    let waits = told.clone();
    let waited = task::spawn_blocking(move || {
      let prepared = self.prepared(&waits, images);
      (self, prepared)
    });
    match waited.await {
      Ok((opening, prepared)) => Ok((opening, prepared?)),
      Err(no) => match no.try_into_panic() {
        Ok(panic) => panic::resume_unwind(panic),
        Err(no) => Err(Fault::refused(format!("the life did not open: {no}"))),
      },
    }
  }

  /// The life booted on the ears of the host, on what it waited for, and then on the ears of the crate.
  fn booted(
    self,
    host: Ears,
    prepared: Prepared,
    told: &Progress,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let works = self.works();
    if self.whole == Some(true) {
      self.reborn()?;
    }
    let kept = self.record.as_ref().filter(|_| self.keeps()).map(PathBuf::from);
    let (record, ears) = self.parted(prepared, told)?;
    (told.0)(Stage::Seed, 0, 0);
    let boots = told.clone();
    let replayed = Box::new(move |done, of| (boots.0)(Stage::Boot, done, of));
    let mut engine = Engine::booting(record.clone(), host.into_iter().chain(ears), replayed)?;
    if let Some(no) = engine.raised() {
      return Err(no.clone());
    }
    // The life stands on its record by the record, so the image beside it is of no life that
    // goes on.
    if let Some(record) = &kept {
      image::drop_of(record);
    }
    // A resume returns to the state at the tip of the record, so the work that the stop cut goes
    // on with no wake.
    if works {
      engine.resume()?;
    }
    Ok((engine, record))
  }

  /// The life that the image of its record holds, on the ears of the host and then on fresh ears
  /// of the crate, each of which stands where the ear of the life before stood. No boot runs, and
  /// no entry before the image is read. A life that went on from the image and wrote no later one,
  /// as one that crashed, kept entries after it: the journal of the life says them again, as a boot
  /// says a record again, so the open costs that tail alone. Each act that an ear of the life of
  /// the image held in flight, and that no such entry ended, is put to its fresh ear as an act that
  /// an earlier life took, at the first time the life is driven, which this does once.
  ///
  /// The session steps the ears that the life before had, by their names, so the life needs each
  /// of them and no other: a life that inspects hears a silent ear in the place of each that it
  /// lacks, as it does for each ear that does work. The ears of the host come back when the image
  /// does not load: its ears are not those of this life, no life goes on from it on the record,
  /// as [`world::stored_after`] says,
  /// monty does not load its session, or the host does not stand on its part, which it is asked
  /// last, so a host that takes its part opens from the image. The life then opens by its record,
  /// on what it waits for again with no image, and a life that keeps drops the image once it
  /// stands. The ears come back too when no image agrees with the record, with what the life
  /// waited for, on which it opens by its record.
  fn imaged(
    &self,
    host: Ears,
    mut prepared: Prepared,
    stands: &mut dyn FnMut(&[u8]) -> bool,
  ) -> Result<Result<Engine, (Ears, Option<Prepared>)>, Fault> {
    let Some(image) = prepared.image.take() else { return Ok(Err((host, Some(prepared)))) };
    let keeps = self.keeps != Some(false) && self.works();
    let record = self.record.as_deref().map(Path::new);
    let path = record.filter(|_| keeps);
    // The entries that a life kept after the image, which went on from it and wrote no later one,
    // or the image is stale.
    let after = record.map(|one| world::stored_after(one, image.bytes, keeps)).transpose()?;
    let goes = after.as_ref().is_some_and(Option::is_some);
    let world::After { entries: tail, store } = after.flatten().unwrap_or_default();
    let mut ears = self.eared(prepared, None, true)?;
    if let (Some(store), Some(path)) = (store, path) {
      let log = world::logging_on(path, tail.len());
      ears.extend([("store".to_owned(), store), ("log".to_owned(), log)]);
    }
    let given = |name: &str| host.iter().chain(&ears).any(|one| one.0 == name);
    let lacked: Vec<String> =
      image.half.names().filter(|name| !given(name)).map(str::to_owned).collect();
    let known = host.iter().chain(&ears).all(|one| image.half.names().any(|name| name == one.0));
    let loads = known && goes && (lacked.is_empty() || !self.works());
    let sand = loads.then(|| image.half.sand(image.session()).ok()).flatten();
    let Some(sand) = sand.filter(|_| stands(image.host())) else {
      return Ok(Err((host, None)));
    };
    ears.extend(lacked.into_iter().map(|name| (name, silent())));
    let mut engine = Engine::imaged(image.half, sand, host.into_iter().chain(ears))?;
    if !tail.is_empty() {
      engine.feed(&tail)?;
      if let Some(no) = engine.raised() {
        return Err(no.clone());
      }
    }
    engine.pump(Waker::noop())?;
    Ok(Ok(engine))
  }

  /// Carries a record whose last life ran another engine of the major across to this build, so a
  /// life never dies on a fault that this build fixed in the kernel: a life that inspects the
  /// record, born on the engine that its fact made holds, writes its checkpoint after its last
  /// entry, as a life that ends writes one, and the life opens from that checkpoint on the engine
  /// of this build. A record of this build, or of a build whose fact made holds no texts, stays as
  /// it is. It reads the last entries of the record, and says whether it wrote that checkpoint.
  fn crossed(&self, last: &[Object], told: &Progress) -> Result<bool, Fault> {
    let segment = checkpoint::segment(last);
    let (source, ..) = world::texts(segment)?;
    let crosses = source != crate::ENGINE && !segment.is_empty();
    let Some(path) = self.record.as_deref().filter(|_| crosses) else {
      return Ok(false);
    };
    (told.0)(Stage::Cross, 0, 0);
    let record = checkpoint::booted(world::last(Path::new(path), &mut |_| {})?.entries);
    let inspects = Self { inspecting: Some(true), keeps: Some(false), whole: None, ..self.clone() };
    let prepared = inspects.prepared(&Progress::default(), false)?;
    let ears = inspects.eared(prepared, Some(&record), checkpoint::stood(&record))?;
    let mut engine = Engine::boot(record, ears)?;
    if let Some(no) = engine.raised() {
      return Err(no.clone());
    }
    checkpoint::keep(Path::new(path), checkpoint::written(&mut engine)?)?;
    Ok(true)
  }

  /// Proves each checkpoint of the record: a life that inspects the entries before it, born on them
  /// as the life that wrote it was, gathers it again, as [`checkpoint::reborn`] says.
  fn reborn(&self) -> Result<(), Fault> {
    let Some(path) = self.record.as_deref().filter(|one| Path::new(one).exists()) else {
      return Ok(());
    };
    let record = world::kept(path)?;
    let inspects = Self { inspecting: Some(true), keeps: Some(false), whole: None, ..self.clone() };
    for at in checkpoint::all(&record) {
      let before = checkpoint::booted(record[..at].to_vec());
      let prepared = inspects.prepared(&Progress::default(), false)?;
      let ears = inspects.eared(prepared, Some(&before), checkpoint::stood(&before))?;
      let mut engine = Engine::boot(before, ears)?;
      if let Some(no) = engine.raised() {
        return Err(no.clone());
      }
      checkpoint::reborn(&record, at, &mut engine)?;
    }
    Ok(())
  }
}

/// The bytes of the record at a path, and 0 for a record that is not there yet.
fn size(path: impl AsRef<Path>) -> u64 {
  fs::metadata(path).map_or(0, |held| held.len())
}

/// The default actor of a standing, and the name of each model of its roster but the operator.
fn offered(standing: ObjectRef<'_>) -> (Option<String>, Option<Vec<String>>) {
  let Some(held) = standing.items() else { return (None, None) };
  let actor = held.get(2).and_then(ObjectRef::as_str).map(str::to_owned);
  let roster = held.first().and_then(ObjectRef::items).map(|models| {
    let names =
      models.iter().filter_map(|model| Some(model.items()?.first()?.as_str()?.to_owned()));
    names.filter(|name| name != OPERATOR).collect()
  });
  (actor, roster)
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
pub use host::{Chain, Life, Open, Question, Shape, Thread};

#[cfg(test)]
#[path = "life.test.rs"]
mod test;
