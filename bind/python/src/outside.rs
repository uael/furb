//! The World and the gate a host writes in python, as the crate reads them.
//!
//! A host of python writes an object with `hears` and, when it asks the engine anything, `answered`. The crate
//! takes a World and a gate of its own, so each of these holds the python object and stands in for it: every call
//! of the trait becomes one call of the object, and what the object gives back is read as a reply.
//!
//! The reply a host gives is what python already says: nothing at all for a fact it only hears, a list of facts
//! for what it says of one, and an [`Ask`] for a question it must put to the engine first.

use std::sync::{Arc, Mutex};

use furb::{Fact, Gate, Reply, World};
use pyo3::{Bound, Py, PyAny, PyResult, Python, exceptions::PyTypeError, prelude::*, types::PyList};

use crate::value::{PyFact, of_python, to_python};

/// A question of the engine, which a World answers a fact with when it must know something first.
///
/// The boundary puts the question and hands the answer back through `answered`, so a World never calls into a
/// life that stands waiting for it.
#[pyclass(module = "furb_sand", name = "Ask", frozen, from_py_object)]
#[derive(Debug, Clone)]
pub struct Ask {
  /// The kind of the question, such as `cwd` or `merged`.
  #[pyo3(get)]
  pub kind: String,
  /// The chain the question is on.
  #[pyo3(get)]
  pub on: String,
  /// The words of the question.
  pub words: Vec<furb::Value>,
}

#[pymethods]
impl Ask {
  #[new]
  #[pyo3(signature = (kind, on, *words))]
  fn new(kind: String, on: String, words: Bound<'_, pyo3::types::PyTuple>) -> PyResult<Self> {
    let mut held = Vec::with_capacity(words.len());
    for one in words.iter() {
      held.push(of_python(&one)?);
    }
    Ok(Ask { kind, on, words: held })
  }

  /// The words of the question.
  #[getter]
  fn words<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    let held = PyList::empty(py);
    for one in &self.words {
      held.append(to_python(py, one)?)?;
    }
    Ok(held)
  }

  fn __repr__(&self) -> String {
    format!("Ask({:?}, {:?}, {:?})", self.kind, self.on, self.words)
  }
}

/// One word, read in the names of the engine, which a World answers a fact with when it must read the life.
///
/// A World of python is a generator beside the engine, so it reads the engine where it answers. A World of a host
/// is not beside it, and this is how it reaches the same names. The boundary reads the word and hands the value
/// back through `answered`, so a World never calls into a life that stands waiting for it.
#[pyclass(module = "furb_sand", name = "Reads", frozen, from_py_object)]
#[derive(Debug, Clone)]
pub struct Reads {
  /// The word, read in the names of the engine.
  #[pyo3(get)]
  pub word: String,
}

#[pymethods]
impl Reads {
  #[new]
  fn new(word: String) -> Self {
    Reads { word }
  }

  fn __repr__(&self) -> String {
    format!("Reads({:?})", self.word)
  }
}

/// The first fault of a World or of a gate, held where the caller of the life reads it.
///
/// A life takes its World and gives it back only when it opened, so the fault of a World that raises while the
/// life is opening would be lost with it. The slot is shared with the caller instead, and the caller reads it
/// whether the life opened or not.
///
/// It is a lock and not a cell, because a life lets the interpreter go while it waits for its host, and nothing
/// it holds may be of one thread alone for that.
pub type Raised = Arc<Mutex<Option<PyErr>>>;

/// A World of python, as the crate reads one.
///
/// Every fault of the python object stands: a World that raises has said something a life cannot answer, and the
/// life stops with it rather than carrying on with a reply that was never given.
pub struct Worlds {
  /// The object the host wrote.
  pub held: Py<PyAny>,
  /// The first fault of the object, which the caller of the life raises once the call is over.
  raised: Raised,
}

impl Worlds {
  /// A World of the crate, over the object a host wrote, with the slot its faults are kept in.
  pub fn new(held: Py<PyAny>, raised: Raised) -> Self {
    Worlds { held, raised }
  }

