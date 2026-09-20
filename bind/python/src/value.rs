//! What crosses between a life and a host that is python.
//!
//! The crate says every value plainly, and python says most of them itself: none, a truth, a number, a text, a
//! list, a tuple and a map are the python ones. Two are not, so the module gives a shape for each: [`Shape`] is a
//! shape of the engine, such as a text or an exit, and [`Fault`] is an exception by its name and what it was made
//! with. A show carries nothing, so it crosses as the one mark [`SHOW`].
//!
//! Nothing of python but these crosses. A host that hands over anything else is told what it handed over, since a
//! value the engine cannot hold is a fault of the host and not of the life.

use furb::{Fact, Value};
use pyo3::{
  Bound, IntoPyObject, PyAny, PyResult, Python,
  exceptions::PyTypeError,
  prelude::*,
  types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};

/// A shape of the engine, by its name and its fields, which a text and an exit are.
///
/// The fields keep the order the shape declares them in, and each one is read by its name: `text.path` and
/// `text.content` are the fields of a text, and `shape.fields` is all of them at once.
#[pyclass(module = "furb_sand", name = "Shape", frozen, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
  /// The name of the shape, as the engine holds it.
  #[pyo3(get)]
  pub name: String,
  /// The fields, in the order the shape declares them.
  pub fields: Vec<(String, Value)>,
}

#[pymethods]
impl Shape {
  #[new]
  fn new(name: String, fields: Bound<'_, PyDict>) -> PyResult<Self> {
    let mut held = Vec::with_capacity(fields.len());
    for (key, value) in fields.iter() {
      held.push((key.extract::<String>()?, of_python(&value)?));
    }
    Ok(Shape { name, fields: held })
  }

  /// The fields of the shape, by name, in the order the shape declares them.
  #[getter]
  fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
    let held = PyDict::new(py);
    for (name, value) in &self.fields {
      held.set_item(name, to_python(py, value)?)?;
    }
    Ok(held)
  }

  /// One field of the shape, by its name, which is how a host reads a text.
  fn __getattr__<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    match self.fields.iter().find(|(held, _)| held == name) {
      Some((_, value)) => to_python(py, value),
      None => Err(pyo3::exceptions::PyAttributeError::new_err(format!("{} has no field {name}", self.name))),
    }
  }

  fn __repr__(&self) -> String {
    let each: Vec<String> = self.fields.iter().map(|(name, value)| format!("{name}={value:?}")).collect();
    format!("{}({})", self.name, each.join(", "))
  }

  fn __eq__(&self, other: &Self) -> bool {
    self == other
  }
}

/// An exception of the engine, by its name and what it was made with.
///
/// It is a value and not a raise: a fault the engine hands back is a thing a host reads, and only a fault of the
/// call a host made is raised in the host. `Refused` is the name of a call the engine will not make.
#[pyclass(module = "furb_sand", name = "Fault", frozen, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
  /// The name of the exception.
  #[pyo3(get)]
  pub name: String,
  /// What the exception was made with.
  pub args: Vec<Value>,
}

#[pymethods]
impl Fault {
  #[new]
  #[pyo3(signature = (name, *args))]
  fn new(name: String, args: Bound<'_, PyTuple>) -> PyResult<Self> {
    let mut held = Vec::with_capacity(args.len());
    for one in args.iter() {
      held.push(of_python(&one)?);
    }
    Ok(Fault { name, args: held })
  }

  /// What the exception was made with.
  #[getter]
  fn args<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
    let mut held = Vec::with_capacity(self.args.len());
    for one in &self.args {
      held.push(to_python(py, one)?);
    }
    PyTuple::new(py, held)
  }

  /// Whether the engine refused the call, which is the one fault a word of a model makes on purpose.
  fn refused(&self) -> bool {
    self.name == "Refused"
  }

  fn __repr__(&self) -> String {
    let each: Vec<String> = self.args.iter().map(|one| format!("{one:?}")).collect();
    format!("{}({})", self.name, each.join(", "))
  }

  fn __eq__(&self, other: &Self) -> bool {
    self == other
  }
}

/// The mark of a show, which carries nothing a host can read.
#[pyclass(module = "furb_sand", name = "Show", frozen, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Show;

#[pymethods]
impl Show {
  fn __repr__(&self) -> &'static str {
    "SHOW"
  }

  fn __eq__(&self, other: &Self) -> bool {
    self == other
  }
}

/// One fact of the engine: its kind, the act it is about, who said it, and its words.
#[pyclass(module = "furb_sand", name = "Fact", frozen, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyFact(pub Fact);

