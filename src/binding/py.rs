//! The door to python, behind the `python` feature: a [`PyEngine`] whose verbs are those of the crate, and the ears
//! the crate writes, each a [`NativeEar`]. A value of the engine comes out as the instance of `furb.python` it is,
//! an exception as the one object it is for the life, a generator of python goes in as an ear, and a function as one
//! the sandbox calls back.

use std::{
  rc::Rc,
  sync::Arc,
  task::{Wake, Waker},
};

use pyo3::{
  Bound, Py, PyAny, PyResult, Python,
  exceptions::{PyBaseException, PyStopIteration, PyTypeError},
  prelude::*,
  types::{
    PyBool, PyDict, PyFloat, PyFunction, PyInt, PyList, PyModule, PyString, PyTuple, PyType,
  },
};

use crate::{
  Ear, Engine, Fault, Heard, Object, ObjectRef, Step,
  engine::Hosted,
  value::{IS, entry, field, marked},
  world,
};

/// What makes a value of the engine the python object it is: the engine of this interpreter, whose classes an
/// instance is made from, and the one object each exception of the life is, by its name and what it was made
/// with. Both are python's own, so a handle is cheap to hold twice.
struct Made {
  /// The engine of this interpreter, `furb.python`, which the package binds whatever the switch says.
  python: Py<PyAny>,
  faults: Py<PyDict>,
}

impl Made {
  fn new(py: Python<'_>) -> PyResult<Self> {
    Ok(Made {
      python: py.import("furb")?.getattr("python")?.unbind(),
      faults: PyDict::new(py).unbind(),
    })
  }

  /// A name of the engine, as python holds it: the engine of monty's own binding of it, so that a verb of the
  /// engine that leaves the sandbox is the verb a python host says, and a class is the class its instances are.
  fn named<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    let python = self.python.bind(py);
    if !python.hasattr(name)? {
      return Ok(None);
    }
    let engine = py.import("furb_monty.engine")?;
    if engine.hasattr(name)? {
      engine.getattr(name).map(Some)
    } else {
      python.getattr(name).map(Some)
    }
  }

  /// The name a callable of python is bound to in the engine, when it is one of its names: a show, a verb, a
  /// class, by identity, since a show of the engine is a function made by another and carries no name of its own.
  fn name_of(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
    if !value.is_callable() {
      return Ok(None);
    }
    let engine = py.import("furb_monty.engine")?;
    for (name, held) in self.python.bind(py).getattr("__dict__")?.cast::<PyDict>()?.iter() {
      let name = name.extract::<String>()?;
      if name.starts_with('_') {
        continue;
      }
      if held.is(value)
        || (engine.hasattr(name.as_str())? && engine.getattr(name.as_str())?.is(value))
      {
        return Ok(Some(name));
      }
    }
    Ok(None)
  }

  /// The class of this name: the engine's, or the interpreter's.
  fn class<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    match self.python.bind(py).getattr(name) {
      Ok(held) => Ok(held),
      Err(_) => py
        .import("builtins")?
        .getattr(name)
        .map_err(|_| PyTypeError::new_err(format!("{name} is no class this interpreter knows"))),
    }
  }

  /// An exception, one object for as long as the life lives: the object this name and these arguments were
  /// made into the first time, every time.
  fn fault<'py>(
    &self,
    py: Python<'py>,
    name: &str,
    args: Vec<Bound<'py, PyAny>>,
  ) -> PyResult<Bound<'py, PyAny>> {
    let key = format!(
      "{name}{}",
      args
        .iter()
        .map(|one| one.repr().map(|r| r.to_string()).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(", ")
    );
    let faults = self.faults.bind(py);
    if let Some(held) = faults.get_item(&key)? {
      return Ok(held);
    }
    let made = self.class(py, name)?.call1(PyTuple::new(py, args)?)?;
    faults.set_item(&key, &made)?;
    Ok(made)
  }
}

/// The door of python: what makes a value the object it is here, and the ears and the functions of the host, which
/// a value of python adds to as it goes in.
#[derive(Clone)]
struct Door(Rc<Doorway>);

