//! The door to python, behind the `python` feature: the module `furb_monty._monty`, which gives each function of
//! the host API, an [`PyEngine`] whose verbs are those of the crate, and the act a verb makes. A value of the engine
//! comes out as the instance of `furb.python` it is, an exception as the one object it is for the life, a generator
//! of python goes in as an ear, and a function as one the sandbox calls back.

use std::{
  cell::{OnceCell, RefCell},
  collections::HashMap,
  sync::Arc,
  task::{Wake, Waker},
};

use pyo3::{
  Borrowed, Bound, Py, PyAny, PyResult, Python,
  exceptions::{PyBaseException, PyStopIteration, PyTypeError},
  prelude::*,
  sync::PyOnceLock,
  types::{
    PyBool, PyCFunction, PyDict, PyFloat, PyFunction, PyInt, PyList, PyModule, PyString, PyTuple,
    PyType,
  },
};

use super::{Given, NativeEar, Record, Said, Told, Value, words};
use crate::{
  Ear, Engine, Fact, Fault, Heard, Object, ObjectRef, Step, Voice,
  ear::{self, Call, Spoken},
  engine::{callable, handed},
  life::{Answer, Opening, Stream},
  value::{IS, entry, field, marked, templated},
};

/// What makes a value of the engine the python object it is: the engine of this interpreter, whose classes an
/// instance is made from, and the one object each exception of the life is, by its name and what it was made
/// with, which a new life forgets.
struct Made {
  /// The engine of this interpreter, `furb.python`, which the package binds whatever the switch says.
  python: Py<PyAny>,
  faults: Py<PyDict>,
}

/// What makes a value the python object it is, for the one interpreter of this process.
static MADE: PyOnceLock<Made> = PyOnceLock::new();

/// What makes a value the python object it is.
fn made(py: Python<'_>) -> PyResult<&'static Made> {
  MADE.get_or_try_init(py, || {
    Ok(Made {
      python: py.import("furb")?.getattr("python")?.unbind(),
      faults: PyDict::new(py).unbind(),
    })
  })
}

impl Made {
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
    let shown = args.iter().map(|one| one.repr().map(|r| r.to_string()).unwrap_or_default());
    let key = format!("{name}{}", shown.collect::<Vec<_>>().join(", "));
    let faults = self.faults.bind(py);
    if let Some(held) = faults.get_item(&key)? {
      return Ok(held);
    }
    let made = self.class(py, name)?.call1(PyTuple::new(py, args)?)?;
    faults.set_item(&key, &made)?;
    Ok(made)
  }
}

impl From<Fault> for PyErr {
  /// What the engine raised, raised here as the exception it is.
  fn from(fault: Fault) -> PyErr {
    Python::attach(|py| exception(py, &fault).map_or_else(|no| no, PyErr::from_value))
  }
}

/// A fault, as the one exception object it is for the life.
fn exception<'py>(py: Python<'py>, fault: &Fault) -> PyResult<Bound<'py, PyAny>> {
  let args =
    fault.args.iter().map(|one| to_python(py, one.as_ref())).collect::<PyResult<Vec<_>>>()?;
  made(py)?.fault(py, &fault.name, args)
}

/// What python raised, as the crossing reads it: its type and what it was made with, or what it says when its words
/// do not cross.
fn fault_of(py: Python<'_>, no: &PyErr) -> Fault {
  let value = no.value(py);
  let name = value.get_type().name().map_or("Exception".to_owned(), |one| one.to_string());
  let args = value.getattr("args").ok().and_then(|args| {
    args.try_iter().ok()?.map(|one| of_python(&one.ok()?).ok()).collect::<Option<Vec<_>>>()
  });
  Fault::new(name, args.unwrap_or_else(|| vec![Object::string(no.to_string())]))
}

impl<'a, 'py> FromPyObject<'a, 'py> for Value {
  type Error = PyErr;

  fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
    of_python(&value).map(Value)
  }
}

impl<'py> IntoPyObject<'py> for Value {
  type Target = PyAny;
  type Output = Bound<'py, PyAny>;
  type Error = PyErr;

  fn into_pyobject(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    to_python(py, self.0.as_ref())
  }
}

impl<'py> IntoPyObject<'py> for Record {
  type Target = PyAny;
  type Output = Bound<'py, PyAny>;
  type Error = PyErr;