  /// One call of the object, and the reply it gave.
  ///
  /// A fault is kept and answered with nothing, so that the life runs to the end of the call it is in and the
  /// caller hears the fault whole. A life that carried on would hear a reply the host never gave.
  fn calls(&mut self, name: &str, args: impl for<'py> FnOnce(Python<'py>) -> PyResult<Bound<'py, PyAny>>) -> Reply {
    if already(&self.raised) {
      return Reply::Nothing;
    }
    let got = Python::attach(|py| -> PyResult<Reply> {
      let one = args(py)?;
      let said = self.held.bind(py).call_method1(name, (one,))?;
      reply_of(&said)
    });
    match got {
      Ok(reply) => reply,
      Err(fault) => {
        keeps(&self.raised, fault);
        Reply::Nothing
      }
    }
  }
}

impl World for Worlds {
  fn hears(&mut self, fact: &Fact) -> Reply {
    let held = fact.clone();
    self.calls("hears", move |py| Ok(PyFact(held).into_pyobject(py)?.into_any()))
  }

  fn answered(&mut self, got: &furb::Value) -> Reply {
    let held = got.clone();
    self.calls("answered", move |py| to_python(py, &held))
  }
}

/// A gate of python, as the crate reads one.
///
/// A host that reads no python leaves it out, and the word then runs and raises where it stands. One that reads
/// python gives an object with `gate`, and the findings it gives are what the model is told.
pub struct Gates {
  /// The object the host wrote, and nothing for a host that gates no word.
  held: Option<Py<PyAny>>,
  /// The first fault of the object, which the caller of the life raises once the call is over.
  raised: Raised,
}

impl Gates {
  /// A gate of the crate, over the object a host wrote, or over nothing, with the slot its faults are kept in.
  pub fn new(held: Option<Py<PyAny>>, raised: Raised) -> Self {
    Gates { held, raised }
  }
}

impl Gate for Gates {
  fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String> {
    let Some(held) = self.held.as_ref() else { return Vec::new() };
    if already(&self.raised) {
      return Vec::new();
    }
    let got = Python::attach(|py| -> PyResult<Vec<String>> {
      held.bind(py).call_method1("gate", (word, ladder, shape))?.extract()
    });
    match got {
      Ok(found) => found,
      Err(fault) => {
        keeps(&self.raised, fault);
        Vec::new()
      }
    }
  }
}

/// Whether a slot holds a fault already, which is what stops a second call of an object that raised.
fn already(raised: &Raised) -> bool {
  raised.lock().is_ok_and(|one| one.is_some())
}

/// The first fault of an object, kept for the caller of the life. A later one is dropped: the first is the one
/// that says what went wrong, and every call after it was made on a host that had already raised.
fn keeps(raised: &Raised, fault: PyErr) {
  if let Ok(mut one) = raised.lock() {
    one.get_or_insert(fault);
  }
}

/// What a host said of a fact, as the crate reads it.
///
/// Nothing at all is nothing said, a list is the facts to say in that order, and an [`Ask`] is a question of the
/// engine. Anything else is a fault of the host, which is told what it gave.
fn reply_of(said: &Bound<'_, PyAny>) -> PyResult<Reply> {
  if said.is_none() {
    return Ok(Reply::Nothing);
  }
  if let Ok(one) = said.extract::<Ask>() {
    return Ok(Reply::Ask { kind: one.kind, on: one.on, words: one.words });
  }
  if let Ok(one) = said.extract::<Reads>() {
    return Ok(Reply::Reads(one.word));
  }
  if let Ok(one) = said.extract::<PyFact>() {
    return Ok(Reply::say(one.0));
  }
  if let Ok(each) = said.cast::<PyList>() {
    let mut held = Vec::with_capacity(each.len());
    for one in each.iter() {
      held.push(one.extract::<PyFact>()?.0);
    }
    return Ok(Reply::Say(held));
  }
  Err(PyTypeError::new_err(format!(
    "a World says nothing, a fact, a list of facts, an Ask or a Reads, and {said} is none"
  )))
}