struct Doorway {
  made: Made,
  hosted: Hosted,
}

impl Door {
  fn made(&self) -> &Made {
    &self.0.made
  }

  /// An ear of python given to boot: an ear of the crate as itself, and a generator heard as an ear.
  fn ear(&self, value: &Bound<'_, PyAny>) -> PyResult<Box<dyn Ear>> {
    if let Some(ear) = NativeEar::taken(value)? {
      return Ok(ear);
    }
    if !generator(value)? {
      let kind = value.get_type().name()?;
      return Err(PyTypeError::new_err(format!("{kind} is no ear: an ear is a generator")));
    }
    Ok(Box::new(PyEar { door: self.clone(), ear: Some(value.clone().unbind()) }))
  }

  /// An ear of python given to a verb, as the value the verb is given: the generator that stands in for it, which
  /// stands at a yield when python started the generator before it crossed.
  fn crossed(&self, value: &Bound<'_, PyAny>) -> PyResult<Object> {
    let started = generator(value)?
      && value
        .py()
        .import("inspect")?
        .call_method1("getgeneratorstate", (value,))?
        .extract::<String>()?
        != "GEN_CREATED";
    Ok(self.0.hosted.ear(self.ear(value)?, started))
  }
}

/// Whether a value of python is a generator, which is what an ear of python is.
fn generator(value: &Bound<'_, PyAny>) -> PyResult<bool> {
  value.py().import("inspect")?.call_method1("isgenerator", (value,))?.is_truthy()
}

/// A generator of python, heard as an ear: sent what it hears, and thrown in what a verb it said raised.
///
/// It yields a saying, which is a tuple, a call of a verb, which is a map of the verb and its words, or nothing, and
/// it hears nothing at its birth.
struct PyEar {
  door: Door,
  ear: Option<Py<PyAny>>,
}

impl Ear for PyEar {
  fn resume(&mut self, heard: Heard) -> Step {
    Python::attach(|py| {
      self.step(py, heard).unwrap_or_else(|no| Step::Raised(fault_of(&self.door, py, &no)))
    })
  }
}

impl PyEar {
  fn step(&mut self, py: Python<'_>, heard: Heard) -> PyResult<Step> {
    let made = self.door.made();
    let ear = self.ear.as_ref().ok_or_else(|| PyTypeError::new_err("the ear is over"))?.bind(py);
    let got = match heard {
      Heard::Born(_) => ear.call_method1("send", (py.None(),)),
      Heard::Fact(fact) => ear.call_method1("send", (to_python(py, made, fact.0.as_ref())?,)),
      Heard::Value(value) => ear.call_method1("send", (to_python(py, made, value.as_ref())?,)),
      Heard::Raised(fault) => ear.call_method1("throw", (fault_to_python(py, made, &fault)?,)),
    };
    let got = match got {
      Ok(got) => got,
      Err(no) if no.is_instance_of::<PyStopIteration>(py) => {
        self.ear.take();
        return Ok(Step::Over);
      }
      Err(no) => return Ok(Step::Raised(fault_of(&self.door, py, &no))),
    };
    Ok(Step::of(&of_python(&self.door, &got)?).unwrap_or_else(Step::Raised))
  }
}

/// An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear.
#[pyclass(module = "furb_monty._monty", name = "NativeEar", unsendable)]
pub struct NativeEar {
  ear: Option<Box<dyn Ear>>,
}

impl NativeEar {
  fn of(ear: Box<dyn Ear>) -> NativeEar {
    NativeEar { ear: Some(ear) }
  }

  /// The ear a value of python holds, when it is one, taken out of it, since an ear hears in one engine.
  fn taken(value: &Bound<'_, PyAny>) -> PyResult<Option<Box<dyn Ear>>> {
    let Ok(held) = value.cast::<NativeEar>() else { return Ok(None) };
    let ear = held.borrow_mut().ear.take();
    ear.map(Some).ok_or_else(|| {
      PyTypeError::new_err("an ear of the crate hears in one engine, and this one hears")
    })
  }
}