  fn into_pyobject(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    to_python(py, self.0.as_ref())
  }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Given {
  type Error = PyErr;

  /// An ear of the crate as itself, and a generator heard as an ear.
  fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
    if let Ok(held) = value.cast::<NativeEar>() {
      return Ok(Given(held.borrow_mut().taken()?));
    }
    if !generator(&value)? {
      let kind = value.get_type().name()?;
      return Err(PyTypeError::new_err(format!("{kind} is no ear: an ear is a generator")));
    }
    Ok(Given(Box::new(PyEar { ear: Some(value.to_owned().unbind()) })))
  }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Said {
  type Error = PyErr;

  fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
    Ok(Said(value.extract::<Vec<String>>().ok()))
  }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Told {
  type Error = PyErr;

  /// A function of python, called with the name of the event, from the thread that tells it; what it raised goes
  /// to the hook of python for what no one can raise.
  fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
    let told = value.to_owned().unbind();
    Ok(Told(Arc::new(move |event| {
      Python::attach(|py| {
        if let Err(no) = told.call1(py, (event,)) {
          no.write_unraisable(py, None);
        }
      });
      true
    })))
  }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Stream {
  type Error = PyErr;

  /// A function of python, told on a thread of the models; what it raised goes to the hook of python for what no one
  /// can raise.
  fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
    let stream = value.to_owned().unbind();
    Ok(Stream(Arc::new(move |rung, chain, text, thinking| {
      Python::attach(|py| {
        if let Err(no) = stream.call1(py, (rung, chain, text, thinking)) {
          no.write_unraisable(py, None);
        }
      });
    })))
  }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Answer {
  type Error = PyErr;

  /// A function of python as a model: it is called with the request, as JSON reads it, and a function
  /// `write(text="", thinking="")` that tells what it writes, on a thread of its own; the turn it gives is the
  /// answer, and what it raised the refusal.
  fn extract(value: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
    let answer = Arc::new(value.to_owned().unbind());
    Ok(Answer(Arc::new(move |request, told| {
      let answer = Arc::clone(&answer);
      Box::pin(async move {
        let called = tokio::task::spawn_blocking(move || {
          Python::attach(|py| {
            let json = py.import("json")?;
            let request = json.call_method1("loads", (request.to_string(),))?;
            let write = PyCFunction::new_closure(py, None, None, move |_, kwargs| {
              let part = |key: &str| -> String {
                let item = kwargs.and_then(|kwargs| kwargs.get_item(key).ok().flatten());
                item.and_then(|one| one.extract().ok()).unwrap_or_default()
              };
              told(&part("text"), &part("thinking"));
            })?;
            let turn = answer.call1(py, (request, write))?;
            json.call_method1("dumps", (turn,))?.extract::<String>()
          })
          .map_err(|no| no.to_string())
        });
        let turn = called.await.map_err(|no| no.to_string())??;
        crate::wire::parsed(&turn).map_err(|fault| fault.message())
      })
    })))
  }
}

/// Whether a value of python is a generator, which is what an ear of python is.
fn generator(value: &Bound<'_, PyAny>) -> PyResult<bool> {
  value.py().import("inspect")?.call_method1("isgenerator", (value,))?.is_truthy()
}

/// A generator of python, heard as an ear: sent what it hears.
///
/// It yields a saying, which is a tuple, or nothing, and it hears nothing at its birth. A verb it says while it hears
/// goes to the life that hears it.
struct PyEar {
  ear: Option<Py<PyAny>>,
}

impl Ear for PyEar {
  fn resume(&mut self, heard: Heard) -> Step {
    Python::attach(|py| self.step(py, heard).unwrap_or_else(|no| Step::Raised(fault_of(py, &no))))
  }
}

impl PyEar {
  fn step(&mut self, py: Python<'_>, heard: Heard) -> PyResult<Step> {
    let ear = self.ear.as_ref().ok_or_else(|| PyTypeError::new_err("the ear is over"))?.bind(py);
    let sent = match heard {
      Heard::Born(_) => py.None().into_bound(py),
      Heard::Fact(fact) => to_python(py, fact.0.as_ref())?,
    };
    let got = match ear.call_method1("send", (sent,)) {
      Ok(got) => got,
      Err(no) if no.is_instance_of::<PyStopIteration>(py) => {
        self.ear.take();
        return Ok(Step::Over);
      }
      Err(no) => return Ok(Step::Raised(fault_of(py, &no))),
    };
    Ok(Step::of(&of_python(&got)?).unwrap_or_else(Step::Raised))
  }
}

/// What the engine of this interpreter steps an ear of the crate with once it is born: the voices its work speaks
/// with, and what wakes the loop when one speaks.
pub(super) struct Stepped {
  voices: Voice,
  waker: Waker,
}

#[pymethods]
impl NativeEar {
  fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
    slf
  }

