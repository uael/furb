//! furb for python, with the engine in monty: one life in the sandbox, reached from python.
//!
//! This is the door between the two interpreters, and it is the crate itself, behind the `python` feature. A host
//! of python hands over its ears as one object, asked under the name of each ear, and drives the life by saying
//! verbs, each by its name with its words. What crosses is what monty carries, made python here: a value of the
//! engine comes out as the instance of `furb.python` it is, a `Text` as a `Text`, an exception as the one object
//! that exception is for the life, and goes in as its name and its fields. A name of the engine crosses as its
//! name, a callable the engine made as one that calls it back, and a callable of python as one the ears call back
//! by name. Nothing of the crossing is python's to do.

use pyo3::{
  Bound, Py, PyAny, PyResult, Python,
  exceptions::{PyBaseException, PyTypeError},
  prelude::*,
  types::{
    PyBool, PyBytes, PyDict, PyFloat, PyFunction, PyInt, PyList, PyModule, PyString, PyTuple,
    PyType,
  },
};

use crate::{
  ear::{Ears, Reply},
  fact::Fact,
  life,
  value::{Fault, IS, Object, ObjectRef, entry, marked},
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

  fn clone_ref(&self, py: Python<'_>) -> Self {
    Made { python: self.python.clone_ref(py), faults: self.faults.clone_ref(py) }
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

/// The ears of the host: the one object of python, asked under the name of each ear.
struct Hosted {
  host: Py<PyAny>,
  made: Made,
}

impl Hosted {
  /// What the object answered, as a reply: nothing, a saying, a verb to say, what it raised, or its end.
  fn reply(&self, py: Python<'_>, got: PyResult<Bound<'_, PyAny>>) -> Reply {
    let got = match got {
      Ok(got) => got,
      // A fault of the host is raised in the ear, so the life reads it where the ear stood.
      Err(fault) => return Reply::Raised(fault_of(py, &fault)),
    };
    if got.is_none() {
      return Reply::Nothing;
    }
    let Ok(held) = got.cast::<PyTuple>() else { return Reply::Nothing };
    let mark =
      held.get_item(0).ok().and_then(|one| one.extract::<String>().ok()).unwrap_or_default();
    let plain = |i: usize| held.get_item(i).and_then(|one| of_python(&self.made, &self.host, &one));
    match mark.as_str() {
      "say" => match plain(1) {
        Ok(said) => Fact::of(said.as_ref()).map_or(Reply::Nothing, Reply::Say),
        Err(fault) => Reply::Raised(fault_of(py, &fault)),
      },
      "calls" => {
        let name =
          held.get_item(1).ok().and_then(|one| one.extract::<String>().ok()).unwrap_or_default();
        match (plain(2), plain(3)) {
          (Ok(args), Ok(kwargs)) => Reply::Calls {
            name,
            args: args
              .as_ref()
              .items()
              .unwrap_or_default()
              .into_iter()
              .map(|one| one.to_owned())
              .collect(),
            kwargs: kwargs
              .as_ref()
              .pairs()
              .unwrap_or_default()
              .into_iter()
              .map(|(key, one)| (key.as_str().unwrap_or_default().to_owned(), one.to_owned()))
              .collect(),
          },
          (Err(fault), _) | (_, Err(fault)) => Reply::Raised(fault_of(py, &fault)),
        }
      }
      "raised" => match held.get_item(1).and_then(|no| fault_of_value(&self.made, &self.host, &no))
      {
        Ok(fault) => Reply::Raised(fault),
        Err(fault) => Reply::Raised(fault_of(py, &fault)),
      },
      "over" => Reply::Over,
      _ => Reply::Nothing,
    }
  }
}

impl Ears for Hosted {
  fn called(
    &mut self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(String, Object)>,
  ) -> Result<Object, Fault> {
    Python::attach(|py| {
      let got = (|| {
        let args: Vec<Bound<'_, PyAny>> = args
          .iter()
          .map(|one| to_python(py, &self.made, one.as_ref()))
          .collect::<PyResult<_>>()?;
        let named = PyDict::new(py);
        for (key, one) in &kwargs {
          named.set_item(key, to_python(py, &self.made, one.as_ref())?)?;
        }
        let got =
          self.host.bind(py).call_method1("called", (name, PyTuple::new(py, args)?, named))?;
        of_python(&self.made, &self.host, &got)
      })();
      got.map_err(|fault| fault_of(py, &fault))
    })
  }

  fn hears(&mut self, name: &str, fact: Option<&Fact>) -> Reply {
    Python::attach(|py| {
      let got = match fact {
        Some(fact) => to_python(py, &self.made, fact.0.as_ref()),
        None => Ok(py.None().into_bound(py)),
      }
      .and_then(|one| self.host.bind(py).call_method1("hears", (name, one)));
      self.reply(py, got)
    })
  }

  fn answered(&mut self, name: &str, got: ObjectRef<'_>) -> Reply {
    Python::attach(|py| {
      let got = to_python(py, &self.made, got)
        .and_then(|one| self.host.bind(py).call_method1("answered", (name, one)));
      self.reply(py, got)
    })
  }
}

/// One life: the engine in the sandbox, and the host it reaches.
#[pyclass(module = "furb_monty._monty", name = "Life", unsendable)]
pub struct Life {
  held: life::Life,
  made: Made,
  /// The ears of the host, which a generator that crosses is given to, under a name of its own.
  ears: Py<PyAny>,
}

#[pymethods]
impl Life {
  /// A life, opened on the ears of the host, the names they hear by in the order the engine hears them, and the
  /// record a World kept. The ears are one object with `hears(name, fact)`, `answered(name, value)`,
  /// `ear(generator)`, which hears one more generator and gives the name it is heard by and whether it was started,
  /// and `callable(function)`, which gives the name the function is called back by.
  #[new]
  fn new(
    py: Python<'_>,
    ears: Py<PyAny>,
    names: Vec<String>,
    record: Bound<'_, PyAny>,
  ) -> PyResult<Self> {
    let made = Made::new(py)?;
    let kept = of_python(&made, &ears, &record)?;
    let kept: Vec<Object> =
      kept.as_ref().items().unwrap_or_default().into_iter().map(|one| one.to_owned()).collect();
    let hosted = Hosted { host: ears.clone_ref(py), made: made.clone_ref(py) };
    let held =
      life::Life::open_on(hosted, names).boot(kept).map_err(|fault| raised(py, &made, &fault))?;
    Ok(Life { held, made, ears })
  }

  /// A life restored from a dump of one that stood still, on the ears of the host under the names it was
  /// dumped with, and on the record a World kept, which the dump must match.
  #[staticmethod]
  fn restored(
    py: Python<'_>,
    ears: Py<PyAny>,
    names: Vec<String>,
    dump: &[u8],
    record: Bound<'_, PyAny>,
  ) -> PyResult<Self> {
    let made = Made::new(py)?;
    let kept = of_python(&made, &ears, &record)?;
    let kept: Vec<Object> =
      kept.as_ref().items().unwrap_or_default().into_iter().map(|one| one.to_owned()).collect();
    let hosted = Hosted { host: ears.clone_ref(py), made: made.clone_ref(py) };
    let held = life::Life::open_on(hosted, names)
      .restore(dump, kept)
      .map_err(|fault| raised(py, &made, &fault))?;
    Ok(Life { held, made, ears })
  }

  /// The life as bytes, where it stands still, for a later life to go on from.
  fn dump<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
    match self.held.dump() {
      Ok(bytes) => Ok(PyBytes::new(py, &bytes)),
      Err(fault) => Err(raised(py, &self.made, &fault)),
    }
  }

  /// The root chain of the life, which is the first act of any record.
  #[getter]
  fn root(&self) -> &str {
    self.held.root()
  }

  /// What boot raised, if it raised, as the exception it is, and nothing otherwise.
  #[getter]
  fn raised<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    match self.held.raised() {
      Some(one) => fault_to_python(py, &self.made, one),
      None => Ok(py.None().into_bound(py)),
    }
  }

  /// One verb of the engine by its name, said by the operator with these words, and what it gave.
  fn verb<'py>(
    &mut self,
    py: Python<'py>,
    name: &str,
    args: Bound<'py, PyAny>,
    kwargs: Bound<'py, PyDict>,
  ) -> PyResult<Bound<'py, PyAny>> {
    let args: Vec<Object> = args
      .try_iter()?
      .map(|one| of_python(&self.made, &self.ears, &one?))
      .collect::<PyResult<_>>()?;
    let mut named = Vec::new();
    for (key, value) in kwargs.iter() {
      named.push((key.extract::<String>()?, of_python(&self.made, &self.ears, &value)?));
    }
    let named = named.iter().map(|(key, value)| (key.as_str(), value.clone())).collect();
    let got = self.held.verb(name, args, named).map_err(|fault| raised(py, &self.made, &fault))?;
    to_python(py, &self.made, got.as_ref())
  }

  /// One callable the engine made, called back by its handle with these words, and what it gave.
  fn made<'py>(
    &mut self,
    py: Python<'py>,
    n: i64,
    args: Bound<'py, PyAny>,
    kwargs: Bound<'py, PyDict>,
  ) -> PyResult<Bound<'py, PyAny>> {
    let args: Vec<Object> = args
      .try_iter()?
      .map(|one| of_python(&self.made, &self.ears, &one?))
      .collect::<PyResult<_>>()?;
    let mut named = Vec::new();
    for (key, value) in kwargs.iter() {
      named.push((key.extract::<String>()?, of_python(&self.made, &self.ears, &value)?));
    }
    let named = named.iter().map(|(key, value)| (key.as_str(), value.clone())).collect();
    let got = self.held.made(n, args, named).map_err(|fault| raised(py, &self.made, &fault))?;
    to_python(py, &self.made, got.as_ref())
  }

  /// A callable the engine made, forgotten: this interpreter holds its handle no more.
  fn forget(&mut self, py: Python<'_>, n: i64) -> PyResult<()> {
    self.held.forget(n).map_err(|fault| raised(py, &self.made, &fault))
  }

  /// One reading of a map of the life where it stands, under these keys: `in`, `at`, `keys` or `len`.
  fn held<'py>(
    &mut self,
    py: Python<'py>,
    name: &str,
    keys: Vec<String>,
    ask: &str,
  ) -> PyResult<Bound<'py, PyAny>> {
    let keys = keys.into_iter().map(Object::string).collect();
    let got = self.held.held(name, keys, ask).map_err(|fault| raised(py, &self.made, &fault))?;
    to_python(py, &self.made, got.as_ref())
  }

  /// Who speaks in the life, and who speaks from now on when a value is given.
  fn site(&mut self, py: Python<'_>, value: Option<&str>) -> PyResult<String> {
    self.held.site(value).map_err(|fault| raised(py, &self.made, &fault))
  }

  /// What to call when an act is done, with what it came to: at once for one done already, and once otherwise.
  fn watch(&mut self, py: Python<'_>, act: &str, then: Py<PyAny>) -> PyResult<()> {
    let made = self.made.clone_ref(py);
    self
      .held
      .watch(act, move |got| {
        Python::attach(|py| {
          if let Ok(value) = to_python(py, &made, got.as_ref()) {
            let _ = then.bind(py).call1((value,));
          }
        });
      })
      .map_err(|fault| raised(py, &self.made, &fault))
  }
}