#[pymethods]
impl NativeEar {
  /// The ear is let go before any engine hears it, so what it holds goes: a store lets its record go.
  fn dispose(&mut self) {
    self.ear.take();
  }
}

/// What wakes the loop of python to drive the engine, when a voice spoke from another thread.
struct Wakes {
  /// The loop the engine was booted in.
  running: Py<PyAny>,
  /// A weak reference to the engine, which a voice keeps nothing alive by.
  engine: Py<PyAny>,
}

impl Wake for Wakes {
  fn wake(self: Arc<Self>) {
    Python::attach(|py| {
      let Ok(engine) = self.engine.bind(py).call0() else { return };
      if engine.is_none() {
        return;
      }
      if let Ok(pump) = engine.getattr("pump") {
        // A loop that is closed drives nothing more, and the engine is gone with it.
        let _ = self.running.bind(py).call_method1("call_soon_threadsafe", (pump,));
      }
    });
  }
}

/// One engine, held on the thread of python.
#[pyclass(module = "furb_monty._monty", name = "Engine", unsendable, weakref)]
pub struct PyEngine {
  engine: Option<Engine>,
  door: Door,
  root: String,
  raised: Option<Fault>,
  waker: Option<Waker>,
}

impl PyEngine {
  fn held(&mut self, py: Python<'_>) -> PyResult<&mut Engine> {
    let made = &self.door.0.made;
    self.engine.as_mut().ok_or_else(|| raised(py, made, &Fault::refused("the engine is disposed")))
  }

  /// One call of the engine, and what the engine raised, raised here as the exception it is.
  fn call<T>(
    &mut self,
    py: Python<'_>,
    call: impl FnOnce(&mut Engine) -> Result<T, Fault>,
  ) -> PyResult<T> {
    let got = call(self.held(py)?);
    got.map_err(|fault| raised(py, self.door.made(), &fault))
  }

  /// One call of the engine with the words python gave, and what it gave, as python holds it.
  fn answered<'py>(
    &mut self,
    py: Python<'py>,
    args: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
    call: impl FnOnce(&mut Engine, Vec<Object>, Vec<(&str, Object)>) -> Result<Object, Fault>,
  ) -> PyResult<Bound<'py, PyAny>> {
    let args =
      args.try_iter()?.map(|one| of_python(&self.door, &one?)).collect::<PyResult<Vec<_>>>()?;
    let mut named = Vec::new();
    for (key, value) in kwargs.iter() {
      named.push((key.extract::<String>()?, of_python(&self.door, &value)?));
    }
    let named = named.iter().map(|(key, value)| (key.as_str(), value.clone())).collect();
    let got = self.call(py, |engine| call(engine, args, named))?;
    to_python(py, self.door.made(), got.as_ref())
  }

  /// One verb said with the words python gave, and what it gave, as python holds it.
  fn said<'py>(
    &mut self,
    py: Python<'py>,
    name: &str,
    given: Vec<Bound<'py, PyAny>>,
    rest: Option<Bound<'py, PyTuple>>,
    named: Vec<(&str, Option<Bound<'py, PyAny>>)>,
  ) -> PyResult<Bound<'py, PyAny>> {
    let mut args =
      given.iter().map(|one| of_python(&self.door, one)).collect::<PyResult<Vec<_>>>()?;
    for one in rest.iter().flat_map(|held| held.iter()) {
      args.push(of_python(&self.door, &one)?);
    }
    let mut kwargs = Vec::new();
    for (key, one) in named {
      if let Some(one) = one {
        kwargs.push((key, of_python(&self.door, &one)?));
      }
    }
    let got = self.call(py, |engine| engine.verb(name, args, kwargs))?;
    to_python(py, self.door.made(), got.as_ref())
  }
}

