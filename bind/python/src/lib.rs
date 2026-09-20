//! furb for python: one life of the engine, in the sandbox, with a World you write in python.
//!
//! The crate runs the engine in monty and gives its surface to a host. This is that surface for a host of python:
//! [`Life`] is one life, a host writes a World and hands it over, and [`Voice`] is how the host says into the
//! life the work that finishes later.
//!
//! Nothing of the engine crosses: the word of a rung runs where the engine runs, and every value that crosses is
//! plain, which `value` is the shape of. The crate takes care of the Kernel, so a host writes the World alone.

mod outside;
mod value;

use std::{sync::Arc, time::Duration};

use furb::{Ears, Entry, Life as Held, Refusal, Sand, Voice as Says, record};
use pyo3::{
  Bound, Py, PyAny, PyResult, Python, create_exception, exceptions::PyException, prelude::*, wrap_pyfunction,
};

use crate::{
  outside::{Ask, Gates, Raised, Worlds},
  value::{Fault, PyFact, Shape, Show, of_python, to_python},
};

create_exception!(
  furb_sand,
  Refused,
  PyException,
  "What a life could not do: the exception the engine raised, or what the crate could not read of what it gave."
);

/// One life: the engine in the sandbox, and the outside it reaches.
///
/// A host opens one with a World of its own and drives it by calling words on it, the way an operator calls a
/// verb. What the host started and has not finished it says through its [`Voice`], and the life hears all of it
/// in the order it was said.
#[pyclass(module = "furb_sand", name = "Life", unsendable)]
pub struct Life {
  /// The life, which holds the sandbox, the World and the gate.
  held: Held<Worlds, Gates>,
  /// The first fault of the World, shared with it; see [`Raised`].
  world: Raised,
  /// The first fault of the gate, shared with it; see [`Raised`].
  gate: Raised,
}

#[pymethods]
impl Life {
  /// A life, opened on a World, a gate and what a World kept of the life before it.
  ///
  /// The Voice is the one the host says into; a life made without one hears nothing that finishes later, which
  /// is enough for a World that answers everything where it hears it.
  #[new]
  #[pyo3(signature = (world, gate = None, record = None, voice = None))]
  fn new(
    py: Python<'_>,
    world: Py<PyAny>,
    gate: Option<Py<PyAny>>,
    record: Option<Bound<'_, PyAny>>,
    voice: Option<Py<Voice>>,
  ) -> PyResult<Self> {
    let kept = match record {
      Some(held) => entries(&held)?,
      None => Vec::new(),
    };
    let ears = match &voice {
      Some(held) => held
        .borrow_mut(py)
        .ears
        .take()
        .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("a Voice is heard by one life"))?,
      None => Ears::made().1,
    };
    let (raised, gated): (Raised, Raised) = (Arc::default(), Arc::default());
    let worlds = Worlds::new(world, Arc::clone(&raised));
    let gates = Gates::new(gate, Arc::clone(&gated));
    let held = Held::boot(Sand::default(), worlds, gates, ears, &kept);
    // A fault of the host is read before the refusal of the life: a life that is opening on a World which raised
    // fails for want of what the World never said, and the fault is what a host must be told.
    caught(&raised, &gated)?;
    Ok(Life { held: opened(held)?, world: raised, gate: gated })
  }

  /// The root chain of the life, which is the first act of any record.
  #[getter]
  fn root(&self) -> &str {
    self.held.root()
  }

  /// One word of the operator, run on a chain, and what it gave.
  ///
  /// This is how a host works a life: a verb of the engine is called by running a word that calls it, which is
  /// what an operator does from python too.
  fn word<'py>(&mut self, py: Python<'py>, word: &str) -> PyResult<Bound<'py, PyAny>> {
    let got = self.held.word(word);
    self.caught()?;
    to_python(py, &raised(got)?)
  }

  /// Everything the host has said into its Voice, done in the life, in the order it was said.
  ///
  /// A command says what it wrote while it runs, a model answers an ask long after the ask was heard, and the
  /// operator answers a prompt whenever the operator answers it. This is the one door they come through.
  fn heard(&mut self) -> PyResult<usize> {
    let got = self.held.heard();
    self.caught()?;
    raised(got)
  }

  /// Wait until the host says something, or until this long has passed, and say whether anything waits.
  ///
  /// A life goes on when a fact is said in it, so while it waits for a model, a command or a person, nothing of
  /// it moves until its host speaks.
  fn waits(&mut self, py: Python<'_>, seconds: f64) -> bool {
    py.detach(|| self.held.waits(Duration::from_secs_f64(seconds.max(0.0))))
  }

  /// What an act came to, and nothing at all while it waits.
  ///
  /// A host says what it owes first, since a fact it is holding may be the very one that settles the act, and
  /// then asks. It never waits in here: how long to wait is the host's to decide.
  fn came<'py>(&mut self, py: Python<'py>, act: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    let got = self.held.came(act);
    self.caught()?;
    match raised(got)? {
      Some(value) => Ok(Some(to_python(py, &value)?)),
      None => Ok(None),
    }
  }

  /// The World of this life, which is the object the host handed over.
  #[getter]
  fn world(&self, py: Python<'_>) -> Py<PyAny> {
    self.held.world().held.clone_ref(py)
  }
}