  fn __next__(slf: &Bound<'_, Self>) -> PyResult<Py<PyAny>> {
    NativeEar::send(slf, &slf.py().None().into_bound(slf.py()))
  }

  /// The ear heard, as the engine of this interpreter steps a generator: nothing at its birth, which is given the
  /// voice its work speaks with under the name the engine steps it by, and each fact after. A verb it calls is said
  /// to the engine of this interpreter at once, and what it gave is what the call gives.
  fn send(slf: &Bound<'_, Self>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let py = slf.py();
    let python = made(py)?.python.bind(py);
    let born = slf.borrow_mut().hearing.get_mut()?.stepped.is_none();
    let heard = if born {
      let name: String = python.getattr("site")?.call_method0("get")?.extract()?;
      let weak = py.import("weakref")?.getattr("ref")?.call1((slf,))?.unbind();
      let waker = waker(py, move |py| match weak.bind(py).call0()?.cast::<NativeEar>() {
        Ok(ear) => pump(ear),
        Err(_) => Ok(()),
      })?;
      let voices = Voice::new();
      voices.drained(&waker);
      let heard = Heard::Born(voices.of(&name));
      slf.borrow_mut().hearing.get_mut()?.stepped = Some(Stepped { voices, waker });
      heard
    } else {
      let fact =
        Fact::of(heard(a)?.as_ref()).ok_or_else(|| PyTypeError::new_err("an ear hears a fact"))?;
      Heard::Fact(fact)
    };
    let mut held = slf.borrow_mut();
    let hearing = held.hearing.get_mut()?;
    let Some(ear) = hearing.ear.as_mut() else { return Err(PyStopIteration::new_err(())) };
    let mut answers = |call: Call| {
      verb_said(py, &call).and_then(|got| of_python(&got)).map_err(|no| fault_of(py, &no))
    };
    match ear::heard(&mut answers, || ear.resume(heard)) {
      Step::Wait => Ok(py.None()),
      Step::Say(saying) => Ok(to_python(py, saying.0.as_ref())?.unbind()),
      Step::Over => {
        hearing.ear = None;
        Err(PyStopIteration::new_err(()))
      }
      Step::Raised(fault) => {
        hearing.ear = None;
        Err(fault.into())
      }
    }
  }
}

/// What the work of an ear of the crate said since, said into the engine of this interpreter under the name of the
/// ear.
fn pump(ear: &Bound<'_, NativeEar>) -> PyResult<()> {
  let py = ear.py();
  let said = match &ear.borrow_mut().hearing.get_mut()?.stepped {
    Some(stepped) => stepped.voices.drained(&stepped.waker),
    None => return Ok(()),
  };
  let python = made(py)?.python.bind(py);
  let (site, say) = (python.getattr("site")?, python.getattr("say")?);
  for one in said {
    let token = site.call_method1("set", (one.by,))?;
    let got = match one.spoken {
      Spoken::Saying(saying) => {
        to_python(py, saying.0.as_ref()).and_then(|saying| say.call1(saying.cast::<PyTuple>()?))
      }
      Spoken::Verb(call) => verb_said(py, &call),
    };
    site.call_method1("reset", (token,))?;
    got?;
  }
  Ok(())
}

/// One verb that an ear of the crate said, said to the engine of this interpreter with its words, and what it gave.
fn verb_said<'py>(py: Python<'py>, call: &Call) -> PyResult<Bound<'py, PyAny>> {
  let args = call.args.iter().map(|one| to_python(py, one.as_ref()));
  let args = PyTuple::new(py, args.collect::<PyResult<Vec<_>>>()?)?;
  let kwargs = PyDict::new(py);
  for (key, one) in &call.kwargs {
    kwargs.set_item(key, to_python(py, one.as_ref())?)?;
  }
  made(py)?.python.bind(py).getattr(call.verb.as_str())?.call(args, Some(&kwargs))
}

/// What wakes the loop of python to drive an engine, when a voice spoke from another thread: the loop the engine was
/// booted in, and what drives it there.
struct Wakes {
  running: Py<PyAny>,
  drive: Py<PyAny>,
}