#[pymethods]
impl PyEngine {
  /// An engine, opened from the record, on these ears, each a generator of python or an ear of the crate under the
  /// name the engine hears it by, in the order the engine offers them a question. It is booted in the running loop,
  /// which drives it when an ear of the crate speaks from a thread of its own.
  #[staticmethod]
  fn boot(
    py: Python<'_>,
    record: Bound<'_, PyAny>,
    ears: Bound<'_, PyAny>,
  ) -> PyResult<Py<PyEngine>> {
    let running = py.import("asyncio")?.call_method0("get_running_loop")?;
    let hosted = Hosted::default();
    let door = Door(Rc::new(Doorway { made: Made::new(py)?, hosted: hosted.clone() }));
    let kept = of_python(&door, &record)?;
    let kept = kept.as_ref().items().unwrap_or_default().into_iter().map(|one| one.to_owned());
    let mut given = Vec::new();
    for pair in ears.try_iter()? {
      let (name, ear): (String, Bound<'_, PyAny>) = pair?.extract()?;
      given.push((name, door.ear(&ear)?));
    }
    let engine = Engine::open(hosted, kept.collect::<Vec<_>>(), given)
      .map_err(|fault| raised(py, door.made(), &fault))?;
    let root = engine.root().to_owned();
    let fault = engine.raised().cloned();
    let held =
      Py::new(py, PyEngine { engine: Some(engine), door, root, raised: fault, waker: None })?;
    let weak = py.import("weakref")?.getattr("ref")?.call1((held.bind(py),))?;
    let wakes = Wakes { running: running.unbind(), engine: weak.unbind() };
    held.borrow_mut(py).waker = Some(Waker::from(Arc::new(wakes)));
    held.borrow_mut(py).pump(py)?;
    Ok(held)
  }

  /// The root chain of the life, which is the first act of any record.
  #[getter]
  fn root(&self) -> &str {
    &self.root
  }

  /// What boot raised, as the exception it is, and nothing when it raised nothing. After a drift the life goes on,
  /// with nothing kept.
  #[getter]
  fn raised<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    match &self.raised {
      Some(one) => fault_to_python(py, self.door.made(), one),
      None => Ok(py.None().into_bound(py)),
    }
  }

  /// Who speaks in the life, and who speaks from now on when a name is given: the site of the contract, which the
  /// work an ear began sets to the name of that ear before it speaks.
  #[pyo3(signature = (value=None))]
  fn site(&mut self, py: Python<'_>, value: Option<&str>) -> PyResult<String> {
    self.call(py, |engine| engine.site(value))
  }

  /// One name of the engine that is no verb, said by its name with these words, and what it gave.
  fn verb<'py>(
    &mut self,
    py: Python<'py>,
    name: &str,
    args: Bound<'py, PyAny>,
    kwargs: Bound<'py, PyDict>,
  ) -> PyResult<Bound<'py, PyAny>> {
    self.answered(py, &args, &kwargs, |engine, args, named| engine.verb(name, args, named))
  }

  /// One callable the engine made, called back by the handle it crossed under, with these words.
  fn made<'py>(
    &mut self,
    py: Python<'py>,
    n: i64,
    args: Bound<'py, PyAny>,
    kwargs: Bound<'py, PyDict>,
  ) -> PyResult<Bound<'py, PyAny>> {
    self.answered(py, &args, &kwargs, |engine, args, named| engine.made(n, args, named))
  }

  /// A callable the engine made, forgotten: python holds its handle no more.
  fn forget(&mut self, py: Python<'_>, n: i64) -> PyResult<()> {
    self.call(py, |engine| engine.forget(n))
  }

  /// What to call when an act is done, with what it came to: at once for one done already, and once otherwise.
  fn watch(&mut self, py: Python<'_>, act: &str, then: Py<PyAny>) -> PyResult<()> {
    let door = self.door.clone();
    self.call(py, |engine| {
      engine.watch(act, move |got| {
        Python::attach(|py| {
          if let Ok(value) = to_python(py, door.made(), got.as_ref()) {
            let _ = then.bind(py).call1((value,));
          }
        });
      })
    })
  }

  /// The engine driven as far as it goes: what the voices of its ears said is said into it, each under the name of
  /// its ear, and whoever awaits an act that is done now is told.
  fn pump(&mut self, py: Python<'_>) -> PyResult<()> {
    let Some(waker) = self.waker.clone() else { return Ok(()) };
    if self.engine.is_none() {
      return Ok(());
    }
    self.call(py, |engine| engine.pump(&waker))
  }

  /// The engine is gone, and its ears with it: a command of the crate ends, a wait ends, and the store lets its
  /// record go.
  fn dispose(&mut self) {
    self.engine.take();
    self.door.0.hosted.clear();
  }
}

