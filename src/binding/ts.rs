//! The door to TypeScript, through N-API, which also makes the declarations of the package.
//!
//! An engine is one [`JsEngine`], whose verbs are the verbs of the contract, made from the contract when the crate is
//! built, as the methods of the crate are. Views, queries and controls give their value at once, and an act is an
//! [`JsAct`], a name that JavaScript awaits. The ears of an engine are generators of JavaScript and the ears the
//! crate writes, each a [`NativeEar`], in the order the engine offers them a question.
pub mod console;
mod host;

use std::{cell::RefCell, rc::Rc, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};

use napi::{
  Env, JsValue,
  bindgen_prelude::{
    ClassInstance, FnArgs, FromNapiValue, Function, JavaScriptClassExt, JsObjectValue,
    Object as JsObject, Promise, Unknown,
  },
  threadsafe_function::ThreadsafeFunctionCallMode,
};
use napi_derive::napi;
use serde_json::Value;

use host::{Door, Held, Word, refused};

use crate::{
  Ear, Engine, Fault, Object, wire,
  world::{self, images},
};

#[napi(object)]
pub struct TextValue {
  pub path: String,
  pub content: String,
}

#[napi(object)]
pub struct ExitValue {
  pub code: Option<i64>,
  pub stdout: TextValue,
  pub stderr: TextValue,
}

#[napi(object)]
pub struct Outcome {
  pub done: bool,
  #[napi(ts_type = "unknown")]
  pub value: Value,
}

#[napi(object)]
pub struct Inspection {
  pub name: String,
  pub kind: String,
  pub representation: String,
  #[napi(ts_type = "unknown")]
  pub value: Option<Value>,
}

/// An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear.
#[napi]
pub struct NativeEar {
  ear: RefCell<Option<Box<dyn Ear>>>,
}

impl NativeEar {
  fn of(ear: Box<dyn Ear>) -> NativeEar {
    NativeEar { ear: RefCell::new(Some(ear)) }
  }

  /// The ear an object of JavaScript holds, when it is one, taken out of it, since an ear hears in one engine.
  fn taken(env: &Env, object: &JsObject<'_>) -> Result<Option<Box<dyn Ear>>, Fault> {
    if !NativeEar::instance_of(env, object).map_err(refused)? {
      return Ok(None);
    }
    // SAFETY: the object is an instance of the class, which is what the value is read as.
    let held = unsafe { ClassInstance::<NativeEar>::from_napi_value(env.raw(), object.raw()) }
      .map_err(refused)?;
    let ear = held.ear.borrow_mut().take();
    ear
      .map(Some)
      .ok_or_else(|| Fault::refused("an ear of the crate hears in one engine, and this one hears"))
  }
}

#[napi]
impl NativeEar {
  /// The ear is let go before any engine hears it, so what it holds goes: a store lets its record go.
  #[napi]
  pub fn dispose(&self) {
    self.ear.borrow_mut().take();
  }
}

/// The POSIX shell that runs a command of this machine, which a host runs its own commands in too.
#[napi]
pub fn shell() -> &'static str {
  world::SHELL
}

/// The ear of the files, which reads and writes a path.
#[napi]
pub fn files() -> NativeEar {
  NativeEar::of(world::files())
}

/// The ear of commands, which runs each in a shell of this machine.
#[napi]
pub fn bash() -> NativeEar {
  NativeEar::of(world::bash())
}

/// The ear of time, which reads the clock, draws a chance, and ends a wait.
#[napi]
pub fn time() -> NativeEar {
  NativeEar::of(world::time())
}

/// The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says to
/// keep.
#[napi(ts_return_type = "{ record: unknown[]; ear: NativeEar }")]
pub fn store<'env>(env: &'env Env, path: String) -> napi::Result<JsObject<'env>> {
  let (record, ear) = world::store(&path).map_err(error)?;
  let mut got = JsObject::new(env)?;
  got.set_named_property("record", records(&record))?;
  got.set_named_property("ear", NativeEar::of(ear).into_instance(env)?)?;
  Ok(got)
}

/// What the store kept at a path, read with no lease and changed in nothing.
#[napi(ts_return_type = "unknown[]")]
pub fn kept(path: String) -> napi::Result<Vec<Value>> {
  Ok(records(&world::kept(&path).map_err(error)?))
}

/// Entries of a record, as JavaScript reads them.
fn records(record: &[Object]) -> Vec<Value> {
  record.iter().map(|one| wire::record(one.as_ref())).collect()
}