impl Wake for Wakes {
  fn wake(self: Arc<Self>) {
    Python::attach(|py| {
      // A loop that is closed drives nothing more, and the engine is gone with it.
      let _ = self.running.bind(py).call_method1("call_soon_threadsafe", (self.drive.bind(py),));
    });
  }
}

/// What wakes the running loop to drive, as `drive` does.
fn waker(
  py: Python<'_>,
  drive: impl Fn(Python<'_>) -> PyResult<()> + Send + Sync + 'static,
) -> PyResult<Waker> {
  let running = py.import("asyncio")?.call_method0("get_running_loop")?.unbind();
  let drive = PyCFunction::new_closure(py, None, None, move |args, _| drive(args.py()))?;
  Ok(Waker::from(Arc::new(Wakes { running, drive: drive.into_any().unbind() })))
}

/// One engine, held on the thread of python and driven in the loop it was booted in, when an ear of the crate speaks
/// from a thread of its own.
#[pyclass(module = "furb_monty._monty", name = "Engine", unsendable, weakref)]
pub struct PyEngine {
  engine: RefCell<Option<Engine>>,
  root: String,
  raised: Option<Fault>,
  record: Vec<Object>,
  waker: OnceCell<Waker>,
}

impl PyEngine {
  /// The engine, held for python, driven once.
  fn held(py: Python<'_>, engine: Engine, record: Vec<Object>) -> PyResult<Py<PyEngine>> {
    made(py)?.faults.bind(py).clear();
    let root = engine.root().to_owned();
    let raised = engine.raised().cloned();
    let held =
      PyEngine { engine: RefCell::new(Some(engine)), root, raised, record, waker: OnceCell::new() };
    let held = Py::new(py, held)?;
    let weak = py.import("weakref")?.getattr("ref")?.call1((&held,))?.unbind();
    let waker = waker(py, move |py| match weak.bind(py).call0()?.cast::<PyEngine>() {
      Ok(engine) => engine.borrow().pump().map_err(Into::into),
      Err(_) => Ok(()),
    })?;
    let _ = held.borrow(py).waker.set(waker);
    held.borrow(py).pump()?;
    Ok(held)
  }

  /// One call of the engine.
  fn call<T>(&self, call: impl FnOnce(&mut Engine) -> Result<T, Fault>) -> Result<T, Fault> {
    let mut held = self.engine.try_borrow_mut().map_err(|_| {
      Fault::refused("the engine hears an ear, which calls the engine by its verbs alone")
    })?;
    call(held.as_mut().ok_or_else(|| Fault::refused("the engine is disposed"))?)
  }

  /// One verb, said by what the life hears now while it hears, and by the operator otherwise.
  fn said(
    &self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    if ear::hearing() {
      return ear::call(name, args, kwargs);
    }
    self.call(|engine| engine.verb(name, args, kwargs))
  }

  /// The engine driven as far as it goes: what the voices of its ears said is said into it, and whoever awaits an act
  /// that is done now is told.
  fn pump(&self) -> Result<(), Fault> {
    let (Some(waker), true) = (self.waker.get(), self.engine.borrow().is_some()) else {
      return Ok(());
    };
    self.call(|engine| engine.pump(waker))
  }
}

#[pymethods]
impl PyEngine {
  /// An engine, opened from the record, on these ears, each a generator of python or an ear of the crate under the
  /// name the engine hears it by, in the order the engine offers them a question. It is booted in the running loop.
  #[staticmethod]
  fn boot(
    py: Python<'_>,
    record: Vec<Value>,
    ears: Vec<(String, Given)>,
  ) -> PyResult<Py<PyEngine>> {
    let ears = ears.into_iter().map(|(name, ear)| (name, ear.0));
    let engine = Engine::boot(record.into_iter().map(|one| one.0), ears)?;
    PyEngine::held(py, engine, Vec::new())
  }

  /// A life opened as every host of the crate opens one: on its record, on these ears of the host, and then on the
  /// ears of the crate and of the extensions. A life whose record drifted is refused.
  #[staticmethod]
  #[pyo3(signature = (ears, **opening))]
  fn open(
    py: Python<'_>,
    ears: Vec<(String, Given)>,
    opening: Option<Opening>,
  ) -> PyResult<Py<PyEngine>> {
    let opening = opening.ok_or_else(|| Fault::refused("a life opens on a directory"))?;
    let (engine, record) = opening.boot(ears.into_iter().map(|(name, ear)| (name, ear.0)))?;
    PyEngine::held(py, engine, record)
  }

  /// The root chain of the life, which is the first act of any record.
  #[getter]
  fn root(&self) -> &str {
    &self.root
  }

  /// What boot raised, as the exception it is, and nothing when it raised nothing. After a drift the life goes on,
  /// with nothing kept.
  #[getter]
  fn raised(&self) -> Option<Value> {
    self.raised.as_ref().map(|fault| Value(fault.object()))
  }

  /// The record the life opened on, which the journal said again whole before boot returned.
  #[getter]
  fn record(&self) -> Record {
    Record(Object::list(self.record.iter().cloned()))
  }

  /// Whether the engine is gone.
  #[getter]
  fn disposed(&self) -> bool {
    self.engine.try_borrow().is_ok_and(|engine| engine.is_none())
  }

  /// Who speaks in the life, and who speaks from now on when a name is given: the site of the contract, which the
  /// work an ear began sets to the name of that ear before it speaks.
  #[pyo3(signature = (value = None))]
  fn site(&self, value: Option<String>) -> Result<String, Fault> {
    self.call(|engine| engine.site(value.as_deref()))
  }

  /// One name of the engine, said by its name with these words, and what it gave.
  #[pyo3(signature = (name, args = None, kwargs = None))]
  fn verb(
    &self,
    name: &str,
    args: Option<Vec<Value>>,
    kwargs: Option<HashMap<String, Value>>,
  ) -> Result<Value, Fault> {
    let (args, kwargs) = words(args, &kwargs);
    self.said(name, args, kwargs).map(Value)
  }

  /// One callable the engine made, called back by the number it went out under, with these words.
  #[pyo3(signature = (n, args = None, kwargs = None))]
  fn made(
    &self,
    n: i64,
    args: Option<Vec<Value>>,
    kwargs: Option<HashMap<String, Value>>,
  ) -> Result<Value, Fault> {
    let (args, kwargs) = words(args, &kwargs);
    self.call(|engine| engine.made(n, args, kwargs)).map(Value)
  }

  /// A callable the engine made, forgotten: the host holds its number no more.
  fn forget(&self, n: i64) -> Result<(), Fault> {
    self.call(|engine| engine.forget(n))
  }

  /// The work that an earlier life left, which waits for a wake that this life says, each act by its name and its
  /// kind, as the engine of the crate finds it.
  fn pending(&self) -> Result<Vec<(String, String)>, Fault> {
    self.call(Engine::pending)
  }

  /// What an act comes to, which python awaits: its value, or the exception it completed with, raised.
  fn result<'py>(&self, py: Python<'py>, act: String) -> PyResult<Bound<'py, PyAny>> {
    let future =
      py.import("asyncio")?.call_method0("get_running_loop")?.call_method0("create_future")?;
    let settled = future.clone().unbind();
    self.call(|engine| {
      engine.watch(&act, move |got| {
        Python::attach(|py| {
          if let Err(no) = settle(settled.bind(py), got) {
            no.write_unraisable(py, None);
          }
        });
      })
    })?;
    Ok(future)
  }

  /// What an act came to, and whether it is done.
  fn outcome(&self, act: String) -> Result<super::Outcome, Fault> {
    let got = self.call(|engine| engine.outcome(&act))?;
    Ok(super::Outcome { done: got.is_some(), value: Value(got.unwrap_or_else(Object::none)) })
  }

  /// One name of a chain, the root when none is given, read without calling it.
  #[pyo3(signature = (name, chain = None))]
  fn inspect(&self, name: String, chain: Option<String>) -> Result<super::Inspection, Fault> {
    let (kind, representation, value) =
      self.call(|engine| engine.inspect(&name, chain.as_deref()))?;
    Ok(super::Inspection { name, kind, representation, value: Value(value) })
  }

  /// Every name the module of a chain binds, the root when none is given, in the order it bound them.
  #[pyo3(signature = (chain = None))]
  fn names(&self, chain: Option<String>) -> Result<Vec<String>, Fault> {
    self.call(|engine| engine.names(chain.as_deref()))
  }

  /// The engine is gone, and its ears with it: a command of the crate ends, a wait ends, and the store lets its
  /// record go.
  fn dispose(&self) -> Result<(), Fault> {
    let mut held = self
      .engine
      .try_borrow_mut()
      .map_err(|_| Fault::refused("the engine is in a call of the host"))?;
    held.take();
    Ok(())
  }
}