// The verbs of the contract, one method each, which the build makes from the contract.
include!(concat!(env!("OUT_DIR"), "/py.rs"));

/// The ear of the files, which reads and writes a path.
#[pyfunction]
fn files() -> NativeEar {
  NativeEar::of(world::files())
}

/// The ear of commands, which runs each in a shell of this machine.
#[pyfunction]
fn bash() -> NativeEar {
  NativeEar::of(world::bash())
}

/// The ear of time, which reads the clock, draws a chance, and ends a wait.
#[pyfunction]
fn time() -> NativeEar {
  NativeEar::of(world::time())
}

/// The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says to
/// keep.
#[pyfunction]
fn store<'py>(py: Python<'py>, path: &str) -> PyResult<(Bound<'py, PyAny>, NativeEar)> {
  let made = Made::new(py)?;
  let (record, ear) = world::store(path).map_err(|fault| raised(py, &made, &fault))?;
  Ok((to_python(py, &made, Object::list(record).as_ref())?, NativeEar::of(ear)))
}

/// What the store kept at a path, read with no lease and changed in nothing.
#[pyfunction]
fn kept<'py>(py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyAny>> {
  let made = Made::new(py)?;
  let record = world::kept(path).map_err(|fault| raised(py, &made, &fault))?;
  to_python(py, &made, Object::list(record).as_ref())
}

/// The gate of the crate, for the Kernel of this interpreter to read a sheet with: what the checker of this thread
/// found on the sheet, each error by its line, and no warning. The checker is the one every engine of monty on the
/// thread gates with, so a word is judged once and the same.
#[pyfunction]
fn gate(py: Python<'_>, sheet: &str) -> PyResult<Vec<(usize, String)>> {
  match crate::gate::checked(sheet) {
    Ok(found) => Ok(found),
    Err(fault) => Err(raised(py, &Made::new(py)?, &fault)),
  }
}

/// What the engine raised, raised here as the exception it is.
fn raised(py: Python<'_>, made: &Made, fault: &Fault) -> PyErr {
  match fault_to_python(py, made, fault) {
    Ok(one) => PyErr::from_value(one),
    Err(no) => no,
  }
}

/// A fault, as the one exception object it is for the life.
fn fault_to_python<'py>(
  py: Python<'py>,
  made: &Made,
  fault: &Fault,
) -> PyResult<Bound<'py, PyAny>> {
  let args =
    fault.args.iter().map(|one| to_python(py, made, one.as_ref())).collect::<PyResult<Vec<_>>>()?;
  made.fault(py, &fault.name, args)
}

/// What python raised, as the crossing reads it: its type and what it was made with, or what it says when its words
/// do not cross.
fn fault_of(door: &Door, py: Python<'_>, no: &PyErr) -> Fault {
  let value = no.value(py);
  let name = value.get_type().name().map_or("Exception".to_owned(), |one| one.to_string());
  let args = value.getattr("args").ok().and_then(|args| {
    args.try_iter().ok()?.map(|one| of_python(door, &one.ok()?).ok()).collect::<Option<Vec<_>>>()
  });
  Fault::new(name, args.unwrap_or_else(|| vec![Object::string(no.to_string())]))
}

