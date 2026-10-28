//! The doors of the crate, one per host language, each behind the feature that names it, and the host API they
//! give, which stands here once.
//!
//! Every function of the host API is defined here, and each door gives it under the same name, in the case of its
//! language. What a door holds of its own is how a value of its language crosses, its engine, and the loop that
//! drives that engine. A door hands the engine to a host that is no rust: the host's ears cross as one object heard
//! by name, the verbs of the contract are said by name with their words, and every value crosses as monty carries
//! it, made into what that language holds. Nothing of the engine lives in a door.
//!
//! A value of a host's language comes in as a [`Value`]: plain data as itself, a generator as an ear, and a function
//! as one the sandbox calls back. Each door hands the ears and the functions over to the life the value comes into,
//! and a verb that a host says while a life hears it goes to that life.

#[cfg(feature = "python")]
pub mod py;
#[cfg(feature = "typescript")]
pub mod ts;

use std::{
  any::Any,
  cell::RefCell,
  collections::HashMap,
  mem::ManuallyDrop,
  ops::{Deref, DerefMut},
  rc::{Rc, Weak},
  sync::{Arc, Mutex, PoisonError},
  task::{Context, Poll},
  thread::{self, ThreadId},
};

#[cfg(feature = "typescript")]
use napi_derive::napi;
#[cfg(feature = "python")]
use pyo3::pyfunction;

use crate::{
  Ear, Fact, Fault, Heard, Life, Object, Spoken, Step,
  extension::{self, Extension, Part},
  life::Opening,
  wire,
  world::{self, images},
};

/// A value of the engine, as a host language holds it, both ways.
pub struct Value(pub Object);

/// A value as the record keeps it, which a later life reads the same: in TypeScript a whole float keeps its mark.
pub struct Record(pub Object);

/// An ear that a host gives: a generator of its language, heard as an ear, or an ear of the crate.
pub struct Given(pub Box<dyn Ear>);

/// A fact as a host holds it, read for its text alone: nothing when one of its words is no text.
pub struct Said(pub Option<Vec<String>>);

/// A function of the host that is told the name of an event, from any thread, and whether the host was told.
pub struct Told(pub Arc<dyn Fn(&str) -> bool + Send + Sync>);

/// An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear. It is let go
/// when it is disposed, so what it holds goes: a command ends, a wait ends, and a store lets its record go. The
/// engine of python steps it as a generator of its own too.
///
/// It stays on the thread that made it, as a [`Homed`] value: a reach of it from another thread panics, and a drop of
/// it on another thread leaves it for the thread that made it to drop.
#[cfg_attr(feature = "python", pyo3::pyclass(module = "furb_monty._monty", weakref))]
#[cfg_attr(feature = "typescript", napi)]
pub struct NativeEar {
  hearing: Homed<Hearing>,
}

/// The ear that a [`Lent`] lends, which is gone once the ear is given back.
type Loan = RefCell<Option<Box<dyn Ear>>>;
/// The ear, and what the engine of python steps it with once it is born there.
struct Hearing {
  ear: Option<Box<dyn Ear>>,
  lent: Option<Weak<Loan>>,
  #[cfg(feature = "python")]
  stepped: Option<py::Stepped>,
}

/// A value of the thread that made it, in an object of a host, which only that thread reaches. The collector of
/// cycles of python drops an object on whatever thread runs it, as the reader of a gateway of xdist, so a value that
/// another thread drops goes to [`ORPHANS`], and its home thread drops it when it makes its next homed value.
pub(crate) struct Homed<T: 'static> {
  home: ThreadId,
  held: ManuallyDrop<T>,
}

// SAFETY: the value is reached on its home thread alone, which each deref checks. Another thread only moves it, whole
// and untouched, into ORPHANS, and the home thread alone drops it there.
unsafe impl<T> Send for Homed<T> {}
// SAFETY: as for Send, since no thread but the home thread reaches the value.
unsafe impl<T> Sync for Homed<T> {}

/// A value that another thread dropped, which waits for its home thread.
struct Orphan {
  home: ThreadId,
  _held: Box<dyn Any>,
}

// SAFETY: an orphan is moved between threads whole, and only its home thread drops it, in `Homed::new`.
unsafe impl Send for Orphan {}

