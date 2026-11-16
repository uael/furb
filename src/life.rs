//! The opening of a life, which every host shares: the directory it stands on, the record it opens on and keeps, the
//! ears of the World that the crate writes, the provider of its models among them, and the extensions it runs. A
//! host adds its own ears, such as the console of its operator, which come first in the order of boot, so each takes
//! a question in the place of an ear of the crate, or wraps it.
//!
//! Every life hears the ear of the extensions and the ear of each official extension, whatever it enables, since a
//! record is made again by running its words: a life whose host turns the
//! extensions off enables nothing new, and runs what its record enables all the same. The ears are named as every host names them, so the record of one
//! host opens in another.
#![expect(clippy::match_bool, reason = "debt: a match on a bool")]

use std::{
  fs, panic,
  path::{Path, PathBuf},
  slice,
  sync::Arc,
  task::Waker,
  time::Duration,
};

use tokio::task;

pub use furb_wire::Stage;

use crate::{
  Ear, Engine, Fault, Object, ObjectRef, checkpoint,
  ear::{hear, hearing, lasting},
  engine::{OPERATOR, ROOT},
  image::{self, Image},
  minify,
  skill::{self, Places},
  world::{self, Catalog, Mode, Retries, Scratch, Writes, skills::skills},
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// The tip of a record as [`Opening::tip`] finds it: the image that agrees with it, the entries of
/// the record when the life opens by them, and the carry across its last crossing.
type Tip = (Option<Image>, Option<world::Last>, Option<Object>);

/// What a life waits for before it boots, as [`Opening::prepared`] gives it.
struct Prepared {
  waited: Waited,
  /// The entries of the record, which the boot takes, so an open reads them one time.
  last: Option<world::Last>,
  /// The carry that the last engine of the record boots on, when the record crossed, as
  /// [`Opening::carried`] makes it.
  carry: Option<Object>,
  /// The image of the record, when one agrees with it: the life then opens from it, and reads no
  /// entry.
  image: Option<Image>,
  /// The lease of the record that the life keeps, which it holds before it reads the record.
  lease: Option<world::Lease>,
}

/// The extensions that the configs turn on and the provider of the models, which a life waits for
/// before it boots and builds its ears of.
#[derive(Clone)]
struct Waited {
  places: Places,
  played: Vec<skill::Skill>,
  /// The line that tells why each extension that the configs turn on does not play.
  refused: Vec<String>,
  provider: world::Provider,
}

/// How a host takes a life, which says whether it opens by its image and whether it first moves to
/// this build, as [`Opening::tip`] says.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Taken {
  /// Its parts, which the host boots by its record alone, on this build.
  Parts,
  /// Booted, by its image or its record, on this build.
  Boot,
  /// Opened, by its image or its record, on the texts that they hold, which its host moves to this
  /// build after.
  Open,
}

/// What the open of a life from its image comes to.
#[expect(
  clippy::large_enum_variant,
  reason = "an open makes one, which its caller takes apart at once, and no collection holds it"
)]
enum Imaged {
  /// The life at the tip of the image.
  Life(Engine),
  /// No image agrees with the record: the ears of the host, and what the life waited for, on which
  /// it opens by its record.
  Back(Ears, Prepared),
  /// The image does not load: the ears of the host, and what the life waited for, which holds no
  /// record yet. The caller reads the record and its carry off the runtime of the ears, as
  /// [`Opening::reread`] does, since a carry boots engines on the catalog, and the life then opens
  /// by its record.
  Unloaded(Ears, Prepared),
}

