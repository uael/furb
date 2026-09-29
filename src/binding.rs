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
  collections::HashMap,
  mem::ManuallyDrop,
  sync::{Arc, Mutex},
  thread::{self, ThreadId},
};

#[cfg(feature = "typescript")]
use napi_derive::napi;
#[cfg(feature = "python")]
use pyo3::pyfunction;

use crate::{
  Ear, Fact, Fault, Object, ear,
  extension::{self, Extension},
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

/// What one thread owns: an ear of rust holds what the thread that made it may touch alone. A host may drop the
/// object that holds it on any thread, as python collects a cycle there, so a drop on another thread frees nothing,
/// which leaves memory alone once the ear is disposed.
struct Owned<T> {
  thread: ThreadId,
  held: ManuallyDrop<T>,
}

// SAFETY: what an Owned holds is touched on the thread that made it alone: every reach of it refuses another thread,
// and a drop on another thread leaves it as it is.
unsafe impl<T> Send for Owned<T> {}
unsafe impl<T> Sync for Owned<T> {}

impl<T> Owned<T> {
  fn new(held: T) -> Self {
    Owned { thread: thread::current().id(), held: ManuallyDrop::new(held) }
  }

  fn get_mut(&mut self) -> Result<&mut T, Fault> {
    if thread::current().id() != self.thread {
      return Err(Fault::refused("an ear of the crate is heard on the thread that made it"));
    }
    Ok(&mut self.held)
  }
}

impl<T> Drop for Owned<T> {
  fn drop(&mut self) {
    if thread::current().id() == self.thread {
      // SAFETY: the value is dropped once, here, on the thread that owns it.
      unsafe { ManuallyDrop::drop(&mut self.held) }
    }
  }
}

/// An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear. It is let go
/// when it is disposed, so what it holds goes: a command ends, a wait ends, and a store lets its record go. The
/// engine of python steps it as a generator of its own too.
#[cfg_attr(feature = "python", pyo3::pyclass(module = "furb_monty._monty", weakref))]
#[cfg_attr(feature = "typescript", napi)]
pub struct NativeEar {
  hearing: Owned<Hearing>,
}

/// The ear, and what the engine of python steps it with once it is born there.
struct Hearing {
  ear: Option<Box<dyn Ear>>,
  #[cfg(feature = "python")]
  stepped: Option<py::Stepped>,
}

impl NativeEar {
  fn of(ear: Box<dyn Ear>) -> NativeEar {
    let hearing = Hearing {
      ear: Some(ear),
      #[cfg(feature = "python")]
      stepped: None,
    };
    NativeEar { hearing: Owned::new(hearing) }
  }

  /// The ear, taken out, since an ear hears in one engine.
  fn taken(&mut self) -> Result<Box<dyn Ear>, Fault> {
    let hearing = self.hearing.get_mut()?;
    #[cfg(feature = "python")]
    if hearing.stepped.is_some() {
      return Err(Fault::refused("an ear of the crate hears in one engine, and this one hears"));
    }
    hearing
      .ear
      .take()
      .ok_or_else(|| Fault::refused("an ear of the crate hears in one engine, and this one hears"))
  }
}

#[cfg_attr(feature = "python", pyo3::pymethods)]
#[cfg_attr(feature = "typescript", napi)]
impl NativeEar {
  /// The ear is let go before any engine hears it, or after the life it heard in, so what it holds goes: a command
  /// ends, a wait ends, and a store lets its record go.
  #[cfg_attr(feature = "typescript", napi)]
  pub fn dispose(&mut self) -> Result<(), Fault> {
    self.hearing.get_mut()?.ear.take();
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

/// An image a host attached: the name of its file, the uri a markdown names it by, its media type, and its size in
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

/// The ear of the files, which reads and writes a path.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn files() -> NativeEar {
  NativeEar::of(world::files())
}

/// The ear of commands, which runs each in a shell of this machine.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn bash() -> NativeEar {
  NativeEar::of(world::bash())
}

/// The ear of time, which reads the clock, draws a chance, and ends a wait.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn time() -> NativeEar {
  NativeEar::of(world::time())
}

/// The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says to
/// keep.
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
pub fn shell() -> &'static str {
  world::SHELL
}

/// Every shape the operator answers, by its name.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn shapes() -> Vec<&'static str> {
  world::SHAPES.to_vec()
}