#[pymethods]
impl PyFact {
  #[new]
  #[pyo3(signature = (kind, about, by = String::new(), *words))]
  fn new(kind: String, about: String, by: String, words: Bound<'_, PyTuple>) -> PyResult<Self> {
    let mut held = Vec::with_capacity(words.len());
    for one in words.iter() {
      held.push(of_python(&one)?);
    }
    Ok(PyFact(Fact::new(kind, about, by, held)))
  }

  /// The kind of the fact, which is its first slot.
  #[getter]
  fn kind(&self) -> &str {
    self.0.kind()
  }

  /// The act the fact is about.
  #[getter]
  fn about(&self) -> &str {
    self.0.about()
  }

  /// Who said the fact: a rung, the operator, the World or the Kernel.
  #[getter]
  fn by(&self) -> &str {
    self.0.by()
  }

  /// The words of the fact, which are everything after who said it.
  #[getter]
  fn words<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
    let mut held = Vec::with_capacity(self.0.words().len());
    for one in self.0.words() {
      held.push(to_python(py, one)?);
    }
    PyTuple::new(py, held)
  }

  /// Whether the fact is a question, which its name says: a question is about itself, under its kind.
  #[getter]
  fn question(&self) -> bool {
    self.0.question()
  }

  /// The chain a question is on, which is its first word, and nothing for a fact that is no question.
  #[getter]
  fn on(&self) -> Option<&str> {
    self.0.on()
  }

  fn __repr__(&self) -> String {
    format!("Fact{:?}", self.0.0)
  }

  fn __eq__(&self, other: &Self) -> bool {
    self == other
  }
}

/// One value of the engine, as python reads it.
pub fn to_python<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
  match value {
    Value::None => Ok(py.None().into_bound(py)),
    Value::Bool(one) => Ok(one.into_pyobject(py)?.to_owned().into_any()),
    Value::Int(one) => Ok(one.into_pyobject(py)?.into_any()),
    Value::Float(one) => Ok(one.into_pyobject(py)?.into_any()),
    Value::Str(one) => Ok(one.into_pyobject(py)?.into_any()),
    Value::List(each) => {
      let held = PyList::empty(py);
      for one in each {
        held.append(to_python(py, one)?)?;
      }
      Ok(held.into_any())
    }
    Value::Tuple(each) => {
      let mut held = Vec::with_capacity(each.len());
      for one in each {
        held.push(to_python(py, one)?);
      }
      Ok(PyTuple::new(py, held)?.into_any())
    }
    Value::Map(each) => {
      let held = PyDict::new(py);
      for (name, one) in each {
        held.set_item(name, to_python(py, one)?)?;
      }
      Ok(held.into_any())
    }
    Value::Shape { name, fields } => {
      Ok(Shape { name: name.clone(), fields: fields.clone() }.into_pyobject(py)?.into_any())
    }
    Value::Error { name, args } => Ok(Fault { name: name.clone(), args: args.clone() }.into_pyobject(py)?.into_any()),
    Value::Show => Ok(Show.into_pyobject(py)?.into_any()),
  }
}

/// One value of python, as the engine holds it.
///
/// A truth is read before a number, since python holds a truth as a number too, and a tuple before a list, since
/// the engine reads the difference between them.
pub fn of_python(held: &Bound<'_, PyAny>) -> PyResult<Value> {
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
  if let Ok(each) = held.cast::<PyTuple>() {
    let mut out = Vec::with_capacity(each.len());
    for one in each.iter() {
      out.push(of_python(&one)?);
    }
    return Ok(Value::Tuple(out));
  }
  if let Ok(each) = held.cast::<PyList>() {
    let mut out = Vec::with_capacity(each.len());
    for one in each.iter() {
      out.push(of_python(&one)?);
    }
    return Ok(Value::List(out));
  }
  if let Ok(each) = held.cast::<PyDict>() {
    let mut out = Vec::with_capacity(each.len());
    for (name, one) in each.iter() {
      out.push((name.extract::<String>()?, of_python(&one)?));
    }
    return Ok(Value::Map(out));
  }
  if let Ok(one) = held.extract::<Shape>() {
    return Ok(Value::Shape { name: one.name, fields: one.fields });
  }
  if let Ok(one) = held.extract::<Fault>() {
    return Ok(Value::Error { name: one.name, args: one.args });
  }
  if held.extract::<Show>().is_ok() {
    return Ok(Value::Show);
  }
  Err(PyTypeError::new_err(format!("a life holds no {}, and {held} is one", held.get_type().name()?)))
}