/// The values that another thread dropped, each dropped later by its home thread. A thread that ends with some keeps
/// them.
static ORPHANS: Mutex<Vec<Orphan>> = Mutex::new(Vec::new());

impl<T: 'static> Homed<T> {
  /// A value of this thread, made after this thread drops each of its values that another thread dropped. They are
  /// dropped outside the lock, since a drop may make a homed value.
  pub(crate) fn new(held: T) -> Homed<T> {
    let home = thread::current().id();
    let mine: Vec<Orphan> = {
      let mut orphans = ORPHANS.lock().unwrap_or_else(PoisonError::into_inner);
      let (mine, others) = orphans.drain(..).partition(|one| one.home == home);
      *orphans = others;
      mine
    };
    drop(mine);
    Homed { home, held: ManuallyDrop::new(held) }
  }

  fn home(&self) {
    assert_eq!(thread::current().id(), self.home, "a value of one thread is reached on another");
  }
}

impl<T> Deref for Homed<T> {
  type Target = T;

  fn deref(&self) -> &T {
    self.home();
    &self.held
  }
}

impl<T> DerefMut for Homed<T> {
  fn deref_mut(&mut self) -> &mut T {
    self.home();
    &mut self.held
  }
}

impl<T> Drop for Homed<T> {
  fn drop(&mut self) {
    if thread::current().id() == self.home {
      // SAFETY: the value is dropped once, here, and never reached after.
      unsafe { ManuallyDrop::drop(&mut self.held) };
    } else {
      // SAFETY: the value is taken once, here, and never reached after.
      let held = Box::new(unsafe { ManuallyDrop::take(&mut self.held) });
      let orphan = Orphan { home: self.home, _held: held };
      ORPHANS.lock().unwrap_or_else(PoisonError::into_inner).push(orphan);
    }
  }
}

impl NativeEar {
  fn of(ear: Box<dyn Ear>) -> NativeEar {
    let hearing = Hearing {
      ear: Some(ear),
      lent: None,
      #[cfg(feature = "python")]
      stepped: None,
    };
    NativeEar { hearing: Homed::new(hearing) }
  }

  /// The ear, lent out, since an ear hears in one engine. The engine holds it, and this ear holds the way to it, so a
  /// dispose lets it go while the engine lives.
  fn taken(&mut self) -> Result<Box<dyn Ear>, Fault> {
    let hearing = &mut self.hearing;
    #[cfg(feature = "python")]
    if hearing.stepped.is_some() {
      return Err(Fault::refused("an ear of the crate hears in one engine, and this one hears"));
    }
    let ear = hearing.ear.take().ok_or_else(|| {
      Fault::refused("an ear of the crate hears in one engine, and this one hears")
    })?;
    let shared = Rc::new(RefCell::new(Some(ear)));
    hearing.lent = Some(Rc::downgrade(&shared));
    Ok(Box::new(Lent(shared)))
  }
}

/// An ear of the crate as an engine holds it, which its [`NativeEar`] lets go at a dispose: from then on it is over.
struct Lent(Rc<Loan>);

impl Ear for Lent {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    match self.0.borrow_mut().as_mut() {
      Some(ear) => ear.resume(heard, cx),
      None => Step::Over,
    }
  }

  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    match self.0.borrow_mut().as_mut() {
      Some(ear) => ear.poll(cx),
      None => Poll::Pending,
    }
  }

  fn answered(&mut self, got: Result<Object, Fault>) {
    if let Some(ear) = self.0.borrow_mut().as_mut() {
      ear.answered(got);
    }
  }

  fn lent(&mut self, heard: Heard, cx: &mut Context<'_>, life: Life) -> (Step, Life) {
    match self.0.borrow_mut().as_mut() {
      Some(ear) => ear.lent(heard, cx, life),
      None => (Step::Over, life),
    }
  }

  fn ran(&mut self) -> Result<(), Fault> {
    self.0.borrow_mut().as_mut().map_or(Ok(()), |ear| ear.ran())
  }

  fn behind(&self) -> bool {
    self.0.try_borrow().is_ok_and(|ear| ear.as_ref().is_some_and(|ear| ear.behind()))
  }
}