/// A future of python, settled with what an act came to: its value, or the exception it completed with.
fn settle(future: &Bound<'_, PyAny>, got: &Object) -> PyResult<()> {
  if future.call_method0("done")?.is_truthy()? {
    return Ok(());
  }
  let value = to_python(future.py(), got.as_ref())?;
  let how = if value.is_instance_of::<PyBaseException>() { "set_exception" } else { "set_result" };
  future.call_method1(how, (value,)).map(drop)
}

/// An act: its name, which a control takes, and what it comes to, which python awaits.
#[pyclass(module = "furb_monty._monty", name = "Act", frozen)]
pub struct PyAct {
  #[pyo3(get)]
  id: String,
  engine: Py<PyEngine>,
}

impl PyAct {
  /// The act a verb of the engine made.
  fn of(engine: &Bound<'_, PyEngine>, got: Object) -> Result<PyAct, Fault> {
    let id = got
      .as_ref()
      .as_str()
      .map(str::to_owned)
      .ok_or_else(|| Fault::refused("a verb gave no act"))?;
    Ok(PyAct { id, engine: engine.clone().unbind() })
  }
}

#[pymethods]
impl PyAct {
  fn __str__(&self) -> &str {
    &self.id
  }

  fn __repr__(&self) -> String {
    format!("Act({:?})", self.id)
  }