/// One value of the sandbox, as python holds it: an instance of a class of the engine as that instance, an
/// exception as the one object it is for the life, and plain data as itself.
fn to_python<'py>(
  py: Python<'py>,
  made: &Made,
  said: ObjectRef<'_>,
) -> PyResult<Bound<'py, PyAny>> {
  if let Some(held) = said.as_bool() {
    return Ok(PyBool::new(py, held).to_owned().into_any());
  }
  if let Some(held) = said.as_int() {
    return Ok(held.into_pyobject(py)?.into_any());
  }
  if let Some(held) = said.as_float() {
    return Ok(PyFloat::new(py, held).into_any());
  }
  if let Some(held) = said.as_str() {
    return Ok(PyString::new(py, held).into_any());
  }
  let each = |held: Vec<ObjectRef<'_>>| {
    held.into_iter().map(|one| to_python(py, made, one)).collect::<PyResult<Vec<_>>>()
  };
  match said.type_name() {
    "NoneType" | "None" => Ok(py.None().into_bound(py)),
    "ellipsis" => Ok(py.Ellipsis().into_bound(py)),
    "list" => Ok(PyList::new(py, each(said.items().unwrap_or_default())?)?.into_any()),
    "tuple" => Ok(PyTuple::new(py, each(said.items().unwrap_or_default())?)?.into_any()),
    "dict" => {
      let pairs = said.pairs().unwrap_or_default();
      // The two marks of a callable that goes out: a name of the engine, and a handle to a callable it made.
      let at = |name: &str| field(&said, name);
      if let Some(mark) = at(IS).and_then(|one| one.as_str()) {
        // A map that holds the key of the mark goes out as its pairs, and is the map it is here.
        if mark == "dict"
          && let Some(held) = at("args").and_then(|one| entry(&one, 0)).and_then(|one| one.items())
        {
          let map = PyDict::new(py);
          for pair in held {
            let (Some(key), Some(one)) = (entry(&pair, 0), entry(&pair, 1)) else {
              return Err(PyTypeError::new_err(
                "a map that goes out as its pairs holds pairs of two",
              ));
            };
            map.set_item(to_python(py, made, key)?, to_python(py, made, one)?)?;
          }
          return Ok(map.into_any());
        }
        if mark == "name"
          && let Some(name) = at("name").and_then(|one| one.as_str())
          && let Some(held) = made.named(py, name)?
        {
          return Ok(held);
        }
        if mark == "made"
          && let Some(n) = at("id").and_then(|one| one.as_int())
        {
          return py.import("furb_monty.engine")?.call_method1("made", (n,));
        }
        // A class a word defined, as the type python holds it by its handle, derived from the type its base is
        // here, and an instance of one, as an object of that type holding the fields the sandbox carries out.
        if mark == "class"
          && let Some(n) = at("id").and_then(|one| one.as_int())
          && let Some(name) = at("name").and_then(|one| one.as_str())
          && let Some(base) = at("base")
        {
          let base = to_python(py, made, base)?;
          return py.import("furb_monty.engine")?.call_method1("classed", (n, name, base));
        }
        if mark == "instance"
          && let Some(class) = at("class")
          && let Some(value) = at("value")
        {
          let class = to_python(py, made, class)?;
          let fields = PyDict::new(py);
          for (key, one) in value.pairs().unwrap_or_default() {
            fields.set_item(to_python(py, made, key)?, to_python(py, made, one)?)?;
          }
          return py.import("furb_monty.engine")?.call_method1("instanced", (class, fields));
        }
      }
      let held = PyDict::new(py);
      for (key, one) in pairs {
        held.set_item(to_python(py, made, key)?, to_python(py, made, one)?)?;
      }
      Ok(held.into_any())
    }
    _ => match Fault::of(said) {
      Some(fault) => {
        made.fault(py, &fault.name, each(fault.args.iter().map(Object::as_ref).collect())?)
      }
      None => match said.pairs() {
        Some(fields) => {
          let kwargs = PyDict::new(py);
          for (key, one) in fields {
            kwargs.set_item(to_python(py, made, key)?, to_python(py, made, one)?)?;
          }
          made.class(py, said.type_name())?.call((), Some(&kwargs))
        }
        // A class of the engine crosses as its name, which python holds as the class its instances are, and a
        // builtin type as the builtin it is; anything else that has no fields, a coroutine, a function that
        // crossed by no mark, shows as what it is.
        None => {
          // The interpreter shows a class of the session wrapped as `Repr("<class 'X'>")` and a builtin type
          // bare as `<class 'X'>`, and the name stands the same in both.
          let shown = said.py_repr();
          let name =
            shown.split_once("<class '").and_then(|(_, rest)| rest.split('\'').next()).filter(
              |name| !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            );
          let Some(name) = name else {
            return Ok(PyString::new(py, &shown).into_any());
          };
          match made.named(py, name)? {
            Some(held) => Ok(held),
            None => match py.import("builtins")?.getattr(name) {
              Ok(held) => Ok(held),
              Err(_) => Ok(PyString::new(py, &shown).into_any()),
            },
          }
        }
      },
    },
  }
}