#[cfg_attr(feature = "python", pyo3::pymethods)]
#[cfg_attr(feature = "typescript", napi)]
impl NativeEar {
  /// The ear is let go before any engine hears it, or after the life it heard in, so what it holds goes: a command
  /// ends, a wait ends, and a store lets its record go.
  #[cfg_attr(feature = "typescript", napi)]
  pub fn dispose(&mut self) -> Result<(), Fault> {
    let hearing = &mut self.hearing;
    hearing.ear.take();
    if let Some(shared) = hearing.lent.take().and_then(|one| one.upgrade()) {
      let ear = shared
        .try_borrow_mut()
        .map_err(|_| Fault::refused("the ear hears, and cannot be let go now"))?
        .take();
      drop(ear);
    }
    Ok(())
  }
}

/// A model of the catalog, as a host reads it: its name, its efforts, its window in tokens, whether it takes an
/// image, and its price in dollars for a million tokens read, written, read from the cache and written to it.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct ModelInfo {
  pub name: String,
  pub efforts: Vec<String>,
  pub window: i64,
  pub images: bool,
  pub price: Option<Vec<f64>>,
}

impl ModelInfo {
  fn of(model: &world::Model) -> ModelInfo {
    ModelInfo {
      name: model.name().to_owned(),
      efforts: model.efforts().into_iter().map(str::to_owned).collect(),
      window: i64::try_from(model.window()).unwrap_or(i64::MAX),
      images: model.sees(),
      price: model.priced().map(Vec::from),
    }
  }
}

/// An image a host attached: the name of its file, the uri a message names it by, its media type, and its size in
/// bytes.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct ImageAttachment {
  pub name: String,
  pub uri: String,
  pub mime_type: String,
  pub size: i64,
}

/// The bytes of an image as base64, and its media type.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct ImageContent {
  pub data: String,
  pub mime_type: String,
}

/// The file that holds an image, and the digest its bytes have.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct ImagePath {
  pub path: String,
  pub digest: String,
}

/// The type of an image: its media type and its extension.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct ImageType {
  pub mime_type: String,
  pub extension: String,
}

/// What an act came to, and whether it is done: nothing while it lives.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct Outcome {
  pub done: bool,
  #[cfg_attr(feature = "typescript", napi(ts_type = "unknown"))]
  pub value: Value,
}

/// One name of a chain, read without calling it: the name, the name of its type, its representation in the sandbox,
/// and its value, which crosses as every value does.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct Inspection {
  pub name: String,
  pub kind: String,
  pub representation: String,
  #[cfg_attr(feature = "typescript", napi(ts_type = "unknown"))]
  pub value: Value,
}

/// The ear of the files, which reads and writes a path, and a path of `scratch://` in the scratch directory that the
/// host names, which the ear of commands is given too.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn files(scratch: String) -> NativeEar {
  NativeEar::of(world::files(world::Scratch::at(scratch), None))
}

/// The ear of commands, which runs each in a shell of this machine, with `TMPDIR` set to the scratch directory that
/// the host names, which the ear of the files is given too. It is given the record that the life opens on, so it
/// closes as interrupted each command that the record shows started and not done, at the wake.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn bash(scratch: String, record: Vec<Value>) -> NativeEar {
  let record: Vec<Object> = record.into_iter().map(|one| one.0).collect();
  NativeEar::of(world::bash(world::Scratch::at(scratch), &record))
}

/// The scratch directory of the record at a path, in the cache of furb, which a host removes when it deletes the
/// record; nothing when the machine has no cache of furb.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn scratch(record: String) -> Option<String> {
  world::Scratch::of(record.as_ref()).map(|one| one.path().display().to_string())
}

/// The ear of time, which reads the clock, draws a chance, and ends a wait.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn time() -> NativeEar {
  NativeEar::of(world::time())
}

/// The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says to
/// keep: a database of SQLite when the name of the path ends in `.sqlite`, `.sqlite3` or `.db`, and else a file of
/// lines.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_return_type = "[unknown[], NativeEar]"))]
pub fn store(path: String) -> Result<(Record, NativeEar), Fault> {
  let (record, ear) = world::store(&path)?;
  Ok((Record(Object::list(record)), NativeEar::of(ear)))
}

/// What the store kept at a path, read with no lease and changed in nothing.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_return_type = "unknown[]"))]
pub fn kept(path: String) -> Result<Record, Fault> {
  Ok(Record(Object::list(world::kept(&path)?)))
}