  fn __await__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    self.engine.bind(py).borrow().result(py, self.id.clone())?.call_method0("__await__")
  }
}

// The verbs of the contract, one method each, which the build makes from the contract.
include!(concat!(env!("OUT_DIR"), "/py.rs"));

/// One value of the sandbox, as python holds it: an instance of a class of the engine as that instance, an
/// exception as the one object it is for the life, and plain data as itself.
fn to_python<'py>(py: Python<'py>, said: ObjectRef<'_>) -> PyResult<Bound<'py, PyAny>> {
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
  let made = made(py)?;
  let each = |held: Vec<ObjectRef<'_>>| {
    held.into_iter().map(|one| to_python(py, one)).collect::<PyResult<Vec<_>>>()
  };
  match said.type_name() {
    "NoneType" | "None" => Ok(py.None().into_bound(py)),
    "ellipsis" => Ok(py.Ellipsis().into_bound(py)),
    "list" => Ok(PyList::new(py, each(said.items().unwrap_or_default())?)?.into_any()),
    "tuple" => Ok(PyTuple::new(py, each(said.items().unwrap_or_default())?)?.into_any()),
    "dict" => {
      let pairs = said.pairs().unwrap_or_default();
      let at = |name: &str| field(&said, name);
      if let Some(mark) = at(IS).and_then(|one| one.as_str()) {
        // A map that holds the key of the mark goes out as its pairs, and is the map it is here.
        if mark == "dict"
          && let Some(held) = at("args").and_then(|one| entry(&one, 0)).and_then(|one| one.items())
        {
          let pairs = held.iter().map(|pair| entry(pair, 0).zip(entry(pair, 1)));
          let pairs = pairs.collect::<Option<Vec<_>>>().ok_or_else(|| {
            PyTypeError::new_err("a map that goes out as its pairs holds pairs of two")
          })?;
          return Ok(dict(py, pairs)?.into_any());
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
        // A class a word defined, as the type python holds it by its number, derived from the type its base is
        // here, and an instance of one, as an object of that type holding the fields the sandbox carries out.
        if mark == "class"
          && let Some(n) = at("id").and_then(|one| one.as_int())
          && let Some(name) = at("name").and_then(|one| one.as_str())
          && let Some(base) = at("base")
        {
          let base = to_python(py, base)?;
          return py.import("furb_monty.engine")?.call_method1("classed", (n, name, base));
        }
        if mark == "instance"
          && let Some(class) = at("class")
          && let Some(value) = at("value")
        {
          let class = to_python(py, class)?;
          let fields = dict(py, value.pairs().unwrap_or_default())?;
          return py.import("furb_monty.engine")?.call_method1("instanced", (class, fields));
        }
        // A value as the record keeps it: an instance of a class of the engine or of the interpreter, by the name of
        // its class, what it was made with and its fields.
        if let Ok(class) = made.class(py, mark) {
          let args = each(at("args").and_then(|one| one.items()).unwrap_or_default())?;
          if class
            .cast::<PyType>()
            .is_ok_and(|one| one.is_subclass_of::<PyBaseException>().unwrap_or_default())
          {
            return made.fault(py, mark, args);
          }
          let fields = pairs.iter().filter(|(key, _)| !matches!(key.as_str(), Some(IS | "args")));
          let kwargs = dict(py, fields.copied())?;
          return class.call(PyTuple::new(py, args)?, Some(&kwargs));
        }
      }
      Ok(dict(py, pairs)?.into_any())
    }
    _ => match crate::value::Fault::of(said) {
      Some(fault) => {
        made.fault(py, &fault.name, each(fault.args.iter().map(Object::as_ref).collect())?)
      }
      None => match said.pairs() {
        Some(fields) => made.class(py, said.type_name())?.call((), Some(&dict(py, fields)?)),
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

/// A map of python from pairs of the sandbox, each key and each value as python holds it.
fn dict<'py, 'a>(
  py: Python<'py>,
  pairs: impl IntoIterator<Item = (ObjectRef<'a>, ObjectRef<'a>)>,
) -> PyResult<Bound<'py, PyDict>> {
  let map = PyDict::new(py);
  for (key, one) in pairs {
    map.set_item(to_python(py, key)?, to_python(py, one)?)?;
  }
  Ok(map)
}

/// One value of python, as the sandbox takes it: a shape as its name, as the engine names one; a name of the
/// engine as its name; an ear of the crate and a generator as an ear, and a function as one the sandbox calls back,
/// each handed over to the life it comes into; a template string as its interpolations; plain data as it is, each
/// entry as it crosses; an instance of a class or an exception as its name and its fields; and nothing else.
fn of_python(value: &Bound<'_, PyAny>) -> PyResult<Object> {
  let py = value.py();
  let each = |held: &Bound<'_, PyAny>| {
    held.try_iter()?.map(|one| of_python(&one?)).collect::<PyResult<Vec<_>>>()
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
  // A class a word defined and a callable the engine made go back in by their number, and an instance of such a
  // class by the number of its class and its fields; any other type goes in as its name, which is how a shape is
  // said.
  if let Ok(n) = value.getattr("__monty__").and_then(|n| n.extract::<i64>())
    && (value.is_instance_of::<PyType>() || value.is_instance_of::<PyFunction>())
  {
    return Ok(marked("made", [("id", Object::int(n))]));
  }
  if let Ok(n) = value.get_type().getattr("__monty__").and_then(|n| n.extract::<i64>()) {
    let mut fields = Vec::new();
    for (key, one) in value.getattr("__dict__")?.cast::<PyDict>()?.iter() {
      fields.push((of_python(&key)?, of_python(&one)?));
    }
    // An exception holds what it was made with apart from its fields.
    if value.is_instance_of::<PyBaseException>() {
      fields.push((Object::string("args"), of_python(&value.getattr("args")?)?));
    }
    return Ok(marked("instance", [("class", Object::int(n)), ("fields", Object::dict(fields))]));
  }
  if value.is_instance_of::<PyType>() {
    return Ok(Object::string(value.getattr("__name__")?.extract::<String>()?));
  }
  if let Some(name) = made(py)?.name_of(py, value)? {
    return Ok(marked("name", [("name", Object::string(name))]));
  }
  let kind = value.get_type().name()?.to_string();
  if matches!(kind.as_str(), "GenericAlias" | "UnionType" | "Union") {
    return Ok(Object::string(bare(&value.repr()?.to_string())));
  }
  if let Ok(held) = value.cast::<NativeEar>() {
    return Ok(handed(held.borrow_mut().taken()?, false));
  }
  if generator(value)? {
    let state = py.import("inspect")?.call_method1("getgeneratorstate", (value,))?;
    let started = state.extract::<String>()? != "GEN_CREATED";
    return Ok(handed(Box::new(PyEar { ear: Some(value.clone().unbind()) }), started));
  }
  if value.is_callable() {
    let f = value.clone().unbind();
    return Ok(callable(move |args| called(&f, args)));
  }
  if kind == "Template" {
    let mut interpolations = Vec::new();
    for one in value.getattr("interpolations")?.try_iter()? {
      let one = one?;
      let expression = of_python(&one.getattr("expression")?)?;
      interpolations.push((of_python(&one.getattr("value")?)?, expression));
    }
    return Ok(templated(interpolations));
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
      pairs.push((of_python(&key)?, of_python(&one)?));
    }
    // A map that holds the key of the mark goes in as its pairs, so the engine never reads it as a mark.
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
      held.push((key.clone(), of_python(&value.getattr(key.as_str())?)?));
    }
    return Ok(marked(&kind, held.iter().map(|(key, one)| (key.as_str(), one.clone()))));
  }
  Err(PyTypeError::new_err(format!("{kind} cannot cross to the engine")))
}

/// A value of python as an ear of the crate hears it in a fact: as it goes in, but an ear or a function, which a
/// hearing takes nothing of, and a value that cannot go in, as python shows it, as the wire says it.
fn heard(value: &Bound<'_, PyAny>) -> PyResult<Object> {
  let each = |held: &Bound<'_, PyAny>| {
    held.try_iter()?.map(|one| heard(&one?)).collect::<PyResult<Vec<_>>>()
  };
  if value.is_exact_instance_of::<PyTuple>() {
    return Ok(Object::tuple(each(value)?));
  }
  if value.is_exact_instance_of::<PyList>() {
    return Ok(Object::list(each(value)?));
  }
  if let Ok(map) = value.cast_exact::<PyDict>()
    && !map.contains(IS)?
  {
    let mut pairs = Vec::new();
    for (key, one) in map.iter() {
      pairs.push((heard(&key)?, heard(&one)?));
    }
    return Ok(Object::dict(pairs));
  }
  let taken = value.is_callable() && !value.is_instance_of::<PyType>();
  if !taken
    && !value.is_instance_of::<NativeEar>()
    && !generator(value)?
    && let Ok(held) = of_python(value)
  {
    return Ok(held);
  }
  Ok(Object::string(value.repr()?.to_string()))
}