/// One value of python, as the sandbox takes it: a shape as its name, as the engine names one; a name of the
/// engine as its name; an ear of the crate and a generator as an ear; a function as one the sandbox calls back; a
/// template string as its interpolations; plain data as it is, each entry as it crosses; an instance of a class or
/// an exception as its name and its fields; and nothing else.
fn of_python(door: &Door, value: &Bound<'_, PyAny>) -> PyResult<Object> {
  let py = value.py();
  let made = door.made();
  let each = |held: &Bound<'_, PyAny>| {
    held.try_iter()?.map(|one| of_python(door, &one?)).collect::<PyResult<Vec<_>>>()
  };
  if value.is_none() {
    return Ok(Object::none());
  }
  if value.is(py.Ellipsis()) {
    return Ok(Object::ellipsis());
  }
  if let Ok(held) = value.cast::<PyBool>() {
    return Ok(Object::bool(held.is_true()));
  }
  if let Ok(held) = value.cast::<PyInt>() {
    return Ok(Object::int(held.extract::<i64>()?));
  }
  if let Ok(held) = value.cast::<PyFloat>() {
    return Ok(Object::float(held.value()));
  }
  if let Ok(held) = value.cast::<PyString>() {
    return Ok(Object::string(held.to_str()?));
  }
  // A class a word defined and a callable the engine made go back in by their handle, and an instance of such a
  // class by the handle of its class and its fields; any other type goes in as its name, which is how a shape is said.
  if let Ok(n) = value.getattr("__monty__").and_then(|n| n.extract::<i64>())
    && (value.is_instance_of::<PyType>() || value.is_instance_of::<PyFunction>())
  {
    return Ok(marked("made", [("id", Object::int(n))]));
  }
  if let Ok(n) = value.get_type().getattr("__monty__").and_then(|n| n.extract::<i64>()) {
    let mut fields = Vec::new();
    for (key, one) in value.getattr("__dict__")?.cast::<PyDict>()?.iter() {
      fields.push((of_python(door, &key)?, of_python(door, &one)?));
    }
    // An exception holds what it was made with apart from its fields.
    if value.is_instance_of::<PyBaseException>() {
      fields.push((Object::string("args"), of_python(door, &value.getattr("args")?)?));
    }
    return Ok(marked("instance", [("class", Object::int(n)), ("fields", Object::dict(fields))]));
  }
  if value.is_instance_of::<PyType>() {
    return Ok(Object::string(value.getattr("__name__")?.extract::<String>()?));
  }
  if let Some(name) = made.name_of(py, value)? {
    return Ok(marked("name", [("name", Object::string(name))]));
  }
  let kind = value.get_type().name()?.to_string();
  if matches!(kind.as_str(), "GenericAlias" | "UnionType" | "Union") {
    return Ok(Object::string(bare(&value.repr()?.to_string())));
  }
  if value.is_instance_of::<NativeEar>() || generator(value)? {
    return door.crossed(value);
  }
  if value.is_callable() {
    let called = Called { door: door.clone(), f: value.clone().unbind() };
    return Ok(door.0.hosted.callable(Box::new(move |args| called.call(args))));
  }
  if kind == "Template" {
    let mut pairs = Vec::new();
    for one in value.getattr("interpolations")?.try_iter()? {
      let one = one?;
      pairs.push(Object::tuple([
        of_python(door, &one.getattr("value")?)?,
        of_python(door, &one.getattr("expression")?)?,
      ]));
    }
    return Ok(marked("Templated", [("interpolations", Object::list(pairs))]));
  }
  if value.is_instance_of::<PyList>() {
    return Ok(Object::list(each(value)?));
  }
  if value.is_instance_of::<PyTuple>() {
    return Ok(Object::tuple(each(value)?));
  }
  if let Ok(held) = value.cast::<PyDict>() {
    let mut pairs = Vec::new();
    for (key, one) in held.iter() {
      pairs.push((of_python(door, &key)?, of_python(door, &one)?));
    }
    // A map that holds the key of the mark goes in as its pairs, so the stand-in never reads it as a mark.
    if held.contains(IS)? {
      let pairs = pairs.into_iter().map(|(key, one)| Object::tuple([key, one]));
      return Ok(marked("dict", [("args", Object::list([Object::list(pairs)]))]));
    }
    return Ok(Object::dict(pairs));
  }
  if value.is_instance_of::<PyBaseException>() {
    return Ok(marked(&kind, [("args", Object::list(each(&value.getattr("args")?)?))]));
  }
  if let Ok(fields) = value.getattr("__dataclass_fields__") {
    let mut held = Vec::new();
    for (key, _) in fields.cast::<PyDict>()?.iter() {
      let key = key.extract::<String>()?;
      held.push((key.clone(), of_python(door, &value.getattr(key.as_str())?)?));
    }
    return Ok(marked(&kind, held.iter().map(|(key, one)| (key.as_str(), one.clone()))));
  }
  Err(PyTypeError::new_err(format!("{kind} cannot cross to the engine")))
}