/// What `work` gives, from a blocking thread of the runtime of the ears, which may wait on the
/// catalog. A panic of the work goes on in the task that awaits.
async fn blocking<T: Send + 'static>(
  work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, Fault> {
  match task::spawn_blocking(work).await {
    Ok(done) => Ok(done),
    Err(no) => match no.try_into_panic() {
      Ok(panic) => panic::resume_unwind(panic),
      Err(no) => Err(Fault::refused(format!("the life did not open: {no}"))),
    },
  }
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
  /// Whether the life enables at its tip the extensions that the configs turn on; true when unsaid. A life runs what
  /// its record enables either way.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub skills: Option<bool>,
  /// The config directory of the user, which the configs and the official extensions read; the one of this process
  /// when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub config: Option<String>,
  /// The home folder of the user, against which the World resolves each path that starts with `~`,
  /// and from which it tells each path; the one of this process when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub home: Option<String>,
  /// The home of the members who answer as the operator, each by his login, as the access list of a
  /// host and its local user give them, each of which a prompt can ask with
  /// `to="operator/<login>"`; none when unsaid. A host that changes it changes the roster at the
  /// next stand.
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
  /// the log of the life when the life keeps. The ear of the kits does work, so a life that
  /// inspects hears it silent.
  ///
  /// The files and the commands share the scratch directory of the life: the one of its record when it keeps the
  /// record, and else one of its own, which goes with the life. A new record starts with an empty scratch directory,
  /// and the files of the record are made again in one that is missing.
  pub fn parts(self) -> Result<(Vec<Object>, Ears), Fault> {
    let untold = Progress::default();
    let prepared = self.prepared(&untold, Taken::Parts, None)?;
    self.parted(prepared, &untold)
  }

  /// Whether the life keeps what it says to its record, under the lease of the store.
  pub fn keeps(&self) -> bool {
    self.keeps != Some(false) && self.works() && self.record.is_some()
  }

  /// The directory of the images: the one that the host names, or else the one beside the record that the life keeps.
  pub fn gallery(&self) -> Option<PathBuf> {
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
    // A life that keeps its record holds its lease, which its store takes.
    let (last, store) = match (&self.record, prepared.last.take(), prepared.lease.take()) {
      (Some(path), read, Some(lease)) => {
        let (path, bytes) = (Path::new(path), size(path));
        let mut reads = |at| (told.0)(Stage::Read, at, bytes);
        let (last, store) = world::stored(path, lease, read, &mut reads)?;
        (last, Some(store))
      }
      (Some(_), Some(read), None) => (read, None),
      (Some(path), None, None) => (world::last(Path::new(path), &mut |_| {})?, None),
      (None, ..) => (world::Last::default(), None),
    };
    // The log takes the place where the last segment starts, so a life after a crossing writes on
    // after the log of the lives before it.
    let tip = checkpoint::crossings(&last.entries).last().copied();
    let logs = self.record.as_ref().filter(|_| keeps);
    let logs = logs.map(|path| world::logging(path, &last.entries, tip));
    let mut entries = last.entries;
    let record: Vec<Object> =
      prepared.carry.into_iter().chain(entries.drain(tip.unwrap_or(0)..)).collect();
    let mut ears = self.eared(prepared.waited, Some(&record), checkpoint::stood(&record))?;
    ears.extend(store.map(|store| (world::STORE.to_owned(), store)));
    ears.extend(logs.map(|log| ("log".to_owned(), log)));
    Ok((record, ears))
  }

  /// The ears of the crate but the store and the log, as [`Opening::parts`] gives them, for a life
  /// that boots on a record, which holds the standing of its root or not, or for a life that opens
  /// from its image and reads no record: its scratch directory then stands as the life before left
  /// it, which the key of the image holds.
  fn eared(&self, waited: Waited, record: Option<&[Object]>, stood: bool) -> Result<Ears, Fault> {
    let works = self.works();
    let keeps = self.keeps != Some(false) && works;
    let Waited { places, played, refused, provider } = waited;
    let told = |line: &str| self.logged(line);
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
    // An ear that holds nothing that a fresh one lacks starts again after it raises.
    let (user, filing, home) = (places.clone(), places.clone(), places.home.clone());
    let life = self.record.clone().unwrap_or_default();
    let ears = vec![
      ("provider", provider),
      (skill::EAR, skill::telling(played, refused, places.clone())),
      ("skills", lasting(skills(user.clone()), move || skills(user.clone()))),
      ("kits", working(works.then(|| world::kits::kits(life, project, gallery.clone(), places)))),
      (
        "files",
        working(scratch.clone().map(|one| {
          let again = gallery.clone();
          lasting(world::files(one.clone(), filing.clone(), gallery.clone()), move || {
            world::files(one.clone(), filing.clone(), again.clone())
          })
        })),
      ),
      (
        "bash",
        working(scratch.map(|one| {
          lasting(world::bash(one.clone(), home.clone()), move || {
            world::bash(one.clone(), home.clone())
          })
        })),
      ),
      ("time", working(works.then(|| lasting(world::time(), world::time)))),
    ];
    Ok(ears.into_iter().map(|(name, ear)| (name.to_owned(), ear)).collect())
  }

  /// What the life waits for before it boots: the extensions that the configs turn on, and the
  /// provider of the models, whose catalog asks the servers and the command lines of the models. A
  /// host with a runtime waits for it on a blocking thread.
  fn prepared(
    &self,
    told: &Progress,
    taken: Taken,
    lease: Option<world::Lease>,
  ) -> Result<Prepared, Fault> {
    let works = self.works();
    // A life holds the record that it keeps before it reads it, so no other process takes the
    // record between the read and the store of the life.
    let lease = match lease {
      Some(lease) => Some(lease),
      None => self.record.as_deref().filter(|_| self.keeps()).map(world::Lease::of).transpose()?,
    };
    let (image, last, carry) = self.tip(told, taken, lease.as_ref())?;
    // The models of a life read the engine that the life runs: the one that its image holds, or
    // the one that the last segment of its record holds; and each chain that read another engine
    // first reads that one, as the image or the carry says.
    let source = (image.is_some() || last.is_some())
      .then(|| Self::texts(image.as_ref(), last.as_ref()).map(|texts| texts.0))
      .transpose()?;
    let systems = match &image {
      Some(image) => image.half.systems.clone(),
      None => checkpoint::systems(carry.as_slice()),
    };
    let unread = || Fault::refused("this build cannot minify the engine that the life runs");
    let system = source
      .filter(|one| one != crate::ENGINE)
      .map(|one| minify::minified(&one).ok_or_else(unread))
      .transpose()?;
    let here = Places::here();
    let places = Places {
      config: self.config.clone().map_or(here.config, PathBuf::from),
      home: self.home.clone().map(PathBuf::from).or(here.home),
    };
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
    let stood = match &last {
      Some(last) => last.entries.iter().rev().find_map(world::stood),
      None => image.as_ref().map(|image| image.half.stood.clone()),
    };
    let (actor, roster) = stood.as_ref().map_or((None, None), |one| offered(one.as_ref()));
    let actor = self.actor.clone().or(actor);
    let provider =
      self.provider(advertised, actor, roster)?.places(places.clone()).systems(systems);
    let provider = match system {
      Some(one) => provider.system(one.into()),
      None => provider,
    };
    Ok(Prepared { waited: Waited { places, played, refused, provider }, last, carry, image, lease })
  }

  /// The entries of the record, with each byte that the read passes told to `passed`, and none
  /// for a record that is not there yet. A record of another major of its schema is refused before
  /// any build writes on it.
  fn last(&self, passed: &mut dyn FnMut(u64)) -> Result<Option<world::Last>, Fault> {
    let Some(path) = self.record.as_deref().map(Path::new).filter(|one| one.exists()) else {
      return Ok(None);
    };
    let held = world::last(path, passed)?;
    world::readable(&held.entries)?;
    Ok(Some(held))
  }

  /// The carry that the last engine of a record boots on, and nothing when the record never
  /// crossed. Each segment but the last replays in a life that inspects it, on the texts of its own
  /// fact made, after the carry of the segment before it, and gives the carry to the texts that the
  /// next fact made names, as [`checkpoint::carried`] says. So each engine replays only what it
  /// made, and the record stays the one truth of the life. A life that keeps its record starts
  /// from the carry that it kept beside it, as [`checkpoint::kept`] says, and keeps the carry of
  /// the last crossing there, so each segment replays on its engine once.
  fn carried(&self, record: &[Object]) -> Result<Option<Object>, Fault> {
    let crossings = checkpoint::crossings(record);
    if crossings.is_empty() {
      return Ok(None);
    }
    let path = self.record.as_deref().map(Path::new).filter(|_| self.keeps());
    let kept = path.and_then(|one| checkpoint::kept(one, record));
    let start = kept.as_ref().map_or(0, |one| one.0);
    let (mut carry, mut from) = kept.map_or((None, 0), |(at, one)| (Some(one), at));
    let left: Vec<usize> = crossings.into_iter().filter(|at| *at > start).collect();
    if left.is_empty() {
      return Ok(carry);
    }
    let inspects =
      Self { inspecting: Some(true), keeps: Some(false), record: None, ..self.clone() };
    let waited = inspects.prepared(&Progress::default(), Taken::Parts, None)?.waited;
    for at in left {
      let missing = || Fault::refused(format!("the record holds no crossing at {at}"));
      let segment = record.get(from..at).ok_or_else(missing)?;
      let entries: Vec<Object> = carry.into_iter().chain(segment.iter().cloned()).collect();
      let ears = inspects.eared(waited.clone(), Some(&entries), checkpoint::stood(&entries))?;
      let mut engine = Engine::boot(entries, ears)?;
      if let Some(no) = engine.raised() {
        return Err(no.clone());
      }
      carry = Some(checkpoint::carried(&mut engine, record.get(at).ok_or_else(missing)?)?);
      from = at;
    }
    if let (Some(path), Some(carry)) = (path, &carry) {
      checkpoint::keep(path, record, from, carry.clone());
    }
    Ok(carry)
  }

  /// A report of the life that no model reads, to the log of the host, or else to stderr.
  fn logged(&self, line: &str) {
    match &self.log {
      Some(log) => (log.0)(line),
      None => eprintln!("furb: {line}"),
    }
  }

  /// The image of the record when one agrees with it, else the entries of the record, which the
  /// open reads in the stage find, with the carry that its last engine boots on. A life that keeps
  /// its record and runs another engine, Kernel or checkpointer than this build crosses first,
  /// under the lease that it holds, as [`Opening::crossed`] says, and then opens by its record, but
  /// for a life that is [`Taken::Open`]: it opens on those texts, by its image or by its record,
  /// and its host moves it to this build after, by [`Engine::cross`]. A record of a schema that
  /// this build does not open, as one from before the stable cut, is refused, by its image too.
  fn tip(&self, told: &Progress, taken: Taken, lease: Option<&world::Lease>) -> Result<Tip, Fault> {
    // The first stage is told at once, also for a record that is not there yet, so a host never
    // waits for the first one behind the catalog.
    let bytes = self.record.as_deref().map_or(0, size);
    (told.0)(Stage::Find, 0, bytes);
    let image = self.image(taken != Taken::Parts).inspect(|_| (told.0)(Stage::Find, bytes, bytes));
    let last = match image {
      Some(_) => None,
      None => self.last(&mut |at| (told.0)(Stage::Find, at, bytes))?,
    };
    if let Some(image) = &image {
      world::readable(slice::from_ref(&image.half.made))?;
    }
    let texts = Self::texts(image.as_ref(), last.as_ref())?;
    // Only a life that keeps its record holds its lease, so only such a life crosses.
    let crossing = lease.filter(|_| taken != Taken::Open);
    let build = Self::texts(None, None)?;
    let Some(lease) = crossing.filter(|_| texts != build) else {
      let carry = last.as_ref().map(|last| self.carried(&last.entries)).transpose()?.flatten();
      return Ok((image, last, carry));
    };
    (told.0)(Stage::Cross, 0, 0);
    // The record crosses only when it carries to this build, so a record that does not replay
    // stays as it was.
    let mut read = match last {
      Some(last) => last,
      None => self.last(&mut |_| {})?.unwrap_or_default(),
    };
    read.entries.push(world::running(&[]));
    let carry = self.carried(&read.entries)?;
    self.crossed(lease, &read)?;
    Ok((None, self.last(&mut |_| {})?, carry))
  }

  /// The texts of the engine, of the Kernel and of the checkpointer that a life runs: those that
  /// its image holds, or those that the last segment of its record holds, or this build's own.
  fn texts(
    image: Option<&Image>,
    last: Option<&world::Last>,
  ) -> Result<(String, String, String), Fault> {
    match (image, last) {
      (Some(image), _) => world::texts(slice::from_ref(&image.half.made)),
      (None, Some(last)) => world::texts(&last.entries),
      (None, None) => world::texts(&[]),
    }
  }

  /// Moves a life that ran another engine, Kernel or checkpointer of the major across to this
  /// build, so a life never dies on a fault that this build fixed: the record takes after the last
  /// entry that a read of it gave the fact made of this build, as [`checkpoint::crossed`] says,
  /// and each later open carries the life across it, which tells each chain the change. The image
  /// goes, since no life goes on from it.
  fn crossed(&self, lease: &world::Lease, read: &world::Last) -> Result<(), Fault> {
    let Some(path) = self.record.as_deref().map(Path::new) else { return Ok(()) };
    checkpoint::crossed(path, lease.clone(), read)?;
    image::drop_of(path);
    Ok(())
  }

  /// The image of the record, when the open reads images and one agrees with the record: it holds
  /// the life at its tip, so the open then reads no entry.
  fn image(&self, images: bool) -> Option<Image> {
    let record = self.record.as_deref().filter(|_| images)?;
    image::read(Path::new(record))
  }

  /// The catalog of the models that a life of this opening reads: the settings of its config
  /// folder and of its directory, and its claude command line. A host that holds no life yet
  /// lists the models that an open may take from it.
  pub fn catalog(&self) -> Result<Catalog, Fault> {
    let claude = self.claude.as_ref().map(PathBuf::from);
    let stall = self.stall.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
    Catalog::load(claude.clone())
      .claude(claude, stall)
      .configured(Path::new(&self.directory), self.config.as_deref().map(Path::new))
      .map_err(|no| Fault::refused(no.to_string()))
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
    let catalog = self.catalog()?;
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
    provider = provider.advertises(advertised).members(members);
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
  /// opened on: the carry of its last engine, when the record crossed, and the entries of its last
  /// segment, as [`Opening::carried`] says. A life whose record drifted is refused, since it would
  /// keep nothing more.
  /// A life that does work then puts the work that an earlier life left to the outside again, as
  /// [`Engine::resume`] says, so no act of the record stays running with no process.
  pub fn boot<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let untold = Progress::default();
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear)).collect();
    let prepared = self.prepared(&untold, Taken::Boot, None)?;
    match self.imaged(host, prepared, &mut |_| true)? {
      Imaged::Life(engine) => Ok((engine, Vec::new())),
      Imaged::Back(host, prepared) => self.booted(host, prepared, &untold),
      Imaged::Unloaded(host, prepared) => {
        let prepared = self.reread(prepared)?;
        self.booted(host, prepared, &untold)
      }
    }
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
  ///
  /// A life that keeps its record opens on the texts that its image or the last segment of its
  /// record holds, as those of an earlier build, so its host shows it at once: the host then moves
  /// it to this build as a stage of its own, before any word runs, by [`Engine::cross`] and an open
  /// again, while [`Engine::crosses`] says that it runs other texts.
  ///
  /// A host that took the lease of the record before, as it holds a life from its birth, gives it
  /// here, and the life opens under it; else a life that keeps its record takes its lease first.
  #[expect(
    clippy::too_many_arguments,
    reason = "an open takes each thing that its host gives: its ears, its two callbacks and its lease"
  )]
  pub async fn open<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
    told: Progress,
    stands: &mut dyn FnMut(&[u8]) -> bool,
    lease: Option<world::Lease>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear)).collect();
    let (opening, prepared) = self.waited(&told, lease).await?;
    if prepared.image.is_some() {
      (told.0)(Stage::Seed, 0, 0);
    }
    match opening.imaged(host, prepared, stands)? {
      Imaged::Life(engine) => Ok((engine, Vec::new())),
      Imaged::Back(host, prepared) => opening.booted(host, prepared, &told),
      // Boxed, so the future of each open holds no room for a reread that few opens take.
      Imaged::Unloaded(host, prepared) => {
        let (opening, prepared) = blocking(move || {
          let prepared = opening.reread(prepared);
          (opening, prepared)
        })
        .await?;
        opening.booted(host, prepared?, &told)
      }
    }
  }

  /// What the life waits for before it boots, as [`Opening::prepared`] gives it, from a blocking
  /// thread of the runtime.
  async fn waited(
    self,
    told: &Progress,
    lease: Option<world::Lease>,
  ) -> Result<(Self, Prepared), Fault> {
    let waits = told.clone();
    let (opening, prepared) = blocking(move || {
      let prepared = self.prepared(&waits, Taken::Open, lease);
      (self, prepared)
    })
    .await?;
    Ok((opening, prepared?))
  }

  /// What the life waited for, with the record read and the carry across its last crossing, for
  /// a life whose image agreed with the record and did not load: the image named the engine and the
  /// standing of the record at its tip, on which the life waited, so the open reads the record in
  /// the stage seed.
  fn reread(&self, mut prepared: Prepared) -> Result<Prepared, Fault> {
    prepared.last = self.last(&mut |_| {})?;
    let entries = prepared.last.as_ref().map(|last| last.entries.as_slice());
    prepared.carry = self.carried(entries.unwrap_or_default())?;
    Ok(prepared)
  }

  /// The life booted on the ears of the host, on what it waited for, and then on the ears of the crate.
  fn booted(
    self,
    host: Ears,
    prepared: Prepared,
    told: &Progress,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let works = self.works();
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
  /// says a record again, so the open costs that tail alone. A life that works then stands the root
  /// once, as a boot does, and puts each act that an ear of the life of the image held in flight,
  /// and that no such entry ended, to its fresh ear at a wake, as an act that an earlier life took,
  /// as [`Engine::resume`] does after a boot, so each way to open reaches the same state.
  ///
  /// The session steps the ears that the life before had, by their names, so the life needs each
  /// of them and no other: a life that inspects hears a silent ear in the place of each that it
  /// lacks, as it does for each ear that does work. The ears of the host come back when the image
  /// does not load: its ears are not those of this life, no life goes on from it on the record,
  /// as [`world::stored_after`] says, monty does not load its session, or the host does not stand
  /// on its part, which it is asked last, so a host that takes its part opens from the image. The
  /// log of the host says why. The life then opens by its record, on what it waited for, since an
  /// image that agrees names the engine and the standing of the record at its tip, and a life
  /// that keeps drops the image once it stands. The ears come back too when no image agrees with
  /// the record, with what the life waited for, on which it opens by its record.
  fn imaged(
    &self,
    host: Ears,
    mut prepared: Prepared,
    stands: &mut dyn FnMut(&[u8]) -> bool,
  ) -> Result<Imaged, Fault> {
    let Some(image) = prepared.image.take() else { return Ok(Imaged::Back(host, prepared)) };
    let keeps = self.keeps != Some(false) && self.works();
    let record = self.record.as_deref().map(Path::new);
    let path = record.filter(|_| keeps);
    // The entries that a life kept after the image, which went on from it and wrote no later one,
    // or the image is stale.
    let lease = prepared.lease.clone().filter(|_| keeps);
    let after = record.map(|one| world::stored_after(one, image.bytes, lease)).transpose()?;
    let goes = after.as_ref().is_some_and(Option::is_some);
    let world::After { entries: tail, store } = after.flatten().unwrap_or_default();
    let mut ears = self.eared(prepared.waited.clone(), None, true)?;
    if let (Some(store), Some(path)) = (store, path) {
      let log = world::logging_on(path, tail.len());
      ears.extend([(world::STORE.to_owned(), store), ("log".to_owned(), log)]);
    }
    let given = |name: &str| host.iter().chain(&ears).any(|one| one.0 == name);
    let lacked: Vec<String> =
      image.half.names().filter(|name| !given(name)).map(str::to_owned).collect();
    let known = host.iter().chain(&ears).all(|one| image.half.names().any(|name| name == one.0));
    let loaded = match (goes, known && (lacked.is_empty() || !self.works())) {
      (false, _) => Err("a later birth of its record passed it".to_owned()),
      (true, false) => Err("its ears are not the ears of this life".to_owned()),
      (true, true) => image
        .session()
        .map_err(|no| no.to_string())
        .and_then(|session| image.half.sand(&session).map_err(|no| no.to_string())),
    };
    // The host is asked last, so a host that takes its part opens from the image.
    let taken = loaded.and_then(|sand| {
      let taken = image.host().is_ok_and(|part| stands(&part));
      if taken { Ok(sand) } else { Err("its host does not take its part".to_owned()) }
    });
    let sand = match taken {
      Ok(sand) => sand,
      Err(why) => {
        self.logged(&format!("The life opens by its record, since its image does not load: {why}"));
        return Ok(Imaged::Unloaded(host, prepared));
      }
    };
    ears.extend(lacked.into_iter().map(|name| (name, silent())));
    let mut engine = Engine::imaged(image.half, sand, host.into_iter().chain(ears))?;
    if !tail.is_empty() {
      engine.feed(&tail)?;
      if let Some(no) = engine.raised() {
        return Err(no.clone());
      }
    }
    // Each later life that works stands the root once after the state that it opens at, as a boot
    // does. A life that inspects keeps no fact, so it stands at the tip of the record as it is.
    if self.works() {
      engine.verb("stand", vec![Object::string(ROOT)], vec![])?;
      engine.flight()?;
    }
    engine.pump(Waker::noop())?;
    Ok(Imaged::Life(engine))
  }
}

/// The bytes of the record at a path, and 0 for a record that is not there yet.
fn size(path: impl AsRef<Path>) -> u64 {
  fs::metadata(path).map_or(0, |held| held.len())
}

/// The default actor of a standing and the name of each model of its roster, each but the operator,
/// who is no model of the catalog, so a later life never tells that the operator left it.
fn offered(standing: ObjectRef<'_>) -> (Option<String>, Option<Vec<String>>) {
  let Some(held) = standing.items() else { return (None, None) };
  let actor =
    held.get(2).and_then(ObjectRef::as_str).filter(|one| *one != OPERATOR).map(str::to_owned);
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