/// One engine, held on the thread of JavaScript.
#[napi(js_name = "Engine")]
pub struct JsEngine {
  held: Rc<Held>,
  root: String,
  raised: Option<Value>,
}

/// An act: its name, which a control takes, and what it comes to, which JavaScript awaits.
#[napi(js_name = "Act")]
pub struct JsAct {
  held: Rc<Held>,
  id: String,
}

impl JsAct {
  /// The name an object of JavaScript holds, when it is an act.
  fn named(env: &Env, object: &JsObject<'_>) -> Result<Option<String>, Fault> {
    if !JsAct::instance_of(env, object).map_err(refused)? {
      return Ok(None);
    }
    // SAFETY: the object is an instance of the class, which is what the value is read as.
    let held = unsafe { ClassInstance::<JsAct>::from_napi_value(env.raw(), object.raw()) }
      .map_err(refused)?;
    Ok(Some(held.id.clone()))
  }
}

#[napi]
impl JsAct {
  #[napi(getter)]
  pub fn id(&self) -> String {
    self.id.clone()
  }

  #[napi(js_name = "toString")]
  pub fn text(&self) -> String {
    self.id.clone()
  }

  #[napi(js_name = "toJSON")]
  pub fn json(&self) -> String {
    self.id.clone()
  }

  #[napi(
    ts_generic_types = "R = unknown, E = never",
    ts_return_type = "Promise<R | E>",
    ts_args_type = "onfulfilled?: ((value: unknown) => R | PromiseLike<R>) | null, onrejected?: ((reason: unknown) => E | PromiseLike<E>) | null"
  )]
  pub fn then<'env>(
    &self,
    env: &'env Env,
    fulfilled: Option<Unknown<'env>>,
    rejected: Option<Unknown<'env>>,
  ) -> napi::Result<Unknown<'env>> {
    let promise = self.held.result(env, &self.id)?;
    type Handlers<'scope> = FnArgs<(Option<Unknown<'scope>>, Option<Unknown<'scope>>)>;
    let then: Function<Handlers<'env>, Unknown<'env>> = promise.get_named_property("then")?;
    then.apply(promise, (fulfilled, rejected).into())
  }
}