/// The ear of the log of the life beside the record at a path, which writes it again from its first line.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn log(path: String) -> Result<NativeEar, Fault> {
  let held = world::kept(&path)?;
  Ok(NativeEar::of(world::log(&path, &held)))
}

/// The facts of the log of the record at a path, read with no lease, when the log agrees with the record: every fact
/// that the life that wrote it said, as an ear of a host hears it, the rungs of each ladder among them. Nothing when
/// the record has no log, or a log that does not agree with it.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_return_type = "unknown[] | null"))]
pub fn logged(path: String) -> Result<Option<Value>, Fault> {
  Ok(world::logged(&path)?.map(|facts| Value(Object::list(facts))))
}

/// What the record at a path tells, read with no life, so no word runs again and a record of any engine opens: the
/// engine that made it, as the record names it, and nothing when it names none; the engine of this build; the
/// version of the build that made it, as the record names it, and nothing when it names none; the version of this
/// build; whether
/// the facts are those of its log; and the file that holds the facts, one on each line, in order. The facts are those
/// of the log when it agrees with the record, and else those that the record kept, with the chains, the rungs, the
/// runs and the dones that they name. A line of the file that does not start with `[`, and a line `["keep", "", n]`,
/// holds no fact.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(
  feature = "typescript",
  napi(
    ts_return_type = "{ made: string | null; engine: string; version: number | null; current: number; logged: boolean; file: string }"
  )
)]
pub fn retold(path: String) -> Result<Value, Fault> {
  let world::Retold { made, version, logged, file, .. } = world::retold(&path)?;
  Ok(Value(Object::dict([
    (Object::string("made"), made.map_or_else(Object::none, Object::string)),
    (Object::string("engine"), Object::string(world::engine())),
    (Object::string("version"), version.map_or_else(Object::none, Object::int)),
    (Object::string("current"), Object::int(world::VERSION)),
    (Object::string("logged"), Object::bool(logged)),
    (Object::string("file"), Object::string(file.display().to_string())),
  ])))
}

/// One line of a record, read with every number exact, as the record keeps it.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_return_type = "unknown"))]
pub fn decode_record(line: String) -> Result<Record, Fault> {
  wire::parsed(&line).map(Record)
}

/// The record a life opens on, and the ears of the crate that it hears after the ears of the host, as every host
/// of the crate opens a life: the provider, the extensions, each official extension, the files, the commands, time,
/// and the store of the record when the life keeps.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (**opening)))]
#[cfg_attr(
  feature = "typescript",
  napi(ts_return_type = "[unknown[], Array<[string, NativeEar]>]")
)]
pub fn opened(opening: Option<Opening>) -> Result<(Record, Vec<(String, NativeEar)>), Fault> {
  let (record, ears) =
    opening.ok_or_else(|| Fault::refused("a life opens on a directory"))?.parts()?;
  let ears = ears.into_iter().map(|(name, ear)| (name, NativeEar::of(ear))).collect();
  Ok((Record(Object::list(record)), ears))
}

/// The gate of the crate: what the type checker of monty found on a sheet, each error by its line, and no warning.
/// The checker is the one every engine of this thread gates with, so a word is judged once and the same.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn gate(sheet: String) -> Result<Vec<(i64, String)>, Fault> {
  let found = crate::gate::checked(&sheet)?;
  Ok(found.into_iter().map(|(line, why)| (i64::try_from(line).unwrap_or(i64::MAX), why)).collect())
}

/// The official extensions, in the order a life runs them.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn official() -> Vec<Extension> {
  extension::official()
}

/// The extensions that a life runs, in the order it enabled them, as the facts of its root say them.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_args_type = "root: unknown[][]"))]
pub fn enabled(root: Vec<Said>) -> Vec<Extension> {
  let words = root.into_iter().filter_map(|one| one.0);
  let facts: Vec<Fact> =
    words.map(|words| Fact(Object::tuple(words.iter().map(Object::string)))).collect();
  extension::enabled(&facts)
}

/// The ear of the extensions, given each extension that the life runs: it enables each at the tip of the life,
/// unless the record enables it, and plays each as a rung on each chain.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn extensions(given: Vec<Extension>) -> NativeEar {
  NativeEar::of(extension::extensions(given))
}