/// A function of python, called back by the sandbox with what the word gave it.
struct Called {
  door: Door,
  f: Py<PyAny>,
}

impl Called {
  fn call(&self, args: Vec<Object>) -> Result<Object, Fault> {
    Python::attach(|py| {
      let got = (|| {
        let args = args
          .iter()
          .map(|one| to_python(py, self.door.made(), one.as_ref()))
          .collect::<PyResult<Vec<_>>>()?;
        let got = self.f.bind(py).call1(PyTuple::new(py, args)?)?;
        of_python(&self.door, &got)
      })();
      got.map_err(|no| fault_of(&self.door, py, &no))
    })
  }
}

/// The name of a shape as the engine names one: what python shows, with every module stripped, since the word of
/// a chain says a class of the engine or of the chain by its bare name.
fn bare(shown: &str) -> String {
  /// A run of name characters, cut after its last dot.
  fn cut(run: &str) -> &str {
    run.rfind('.').map_or(run, |at| &run[at + 1..])
  }
  let mut out = String::new();
  let mut run = String::new();
  for c in shown.chars() {
    if c.is_alphanumeric() || matches!(c, '_' | '.' | ':' | '/') {
      run.push(c);
    } else {
      out.push_str(cut(&run));
      run.clear();
      out.push(c);
    }
  }
  out.push_str(cut(&run));
  out
}

/// The extension module.
#[pymodule]
fn _monty(module: &Bound<'_, PyModule>) -> PyResult<()> {
  module.add_class::<PyEngine>()?;
  module.add_class::<NativeEar>()?;
  module.add_function(wrap_pyfunction!(files, module)?)?;
  module.add_function(wrap_pyfunction!(bash, module)?)?;
  module.add_function(wrap_pyfunction!(time, module)?)?;
  module.add_function(wrap_pyfunction!(store, module)?)?;
  module.add_function(wrap_pyfunction!(kept, module)?)?;
  module.add_function(wrap_pyfunction!(gate, module)?)?;
  Ok(())
}