#[napi]
impl JsEngine {
  /// An engine, opened from the record, on these ears, each a generator of JavaScript or an ear of the crate under
  /// the name the engine hears it by, in the order the engine offers them a question.
  #[napi(
    factory,
    ts_args_type = "record: unknown[], ears: Array<[string, Generator<unknown, unknown, unknown> | NativeEar]>"
  )]
  pub fn boot(
    env: Env,
    record: Vec<Value>,
    ears: Vec<(String, Unknown<'_>)>,
  ) -> napi::Result<Self> {
    let record = record.iter().map(wire::inward).collect::<Result<Vec<_>, _>>().map_err(error)?;
    let hosted = crate::engine::Hosted::default();
    let door = Door::new(env, hosted.clone())?;
    let mut given = Vec::new();
    for (name, value) in ears {
      let object = value.coerce_to_object()?;
      let ear = match NativeEar::taken(&env, &object).map_err(error)? {
        Some(ear) => ear,
        None => door.ear(object).map_err(error)?,
      };
      given.push((name, ear));
    }
    let engine = Engine::open(hosted, record, given).map_err(error)?;
    let root = engine.root().to_owned();
    let raised = engine.raised().map(|fault| wire::record(fault.object().as_ref()));
    Ok(Self { held: Held::new(&env, engine, door)?, root, raised })
  }

  #[napi(getter)]
  pub fn root(&self) -> String {
    self.root.clone()
  }

  /// What boot raised, and nothing when it raised nothing. After a drift the life goes on, with nothing kept.
  #[napi(getter, ts_return_type = "{ is: string; args: unknown[] } | null")]
  pub fn raised(&self) -> Option<Value> {
    self.raised.clone()
  }

  #[napi(getter)]
  pub fn disposed(&self) -> bool {
    self.held.engine.try_borrow().is_ok_and(|engine| engine.is_none())
  }

  /// Who speaks in the life, and who speaks from now on when a name is given: the site of the contract, which the work
  /// an ear began sets to the name of that ear before it speaks.
  #[napi]
  pub fn site(&self, value: Option<String>) -> napi::Result<String> {
    self.held.call(|engine| engine.site(value.as_deref()))
  }

  /// What an act comes to, which JavaScript awaits.
  #[napi(ts_generic_types = "T = unknown", ts_return_type = "Promise<T>")]
  pub fn result<'env>(&self, env: &'env Env, id: String) -> napi::Result<JsObject<'env>> {
    self.held.result(env, &id)
  }

  /// What an act came to, and whether it is done.
  #[napi]
  pub fn outcome(&self, id: String) -> napi::Result<Outcome> {
    self.held.call(move |engine| {
      let got = engine.outcome(&id)?;
      Ok(Outcome {
        done: got.is_some(),
        value: got.map_or(Value::Null, |value| wire::outward(value.as_ref())),
      })
    })
  }

  /// One name of a chain, read without calling it, with its type and its representation in the sandbox. The value
  /// crosses as every value does, so a map that holds the key `is` crosses as its pairs.
  #[napi]
  pub fn inspect(&self, name: String, chain: Option<String>) -> napi::Result<Inspection> {
    self.held.call(move |engine| {
      let chain = Object::string(chain.unwrap_or_else(|| engine.root().into()));
      let key = Object::string(&name);
      let bound = vec![("__chain", chain), ("__name", key)];
      let raw = engine.word("module(__chain)[__name]", bound.clone())?;
      let value = engine.word("outward(module(__chain)[__name], __engine)", bound)?;
      Ok(Inspection {
        kind: raw.as_ref().type_name().into(),
        representation: raw.as_ref().py_repr(),
        value: Some(wire::outward(value.as_ref())).filter(|value| !value.is_null()),
        name,
      })
    })
  }

  /// Every name the module of a chain binds, in the order it bound them.
  #[napi]
  pub fn names(&self, chain: Option<String>) -> napi::Result<Vec<String>> {
    let value = self.held.call(move |engine| {
      let chain = Object::string(chain.unwrap_or_else(|| engine.root().into()));
      Ok(wire::outward(
        engine.word("[str(x) for x in module(__chain)]", vec![("__chain", chain)])?.as_ref(),
      ))
    })?;
    serde_json::from_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
  }

  /// The engine is gone, and every result JavaScript awaits of it is refused. Its ears go with it: a command of the
  /// crate ends, a wait ends, and the store lets its record go.
  #[napi]
  pub fn dispose(&self) -> napi::Result<()> {
    self.held.dispose()
  }
}

impl JsEngine {
  /// One verb said with the words JavaScript gave, and what it gave, as JavaScript reads it.
  fn plain<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<Unknown<'env>> {
    let got = self.said(env, name, given, rest, options, keys)?;
    self.held.door.outward(env, &got).map_err(error)
  }

  /// One verb that makes an act, said with the words JavaScript gave, and the act.
  fn acted<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<JsAct> {
    let got = self.said(env, name, given, rest, options, keys)?;
    let id = got.as_ref().as_str().ok_or_else(|| napi::Error::from_reason("a verb gave no act"))?;
    Ok(JsAct { held: Rc::clone(&self.held), id: id.to_owned() })
  }

  fn said<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<Object> {
    let door = self.held.door.clone();
    let mut args = Vec::new();
    for (one, word) in given {
      args.push(door.inward(env, one, word, 0).map_err(error)?);
    }
    for one in rest.unwrap_or_default() {
      args.push(door.inward(env, one, Word::Plain, 0).map_err(error)?);
    }
    let kwargs = match options {
      Some(options) => door.named(env, options, keys).map_err(error)?,
      None => Vec::new(),
    };
    self.held.call(|engine| {
      let kwargs = kwargs.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
      engine.verb(name, args, kwargs)
    })
  }
}

// The verbs of the contract, one method each, which the build makes from the contract.
include!(concat!(env!("OUT_DIR"), "/ts.rs"));

fn error(fault: Fault) -> napi::Error {
  napi::Error::from_reason(fault.to_string())
}

#[napi]
pub fn engine_source() -> &'static str {
  crate::ENGINE
}

#[napi(ts_return_type = "unknown")]
pub fn decode_record(line: String) -> napi::Result<Value> {
  let value: &serde_json::value::RawValue = serde_json::from_str(&line)
    .map_err(|error| napi::Error::new(napi::Status::InvalidArg, error.to_string()))?;
  wire::decoded(value, 0).map_err(error)
}