/// The part for a TUI of each extension that a life in the directory of a project runs and whose manifest names
/// one, as the configs of the user and of the project say it now, which a TUI loads beside the life.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn parts(project: String) -> Result<Vec<Part>, Fault> {
  extension::parts(&extension::Places::here(), std::path::Path::new(&project))
}

/// The ear of the memory extension, which finds the memory of a path in its folders and in the config directory.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn memory(config: String) -> NativeEar {
  NativeEar::of(extension::memory::memory(config.into()))
}

/// The ear of the skills extension, which finds skills in the folders of a chain and in the config directory.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn skills(config: String) -> NativeEar {
  NativeEar::of(extension::skills::skills(config.into()))
}

/// The config directory of the user for this process, where the configs, the extensions of the user and the
/// preferences of the TUI stand.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn config_directory() -> String {
  extension::Places::here().config.display().to_string()
}

/// The POSIX shell that runs a command of this machine, which a host runs its own commands in too.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn shell() -> String {
  world::shell().display().to_string()
}

/// Every shape the operator answers, by its name.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn shapes() -> Vec<&'static str> {
  world::SHAPES.to_vec()
}

/// Whether the operator answers a shape: one of the shapes it offers, or a Literal of values that a line names.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn answers(shape: String) -> bool {
  world::answers(&shape)
}

/// A line of the operator as a value of the shape a prompt wants, by the rules every console of the crate reads a
/// line by, as the record keeps it; or the refusal of a line that is no value of the shape, and of a shape the
/// operator answers not.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_return_type = "unknown"))]
pub fn answered(shape: String, line: String) -> Result<Record, Fault> {
  world::answered(&shape, &line).map(Record)
}

/// The models that the user and project settings offer, with the claude command line at a path when it is given.
/// The directory is the current directory when unsaid, and config is the user config directory when unsaid.
/// Invalid settings give a refusal.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (claude = None, directory = None, config = None)))]
#[cfg_attr(feature = "typescript", napi)]
pub fn models(
  claude: Option<String>,
  directory: Option<String>,
  config: Option<String>,
) -> Result<Vec<ModelInfo>, Fault> {
  let catalog = configured_catalog(claude, directory, config)?;
  Ok(catalog.offered().iter().map(ModelInfo::of).collect())
}

fn configured_catalog(
  claude: Option<String>,
  directory: Option<String>,
  config: Option<String>,
) -> Result<world::Catalog, Fault> {
  let directory = directory
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
  let config = config.map(std::path::PathBuf::from);
  world::Catalog::load()
    .claude(claude.map(Into::into), None)
    .configured(&directory, config.as_deref())
    .map_err(Fault::refused)
}

/// What a first run checks of this machine, as a host reads it: whether the claude command line offers a model and
/// whether it is signed in, each configured provider with the names of its credential and how many of its models the
/// catalog offers, and the local server that `FURB_LOCAL_URL` names, with the models it offers or why it listed none.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct SetupInfo {
  pub claude: bool,
  pub signed: bool,
  pub providers: Vec<ProviderInfo>,
  pub local: Option<LocalInfo>,
}

/// A provider as a first run checks it: its name, the names of its credential in the environment, whether the
/// settings enable it, and how many of its models the catalog offers.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct ProviderInfo {
  pub provider: String,
  pub env: Vec<String>,
  pub offered: i64,
  /// Whether the provider settings enable this provider.
  pub enabled: bool,
}

/// The local server as a first run checks it: its address, how many models the catalog offers of it, and why it
/// listed none, when it did not answer.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct LocalInfo {
  pub address: String,
  pub offered: i64,
  pub failed: Option<String>,
}

/// What a first run checks with the user and project settings, and the claude command line at a path when it is
/// given. It asks the local server for what it lists, so it waits for that server. The directory is the current
/// directory when unsaid, config is the user config directory when unsaid, and invalid settings give a refusal.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (claude = None, directory = None, config = None)))]
#[cfg_attr(feature = "typescript", napi)]
pub fn setup(
  claude: Option<String>,
  directory: Option<String>,
  config: Option<String>,
) -> Result<SetupInfo, Fault> {
  let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
  let found = configured_catalog(claude, directory, config)?.setup();
  Ok(SetupInfo {
    claude: found.claude,
    signed: found.signed,
    providers: (found.providers.into_iter())
      .map(|one| ProviderInfo {
        provider: one.provider,
        env: one.env,
        offered: count(one.offered),
        enabled: one.enabled,
      })
      .collect(),
    local: found.local.map(|one| LocalInfo {
      address: one.address,
      offered: count(one.offered),
      failed: one.failed,
    }),
  })
}

