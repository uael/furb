//! furb for python, with the engine in monty: one life in the sandbox, reached from python.
//!
//! This is the one door between the two interpreters. A host of python hands over one callable, which is asked
//! for every ear of the life by name and answers plain, and drives the life by running words in it. Every value
//! that crosses is plain data: none, a truth, a number, a text, a list, and a map from a text to plain data. What
//! a tuple, a shape, an exception or a callable is in that form is `furb_monty.engine`'s to say, on both sides.

use furb::{Host, Life as Held, Refusal, Sand, Value, gate::Ty};
use pyo3::{
  Bound, Py, PyAny, PyResult, Python, create_exception,
  exceptions::{PyException, PyRuntimeError, PyTypeError},
  prelude::*,
  types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};

create_exception!(
  furb_monty._monty,
  Raised,
  PyException,
  "What the engine raised, as its name and what it was made with, which furb_monty.engine raises as the exception it is."
);

thread_local! {
  /// The one gate of this thread: one reading of the contract and of the typeshed, which every life and every
  /// Gate of the thread shares, so the reading costs once for the process and not once for each life.
  static GATE: Ty = Ty::new();
}

/// The callable of the host, which every ear of the life reaches through.
struct Hosted(Py<PyAny>);

impl Host for Hosted {
  fn called(&mut self, name: &str, said: &Value) -> Value {
    Python::attach(|py| {
      let got =
        to_python(py, &said.plain()).and_then(|one| self.0.bind(py).call1((name, one))).and_then(|one| of_python(&one));
      match got {
        Ok(value) => value,
        // A fault of the host, raised in the ear that spoke, since the host said nothing the ear could hear.
        Err(fault) => Value::Map(vec![(
          "raised".to_owned(),
          Value::Error {
            name: fault.get_type(py).name().map_or("Exception".to_owned(), |one| one.to_string()),
            args: vec![Value::Str(fault.to_string())],
          }
          .plain(),
        )]),
      }
    })
  }
}

/// One life: the engine in the sandbox, and the host it reaches.
#[pyclass(module = "furb_monty._monty", name = "Life", unsendable)]
pub struct Life {
  held: Held<Hosted>,
}

#[pymethods]
impl Life {
  /// A life, opened on the callable of the host, the names of its ears, and the record a World kept, plain.
  #[new]
  fn new(host: Py<PyAny>, ears: Vec<String>, record: Bound<'_, PyAny>) -> PyResult<Self> {
    let kept = of_python(&record)?;
    let held = Held::boot(Sand::default(), Hosted(host), &ears, &kept, GATE.with(Ty::clone)).map_err(raised)?;
    Ok(Life { held })
  }

  /// The root chain of the life, which is the first act of any record.
  #[getter]
  fn root(&self) -> &str {
    self.held.root()
  }

  /// What boot raised, if it raised, plain, which the engine of python raises as the exception it is.
  #[getter]
  fn raised<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    match self.held.raised() {
      Some(one) => to_python(py, &one.plain()),
      None => Ok(py.None().into_bound(py)),
    }
  }

  /// One word of the operator, run in the names of the engine, and what it gave, plain.
  fn word<'py>(&mut self, py: Python<'py>, word: &str) -> PyResult<Bound<'py, PyAny>> {
    let got = self.held.word(word).map_err(raised)?;
    to_python(py, &got.plain())
  }
}

/// The gate of the crate, for the Kernel of this interpreter to read a sheet with.
///
/// The one reading of the contract of this thread, which every life of monty shares too: what ty found on each
/// sheet, by its line.
#[pyclass(module = "furb_monty._monty", name = "Gate", unsendable)]
pub struct Gate {
  held: Ty,
}

#[pymethods]
impl Gate {
  #[new]
  fn new() -> Self {
    Gate { held: GATE.with(Ty::clone) }
  }

  /// What ty found on a sheet, each finding by its line, and none of the warnings.
  fn checked(&self, sheet: &str) -> Vec<(usize, String)> {
    self.held.checked(sheet)
  }
}

/// What the engine raised, as the exception a host reads: its name and what it was made with.
fn raised(why: Refusal) -> PyErr {
  match why {
    Refusal::Raised(Value::Error { name, args }) => Python::attach(|py| {
      let held: Vec<Py<PyAny>> =
        args.iter().filter_map(|one| to_python(py, &one.plain()).ok()).map(Bound::unbind).collect();
      Raised::new_err((name, held))
    }),
    other => PyRuntimeError::new_err(other.to_string()),
  }
}

/// One plain value, as python reads it.
fn to_python<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
  match value {
    Value::None => Ok(py.None().into_bound(py)),
    Value::Bool(one) => Ok(one.into_pyobject(py)?.to_owned().into_any()),
    Value::Int(one) => Ok(one.into_pyobject(py)?.into_any()),
    Value::Float(one) => Ok(one.into_pyobject(py)?.into_any()),
    Value::Str(one) => Ok(one.into_pyobject(py)?.into_any()),
    Value::List(each) | Value::Tuple(each) => {
      let held = PyList::empty(py);
      for one in each {
        held.append(to_python(py, one)?)?;
      }
      Ok(held.into_any())
    }
    Value::Map(each) => {
      let held = PyDict::new(py);
      for (name, one) in each {
        held.set_item(name, to_python(py, one)?)?;
      }
      Ok(held.into_any())
    }
    Value::Act(_) | Value::Name(_) | Value::Shape { .. } | Value::Error { .. } | Value::Show => {
      to_python(py, &value.plain())
    }
  }
}

/// One value of python, read as the plain form it must be.
///
/// A truth is read before a number, since python holds a truth as a number too. A tuple is read as a list, since
/// the plain form says a tuple by a mark and never by the type. Anything else is a fault of the host.
fn of_python(held: &Bound<'_, PyAny>) -> PyResult<Value> {
  if held.is_none() {
    return Ok(Value::None);
  }
  if let Ok(one) = held.cast::<PyBool>() {
    return Ok(Value::Bool(one.is_true()));
  }
  if let Ok(one) = held.cast::<PyInt>() {
    return Ok(Value::Int(one.extract()?));
  }
  if let Ok(one) = held.cast::<PyFloat>() {
    return Ok(Value::Float(one.extract()?));
  }
  if let Ok(one) = held.cast::<PyString>() {
    return Ok(Value::Str(one.extract()?));
  }
  if let Ok(each) = held.cast::<PyList>() {
    return each.iter().map(|one| of_python(&one)).collect::<PyResult<Vec<Value>>>().map(Value::List);
  }
  if let Ok(each) = held.cast::<PyTuple>() {
    return each.iter().map(|one| of_python(&one)).collect::<PyResult<Vec<Value>>>().map(Value::List);
  }
  if let Ok(each) = held.cast::<PyDict>() {
    let mut out = Vec::with_capacity(each.len());
    for (name, one) in each.iter() {
      out.push((name.extract::<String>()?, of_python(&one)?));
    }
    return Ok(Value::Map(out));
  }
  Err(PyTypeError::new_err(format!("the plain form holds no {}, and {held} is one", held.get_type().name()?)))
}

/// The module a host imports.
#[pymodule]
fn _monty(module: &Bound<'_, PyModule>) -> PyResult<()> {
  module.add_class::<Life>()?;
  module.add_class::<Gate>()?;
  module.add("Raised", module.py().get_type::<Raised>())?;
  Ok(())
}