/// A function of python, called back by the sandbox with what the word gave it, and what it gave.
fn called(f: &Py<PyAny>, args: Vec<Object>) -> Result<Object, Fault> {
  Python::attach(|py| {
    let got = (|| {
      let args =
        args.iter().map(|one| to_python(py, one.as_ref())).collect::<PyResult<Vec<_>>>()?;
      of_python(&f.bind(py).call1(PyTuple::new(py, args)?)?)
    })();
    got.map_err(|no| fault_of(py, &no))
  })
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

/// The extension module: every function of the host API, the engine, its act, the ear of the crate, the engine and
/// the system prompt, and each constant of the contract.
#[pymodule]
fn _monty(module: &Bound<'_, PyModule>) -> PyResult<()> {
  use super::*;
  module.add_class::<PyEngine>()?;
  module.add_class::<PyAct>()?;
  module.add_class::<NativeEar>()?;
  let functions = [
    wrap_pyfunction!(files, module)?,
    wrap_pyfunction!(bash, module)?,
    wrap_pyfunction!(time, module)?,
    wrap_pyfunction!(store, module)?,
    wrap_pyfunction!(kept, module)?,
    wrap_pyfunction!(decode_record, module)?,
    wrap_pyfunction!(opened, module)?,
    wrap_pyfunction!(gate, module)?,
    wrap_pyfunction!(official, module)?,
    wrap_pyfunction!(enabled, module)?,
    wrap_pyfunction!(extensions, module)?,
    wrap_pyfunction!(memory, module)?,
    wrap_pyfunction!(skills, module)?,
    wrap_pyfunction!(config_directory, module)?,
    wrap_pyfunction!(shell, module)?,
    wrap_pyfunction!(shapes, module)?,
    wrap_pyfunction!(answered, module)?,
    wrap_pyfunction!(models, module)?,
    wrap_pyfunction!(model, module)?,
    wrap_pyfunction!(levels, module)?,
    wrap_pyfunction!(attach_image, module)?,
    wrap_pyfunction!(image_content, module)?,
    wrap_pyfunction!(image_path, module)?,
    wrap_pyfunction!(image_reference, module)?,
    wrap_pyfunction!(image_references, module)?,
    wrap_pyfunction!(image_type, module)?,
    wrap_pyfunction!(hearing, module)?,
    wrap_pyfunction!(call, module)?,
    wrap_pyfunction!(on_console_end, module)?,
  ];
  for one in functions {
    module.add_function(one)?;
  }
  module.add("ENGINE", crate::ENGINE)?;
  module.add("SYSTEM", crate::SYSTEM)?;
  crate::engine::constants(module)
}