/// The model the catalog knows by a name, as `provider:id` or as an id that one model alone holds, which for a model of
/// the local server holds a colon, whether it offers that model or not; nothing when it knows none.
/// It reads the user and project settings, with the current directory and the user config directory when unsaid.
/// Invalid settings give a refusal.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (name, directory = None, config = None)))]
#[cfg_attr(feature = "typescript", napi)]
pub fn model(
  name: String,
  directory: Option<String>,
  config: Option<String>,
) -> Result<Option<ModelInfo>, Fault> {
  Ok(configured_catalog(None, directory, config)?.find(&name).as_ref().map(ModelInfo::of))
}

/// A ChatGPT sign-in state, with its browser address while it waits and a message when it fails. It holds no token.
#[cfg_attr(feature = "typescript", napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct CodexAuthInfo {
  pub state: String,
  pub url: Option<String>,
  pub message: Option<String>,
}

/// Read, start, refresh or cancel ChatGPT sign-in for codex. The default action reads its state.
/// The config directory is the user config directory when unsaid. A start returns before the browser completes.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (action = None, config = None)))]
#[cfg_attr(feature = "typescript", napi)]
pub fn codex_auth(action: Option<String>, config: Option<String>) -> Result<CodexAuthInfo, Fault> {
  use crate::world::codex as auth;
  let path = config.map(std::path::PathBuf::from);
  let status = match action.as_deref().unwrap_or("status") {
    "status" => auth::status(path.as_deref()),
    "start" => auth::begin(path.as_deref()).map_err(Fault::refused)?,
    "refresh" => auth::refresh(path.as_deref()).map_err(Fault::refused)?,
    "cancel" => {
      auth::cancel(path.as_deref());
      auth::status(path.as_deref())
    }
    _ => return Err(Fault::refused("Use status, start, refresh or cancel for codex")),
  };
  Ok(CodexAuthInfo { state: status.state, url: status.url, message: status.message })
}

/// The levels of effort, from least to most, which an actor names after its model.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn levels() -> Vec<&'static str> {
  world::catalog::LEVELS.to_vec()
}

/// An image copied into a directory of images under the digest of its bytes, as a message attaches it.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn attach_image(directory: String, path: String) -> Result<ImageAttachment, Fault> {
  let got = images::attach(directory.as_ref(), path.as_ref()).map_err(Fault::refused)?;
  let size = i64::try_from(got.size).unwrap_or(i64::MAX);
  Ok(ImageAttachment { name: got.name, uri: got.uri, mime_type: got.media.to_owned(), size })
}

/// The bytes of the image of a uri, as base64, and its media type, once the bytes have the digest the uri names.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_content(directory: String, uri: String) -> Result<ImageContent, Fault> {
  use base64::{Engine as _, engine::general_purpose::STANDARD};
  let (media, bytes) = images::read(directory.as_ref(), &uri).map_err(Fault::refused)?;
  Ok(ImageContent { data: STANDARD.encode(bytes), mime_type: media.to_owned() })
}

/// The file that holds the image of a uri, and the digest its bytes have.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_path(directory: String, uri: String) -> Result<ImagePath, Fault> {
  let (path, digest) = images::path(directory.as_ref(), &uri).map_err(Fault::refused)?;
  Ok(ImagePath { path: path.display().to_string(), digest })
}

/// How a message names an image: `![name](uri)`.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_reference(name: String, uri: String) -> String {
  images::reference(&name, &uri)
}

/// Each image a message names, as its text in the message, its name, and its uri.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_references(message: String) -> Vec<images::Named> {
  images::references(&message)
}

/// The type of an image by its first bytes: its media type and its extension, for a PNG, a JPEG, a GIF or a WebP.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_type(data: &[u8]) -> Result<ImageType, Fault> {
  let (media, extension) = images::kind(data).map_err(Fault::refused)?;
  Ok(ImageType { mime_type: media.to_owned(), extension: extension.to_owned() })
}