/// The gate of the crate, for the Kernel of this interpreter to read a sheet with: what the checker of this thread
/// found on the sheet, each error by its line, and no warning. The checker is the one every life of monty on the
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

/// A fault of python, as the crossing reads one: its type and what it says.
fn fault_of(py: Python<'_>, fault: &PyErr) -> Fault {
  let name = fault.get_type(py).name().map_or("Exception".to_owned(), |one| one.to_string());
  Fault::new(name, vec![Object::string(fault.to_string())])
}

/// An exception of python, as the crossing reads one: its type and what it was made with.
fn fault_of_value(made: &Made, ears: &Py<PyAny>, no: &Bound<'_, PyAny>) -> PyResult<Fault> {
  let name = no.get_type().name()?.to_string();
  let args = no.getattr("args")?;
  let args = args
    .cast::<PyTuple>()?
    .iter()
    .map(|one| of_python(made, ears, &one))
    .collect::<PyResult<Vec<_>>>()?;
  Ok(Fault::new(name, args))
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
    "list" => Ok(PyList::new(py, each(said.items().unwrap_or_default())?)?.into_any()),
    "tuple" => Ok(PyTuple::new(py, each(said.items().unwrap_or_default())?)?.into_any()),
    "dict" => {
      let pairs = said.pairs().unwrap_or_default();
      // The two marks of a callable that goes out: a name of the engine, and a handle to a callable it made.
      let at =
        |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|(_, one)| *one);
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
/// engine as its name; a generator as an ear of the host, which the ears hear from now on; a callable as one the
/// ears call back by name; a template string as its interpolations; plain data as it is, each entry as it crosses;
/// an instance of a class or an exception as its name and its fields; and nothing else.
fn of_python(made: &Made, ears: &Py<PyAny>, value: &Bound<'_, PyAny>) -> PyResult<Object> {
  let py = value.py();
  let each = |held: &Bound<'_, PyAny>| {
    held.try_iter()?.map(|one| of_python(made, ears, &one?)).collect::<PyResult<Vec<_>>>()
  };
  if value.is_none() {
    return Ok(Object::none());
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
      fields.push((of_python(made, ears, &key)?, of_python(made, ears, &one)?));
    }
    // An exception holds what it was made with apart from its fields.
    if value.is_instance_of::<PyBaseException>() {
      fields.push((Object::string("args"), of_python(made, ears, &value.getattr("args")?)?));
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
  if kind == "generator" {
    let (name, started): (String, bool) = ears.bind(py).call_method1("ear", (value,))?.extract()?;
    return Ok(marked("ear", [("name", Object::string(name)), ("started", Object::bool(started))]));
  }
  if value.is_callable() {
    let name: String = ears.bind(py).call_method1("callable", (value,))?.extract()?;
    return Ok(marked("callable", [("name", Object::string(name))]));
  }
  if kind == "Template" {
    let mut pairs = Vec::new();
    for one in value.getattr("interpolations")?.try_iter()? {
      let one = one?;
      pairs.push(Object::tuple([
        of_python(made, ears, &one.getattr("value")?)?,
        of_python(made, ears, &one.getattr("expression")?)?,
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
      pairs.push((of_python(made, ears, &key)?, of_python(made, ears, &one)?));
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
      held.push((key.clone(), of_python(made, ears, &value.getattr(key.as_str())?)?));
    }
    return Ok(marked(&kind, held.iter().map(|(key, one)| (key.as_str(), one.clone()))));
  }
  Err(PyTypeError::new_err(format!("{kind} cannot cross to the engine")))
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
  module.add_class::<Life>()?;
  module.add_function(wrap_pyfunction!(gate, module)?)?;
  Ok(())
}