/// A line of the operator as a value of the shape a thread wants, by the rules every console of the crate reads a
/// line by, as the record keeps it; or the refusal of a line that is no value of the shape, and of a shape the
/// operator answers not.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi(ts_return_type = "unknown"))]
pub fn answered(shape: String, line: String) -> Result<Record, Fault> {
  world::answered(&shape, &line).map(Record)
}

/// The models the catalog of this machine offers, with the claude command line at a path when it is given, each
/// named as the catalog names it.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (claude = None)))]
#[cfg_attr(feature = "typescript", napi)]
pub fn models(claude: Option<String>) -> Vec<ModelInfo> {
  let catalog = world::Catalog::load().claude(claude.map(Into::into), None);
  catalog.offered().into_iter().map(ModelInfo::of).collect()
}

/// The model the catalog knows by a name, as `provider:id` or as an id that one model alone holds, whether it offers
/// that model or not; nothing when it knows none.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn model(name: String) -> Option<ModelInfo> {
  world::Catalog::load().find(&name).map(ModelInfo::of)
}

/// The levels of effort, from least to most, which an actor names after its model.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn levels() -> Vec<&'static str> {
  world::catalog::LEVELS.to_vec()
}

/// An image copied into a directory of images under the digest of its bytes, as the markdown of a thread
/// attaches it.
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

/// How a markdown names an image: `![name](uri)`.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_reference(name: String, uri: String) -> String {
  images::reference(&name, &uri)
}

/// Each image a markdown names, as its text in the markdown, its name, and its uri.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_references(markdown: String) -> Vec<images::Named> {
  images::references(&markdown)
}

/// The type of an image by its first bytes: its media type and its extension, for a PNG, a JPEG, a GIF or a WebP.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn image_type(data: &[u8]) -> Result<ImageType, Fault> {
  let (media, extension) = images::kind(data).map_err(Fault::refused)?;
  Ok(ImageType { mime_type: media.to_owned(), extension: extension.to_owned() })
}

/// Whether a life of this thread hears an ear or a function of the host now, so that a verb said now is said by
/// what it hears.
#[cfg_attr(feature = "python", pyfunction)]
#[cfg_attr(feature = "typescript", napi)]
pub fn hearing() -> bool {
  ear::hearing()
}

/// One verb of the engine, called by its name with its words by the ear or the function of the host that a life
/// hears now, and what it gave. Who speaks is the verb `spoken`, and a callable the engine made is the verb `made`,
/// with its number and its words.
#[cfg_attr(feature = "python", pyfunction, pyo3(signature = (verb, args = None, kwargs = None)))]
#[cfg_attr(
  feature = "typescript",
  napi(
    ts_args_type = "verb: string, args?: unknown[], kwargs?: Record<string, unknown>",
    ts_return_type = "unknown"
  )
)]
pub fn call(
  verb: String,
  args: Option<Vec<Value>>,
  kwargs: Option<HashMap<String, Value>>,
) -> Result<Value, Fault> {
  let (args, kwargs) = words(args, &kwargs);
  ear::call(&verb, args, kwargs).map(Value)
}

/// The words of a call that a host gave, by position and by name, as the engine takes them.
fn words(
  args: Option<Vec<Value>>,
  kwargs: &Option<HashMap<String, Value>>,
) -> (Vec<Object>, Vec<(&str, Object)>) {
  let args = args.unwrap_or_default().into_iter().map(|one| one.0).collect();
  (args, kwargs.iter().flatten().map(|(key, one)| (key.as_str(), one.0.clone())).collect())
}

/// The callback of the host that hears the end of the console, which a later callback replaces.
static ENDING: Mutex<Option<Told>> = Mutex::new(None);

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
  *ENDING.lock().map_err(|_| Fault::refused("the console callback is poisoned"))? = Some(callback);
  #[cfg(windows)]
  console::listen()?;
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

  use super::ENDING;
  use crate::Fault;

  /// The handler of this module, added to the handlers of the process, once.
  pub fn listen() -> Result<(), Fault> {
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
    let told = ENDING.lock().ok().and_then(|slot| slot.as_ref().map(|one| one.0.clone()));
    if !told.is_some_and(|told| told(name)) {
      return 0;
    }
    loop {
      std::thread::park();
    }
  }
}