/// What the provider of the crate stands on and asks.
#[napi(object, object_to_js = false)]
pub struct ProviderOptions<'env> {
  /// The directory the life stands on, which each chain stands in until it goes elsewhere.
  pub directory: String,
  /// The models it offers, each named `provider:id`, or by an id that one model of the catalog alone holds; every
  /// model that the catalog offers when unsaid.
  pub roster: Option<Vec<String>>,
  /// The actor a prompt goes to when it names none, as model/effort, whose effort moves to the nearest one the model
  /// takes; the first model at its first effort when unsaid.
  pub actor: Option<String>,
  /// The path of the claude command line to run, in place of the one that `FURB_CLAUDE_BIN` names or this machine
  /// holds.
  pub claude: Option<String>,
  /// How many milliseconds a turn of claude may go with no progress.
  pub stall_ms: Option<f64>,
  /// The directory of the images that a turn names.
  pub images: Option<String>,
  /// A function that answers each request in place of the model, with a turn, and may tell what it writes as it
  /// writes it.
  #[napi(
    ts_type = "(request: { actor: string; chain: string; messages: unknown[]; settings: Record<string, unknown> }, write: (delta: { text?: string; thinking?: string }) => void) => Promise<unknown>"
  )]
  pub answer: Option<Function<'env, (), Promise<Value>>>,
  /// Told what a model writes as it writes it: the rung it writes for, the chain of that rung, and what it added to
  /// its text and to its thought.
  #[napi(ts_type = "(rung: string, chain: string, text: string, thinking: string) => void")]
  pub stream: Option<Function<'env, (), ()>>,
}

/// The ear of the provider of models, which answers a stand and takes each reply: the catalog makes the models of
/// its roster, and a function of the host answers them in place of the models when it gives one.
#[napi]
pub fn provider(options: ProviderOptions<'_>) -> napi::Result<NativeEar> {
  let mut catalog = world::Catalog::load();
  if let Some(bin) = options.claude {
    let stall =
      options.stall_ms.and_then(|ms| std::time::Duration::try_from_secs_f64(ms / 1e3).ok());
    catalog = catalog.with_claude(world::claude::Claude::with(Some(bin.into()), stall));
  }
  let (models, actor) = catalog
    .roster(options.roster.as_deref(), options.actor.as_deref())
    .map_err(napi::Error::from_reason)?;
  let models = match options.answer {
    Some(answer) => {
      let host = hosted(&answer)?;
      models.into_iter().map(|model| model.hosted(host.clone())).collect()
    }
    None => models,
  };
  let mut made = world::Provider::new(options.directory, models).actor(actor);
  if let Some(images) = options.images {
    made = made.images(images);
  }
  if let Some(stream) = options.stream {
    let told = stream
      .build_threadsafe_function::<(String, String, String, String)>()
      .weak::<true>()
      .build_callback(|call| Ok(FnArgs::from(call.value)))?;
    made = made.writes(Arc::new(move |rung, chain, text, thinking| {
      let said = (rung.to_owned(), chain.to_owned(), text.to_owned(), thinking.to_owned());
      told.call(said, ThreadsafeFunctionCallMode::NonBlocking);
    }));
  }
  Ok(NativeEar::of(made.ear()))
}

/// A function of JavaScript as a model: it is called on the thread of JavaScript with the request and a function
/// that tells what it writes, and the turn its promise gives is the answer, or what it threw is the refusal.
fn hosted(answer: &Function<'_, (), Promise<Value>>) -> napi::Result<world::Hosted> {
  /// What a call tells as it writes, as one type, which a future of the call carries to the thread of JavaScript.
  struct Writing(world::Told);
  let called = answer
    .build_threadsafe_function::<(Value, Writing)>()
    .weak::<true>()
    .build_callback(|call| {
      let (request, Writing(told)) = call.value;
      let request = call.env.to_js_value(&request)?;
      let write: Function<'_, Value, ()> =
        call.env.create_function_from_closure("write", move |cx| {
          let delta: Value = cx.first_arg()?;
          let part =
            |key: &str| delta.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
          told(&part("text"), &part("thinking"));
          Ok(())
        })?;
      // The values go as they are on the thread of JavaScript, where the call takes them at once.
      Ok(FnArgs::from((request.raw(), write.raw())))
    })?;
  let called = Arc::new(called);
  Ok(Arc::new(move |request, told| {
    let called = Arc::clone(&called);
    let writing = Writing(told);
    Box::pin(async move {
      let promise =
        called.call_async_catch((request, writing)).await.map_err(|no| no.reason.clone())?;
      let turn = promise.await.map_err(|no| no.reason.clone())?;
      wire::inward(&turn).map_err(|fault| fault.message())
    })
  }))
}