/// The words of a call that a host gave, by position and by name, as the engine takes them.
fn words(
  args: Option<Vec<Value>>,
  kwargs: &Option<HashMap<String, Value>>,
) -> (Vec<Object>, Vec<(&str, Object)>) {
  let args = args.unwrap_or_default().into_iter().map(|one| one.0).collect();
  (args, kwargs.iter().flatten().map(|(key, one)| (key.as_str(), one.0.clone())).collect())
}

/// Call back when the console of Windows ends this process: at Ctrl+Break, at the close of the console, at a logoff
/// and at a shutdown, with the name of the event: `break`, `close`, `logoff` or `shutdown`. The system holds the
/// process until the callback ends it, or, at a close, a logoff or a shutdown, until the system's own limit. A later
/// callback replaces an earlier one. Ctrl+C stays SIGINT, which the host hears as a signal. A system that is not
/// Windows has no such console, and gives this callback no event.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(
  feature = "typescript",
  napi(
    ts_args_type = "callback: (event: \"break\" | \"close\" | \"logoff\" | \"shutdown\") => void"
  )
)]
pub fn on_console_end(callback: Told) -> Result<(), Fault> {
  #[cfg(windows)]
  console::listen(callback)?;
  #[cfg(not(windows))]
  drop(callback);
  Ok(())
}

#[cfg(windows)]
mod console {
  use std::sync::OnceLock;

  use windows_sys::{
    Win32::System::Console::{
      CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
      SetConsoleCtrlHandler,
    },
    core::BOOL,
  };

  use tokio::sync::{mpsc, oneshot};

  use super::Told;
  use crate::{Fault, ear};

  /// What the keeper of the callback of the console is told: a callback, which replaces the one before it, or an
  /// event, whose answer says whether the host was told.
  enum Ending {
    Callback(Told),
    Event(&'static str, oneshot::Sender<bool>),
  }

  /// The keeper of the callback of the host that hears the end of the console, which it owns alone.
  fn ending() -> &'static mpsc::UnboundedSender<Ending> {
    static ENDING: OnceLock<mpsc::UnboundedSender<Ending>> = OnceLock::new();
    ENDING.get_or_init(|| {
      let (tells, mut told) = mpsc::unbounded_channel();
      ear::reactor().spawn(async move {
        let mut callback: Option<Told> = None;
        while let Some(one) = told.recv().await {
          match one {
            Ending::Callback(one) => callback = Some(one),
            Ending::Event(name, answer) => {
              let _ = answer.send(callback.as_ref().is_some_and(|one| (one.0)(name)));
            }
          }
        }
      });
      tells
    })
  }

  /// The callback, kept, and the handler of this module, added to the handlers of the process, once.
  pub fn listen(callback: Told) -> Result<(), Fault> {
    ending()
      .send(Ending::Callback(callback))
      .map_err(|_| Fault::refused("the keeper of the console callback is gone"))?;
    static ADDED: OnceLock<bool> = OnceLock::new();
    // The system calls the handlers from the last added to the first, so this one comes before those of the
    // runtime, which end the process at once.
    if *ADDED.get_or_init(|| unsafe { SetConsoleCtrlHandler(Some(heard), 1) } != 0) {
      Ok(())
    } else {
      let why = std::io::Error::last_os_error();
      Err(Fault::refused(format!("The console handler was not added: {why}")))
    }
  }

  /// The system calls this on a thread of its own. It gives the event to the host and keeps the thread, since the
  /// system ends the process once a handler returns. An event with no callback, and Ctrl+C, go on to the next handler.
  unsafe extern "system" fn heard(event: u32) -> BOOL {
    let name = match event {
      CTRL_BREAK_EVENT => "break",
      CTRL_CLOSE_EVENT => "close",
      CTRL_LOGOFF_EVENT => "logoff",
      CTRL_SHUTDOWN_EVENT => "shutdown",
      _ => return 0,
    };
    let (answer, told) = oneshot::channel();
    if ending().send(Ending::Event(name, answer)).is_err() || told.blocking_recv() != Ok(true) {
      return 0;
    }
    loop {
      std::thread::park();
    }
  }
}