impl Life {
  /// The first fault of this life's World or gate; see [`caught`].
  fn caught(&self) -> PyResult<()> {
    caught(&self.world, &self.gate)
  }
}

/// How a host says into a life when nothing asked it to.
///
/// A Voice may be carried anywhere the host does its work, including another thread, since what it says waits in
/// order until the life is ready to hear it. One is handed to a life when the life is opened, and the life hears
/// what it holds at every [`Life::heard`].
#[pyclass(module = "furb_sand", name = "Voice")]
pub struct Voice {
  /// What the host says into.
  says: Says,
  /// The Ears of this Voice, until the life it was made for takes them.
  ///
  /// A Voice is heard by one life: the life that is opened on it takes these, and a second life opened on the
  /// same Voice finds nothing and is refused.
  ears: Option<Ears>,
}

#[pymethods]
impl Voice {
  /// A Voice and the Ears that hear it, made together.
  #[new]
  fn new() -> Self {
    let (says, ears) = Ears::made();
    Voice { says, ears: Some(ears) }
  }

  /// One fact, said into the life.
  ///
  /// A word said after the life is gone is dropped, since there is nobody to hear it.
  fn fact(&self, one: &PyFact) {
    self.says.send(one.0.clone());
  }

  /// One act, closed with a value, which is how the work a host started answers the act that asked for it.
  fn close(&self, act: &str, value: Bound<'_, PyAny>) -> PyResult<()> {
    self.says.close(act, of_python(&value)?);
    Ok(())
  }

  /// One chain, paused, which stops it until the operator wakes it.
  fn pause(&self, chain: &str) {
    self.says.pause(chain);
  }

  /// Whether anything said into this Voice is still waiting to be heard.
  fn waiting(&self) -> bool {
    self.says.waiting()
  }
}

/// The first fault of the World or of the gate, raised now that the call they were in is over.
///
/// A fault of the host is the host's and not the life's, so it stands as it was raised, and a life that heard
/// nothing where the host raised goes on for whoever catches it.
fn caught(world: &Raised, gate: &Raised) -> PyResult<()> {
  for one in [world, gate] {
    if let Some(fault) = one.lock().ok().and_then(|mut held| held.take()) {
      return Err(fault);
    }
  }
  Ok(())
}

/// The line a World writes for one entry it was told to keep.
///
/// The record of a life is the lines a World kept, and a life is opened again on the text of them. What the
/// engine tells a World to keep is one entry, and this is the one line of it, so that every host writes the same
/// record and no host derives the shape of one for itself.
#[pyfunction]
fn line(entry: Bound<'_, PyAny>) -> PyResult<String> {
  let held = of_python(&entry)?;
  let each = held.as_entries().unwrap_or_default();
  let made = Entry {
    before: each.first().and_then(furb::Value::as_str).unwrap_or_default().to_owned(),
    fact: furb::Fact(each.get(1).and_then(furb::Value::as_entries).unwrap_or_default().to_vec()),
    answer: each.get(2).cloned(),
  };
  Ok(made.line())
}

/// The record a life is opened on, from what a host holds.
///
/// A host keeps a record as the lines a World writes, so it hands over the text of the file, and a host that
/// keeps none hands over nothing.
fn entries(held: &Bound<'_, PyAny>) -> PyResult<Vec<Entry>> {
  let text: String = held.extract()?;
  record::read(&text).map_err(|drift| Refused::new_err(drift.0))
}

/// A life that opened, or the refusal it did not open on.
fn opened(got: Result<Held<Worlds, Gates>, Refusal>) -> PyResult<Held<Worlds, Gates>> {
  got.map_err(refusal)
}

/// What a call of the engine gave, or the refusal it raised in the caller.
fn raised<T>(got: Result<T, Refusal>) -> PyResult<T> {
  got.map_err(refusal)
}

/// One refusal of the engine, as the exception a host catches.
fn refusal(why: Refusal) -> PyErr {
  Refused::new_err(why.to_string())
}

/// The module a host imports.
#[pymodule]
fn furb_sand(module: &Bound<'_, PyModule>) -> PyResult<()> {
  module.add_class::<Life>()?;
  module.add_class::<Voice>()?;
  module.add_class::<PyFact>()?;
  module.add_class::<Ask>()?;
  module.add_class::<Shape>()?;
  module.add_class::<Fault>()?;
  module.add_class::<Show>()?;
  module.add("Refused", module.py().get_type::<Refused>())?;
  module.add_function(wrap_pyfunction!(line, module)?)?;
  module.add("SHOW", Show)?;
  module.add("WORLD", furb::host::WORLD)?;
  Ok(())
}