/// A model of the catalog as JavaScript reads it: its name, its efforts, its window, whether it takes an image, and
/// its price in dollars for a million tokens read, written, read from the cache and written to it.
#[napi(object)]
pub struct ModelInfo {
  pub name: String,
  pub efforts: Vec<String>,
  pub window: f64,
  pub images: bool,
  pub price: Option<Vec<f64>>,
}

/// The models the catalog of this machine offers, or the ones it names when names are given, each named as the
/// catalog names it; a name it knows no model by is refused.
#[napi]
pub fn models(names: Option<Vec<String>>) -> napi::Result<Vec<ModelInfo>> {
  let catalog = world::Catalog::load();
  let found = match names {
    Some(names) => names
      .iter()
      .map(|name| {
        catalog.find(name).ok_or_else(|| napi::Error::from_reason(format!("No model is {name}.")))
      })
      .collect::<napi::Result<Vec<_>>>()?,
    None => catalog.offered(),
  };
  let info = |model: &world::Model| ModelInfo {
    name: model.name().to_owned(),
    efforts: model.efforts().into_iter().map(str::to_owned).collect(),
    window: model.window() as f64,
    images: model.sees(),
    price: model.priced().map(Vec::from),
  };
  Ok(found.into_iter().map(info).collect())
}

/// The levels of effort, from least to most, which an actor names after its model.
#[napi]
pub fn efforts() -> Vec<&'static str> {
  world::catalog::LEVELS.to_vec()
}

/// An image a host attached: the name of its file, the uri a message names it by, its media type, and its size.
#[napi(object)]
pub struct ImageAttachment {
  pub name: String,
  pub uri: String,
  pub mime_type: String,
  pub size: f64,
}

/// An image copied into a directory of images under the digest of its bytes, as a message attaches it.
#[napi]
pub fn attach_image(directory: String, path: String) -> napi::Result<ImageAttachment> {
  let got = images::attach(directory.as_ref(), path.as_ref()).map_err(napi::Error::from_reason)?;
  Ok(ImageAttachment {
    name: got.name,
    uri: got.uri,
    mime_type: got.media.to_owned(),
    size: got.size as f64,
  })
}

/// The bytes of the image of a uri, as base64, and its media type, once the bytes have the digest the uri names.
#[napi(ts_return_type = "{ data: string; mimeType: string }")]
pub fn image_content(env: &Env, directory: String, uri: String) -> napi::Result<JsObject<'_>> {
  let (media, bytes) = images::read(directory.as_ref(), &uri).map_err(napi::Error::from_reason)?;
  let mut got = JsObject::new(env)?;
  got.set_named_property("data", STANDARD.encode(bytes))?;
  got.set_named_property("mimeType", media)?;
  Ok(got)
}

/// The file that holds the image of a uri, and the digest its bytes have.
#[napi(ts_return_type = "{ path: string; digest: string }")]
pub fn image_path(env: &Env, directory: String, uri: String) -> napi::Result<JsObject<'_>> {
  let (path, digest) = images::path(directory.as_ref(), &uri).map_err(napi::Error::from_reason)?;
  let mut got = JsObject::new(env)?;
  got.set_named_property("path", path.display().to_string())?;
  got.set_named_property("digest", digest)?;
  Ok(got)
}

/// How a message names an image: `![name](uri)`.
#[napi(ts_args_type = "image: { name: string; uri: string }")]
pub fn image_reference(image: JsObject<'_>) -> napi::Result<String> {
  let (name, uri): (String, String) =
    (image.get_named_property("name")?, image.get_named_property("uri")?);
  Ok(images::reference(&name, &uri))
}

/// Each image a message names, as its text in the message, its name, and its uri.
#[napi(ts_return_type = "{ text: string; name: string; uri: string }[]")]
pub fn image_references(message: String) -> Vec<Value> {
  let each = images::references(&message).into_iter();
  each.map(|one| serde_json::json!({"text": one.text, "name": one.name, "uri": one.uri})).collect()
}

/// The type of an image by its first bytes: its media type and its extension, for a PNG, a JPEG, a GIF or a WebP.
#[napi(ts_return_type = "{ mimeType: string; extension: string }")]
pub fn image_type(data: &[u8]) -> napi::Result<Value> {
  let (media, extension) = images::kind(data).map_err(napi::Error::from_reason)?;
  Ok(serde_json::json!({"mimeType": media, "extension": extension}))
}
