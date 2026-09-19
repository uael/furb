//! The crate as python reads it: the Kernel of the crate, given to a life of the engine of python.
//!
//! The engine takes the Kernel as a generator, which hears every fact and speaks by yielding one. [`Kernel`] is
//! that generator: it hears the facts of the engine, gates and runs the word of every rung in the sandbox, and
//! says what the sandbox gave.
//!
//! What this gives a life is the one thing the sandbox is for: the engine stays where it is, and the word of a
//! model, which nobody trusts, runs where it reaches the engine and nothing else.

use pyo3::{
  Bound, Py, PyAny, PyResult, Python,
  exceptions::PyStopIteration,
  prelude::*,
  types::{PyAnyMethods, PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};

use crate::{
  fact::{Fact, Value},
  kernel::Native,
  sandbox::{Awaited, Called, Held, Monty, Reach},
  world::{Kernel as KernelOf, Reply},
};

/// The engine of python, as the word of a model reaches it.
struct Engine {
  /// The module of the engine, which holds every verb and every name a word may say.
  module: Py<PyAny>,
  /// What the host holds for a word: a show a verb gave it, by the name the sandbox carries it under.
  ///
  /// Nothing of a callable can cross, so the word carries a name and the callable stays here, whole, and goes
  /// back to the engine as itself wherever the word hands it to a verb.
  holding: Py<PyDict>,
}

impl Engine {
  /// What the globals of a chain hold under a name, as python answers it.
  ///
  /// The module of a chain holds what the engine defines and what the chain binds of its own, so it answers
  /// first; the engine answers for a name no chain of the life has yet.
  fn look(&self, py: Python<'_>, chain: &str, name: &str) -> PyResult<Held> {
    let module = self.module.bind(py);
    let held = module
      .getattr("modules")
      .and_then(|held| held.get_item(chain))
      .and_then(|bound| bound.get_item(name));
    let got = match held {
      Ok(got) => got,
      Err(_) if module.hasattr(name)? => module.getattr(name)?,
      Err(_) => return Ok(Held::Nothing),
    };
    if got.is_callable() { Ok(Held::Verb) } else { Ok(Held::Is(of_py(&got)?)) }
  }
}

impl Reach for Engine {
  fn holds(&mut self, chain: &str, name: &str) -> Held {
    Python::attach(|py| self.look(py, chain, name).unwrap_or(Held::Nothing))
  }

  fn call(&mut self, rung: &str, chain: &str, name: &str, args: Vec<Value>, kwargs: Vec<(String, Value)>) -> Called {
    Python::attach(|py| {
      let said = self.calling(py, rung, chain, name, &args, &kwargs);
      match said {
        Ok(called) => called,
        Err(raised) => {
          let held = raised.value(py);
          Called::Raised { name: held.get_type().name().map_or_else(|_| "Exception".to_owned(), |n| n.to_string()), why: raised.to_string() }
        }
      }
    })
  }

  fn awaits(&mut self, rung: &str, act: &str) -> Awaited {
    Python::attach(|py| self.awaiting(py, rung, act).unwrap_or(Awaited::Waits))
  }

  fn binds(&mut self, chain: &str, held: Vec<(String, Value)>) {
    Python::attach(|py| {
      let module = self.module.bind(py);
      let Ok(bound) = module.getattr("modules").and_then(|held| held.get_item(chain)) else {
        return;
      };
      for (name, one) in held {
        if let Ok(value) = into_py(py, module, Some(self.holding.bind(py)), &one) {
          let _ = bound.set_item(name, value);
        }
      }
    });
  }

  fn field(&mut self, of: &Value, name: &str) -> Held {
    Python::attach(|py| {
      let held = into_py(py, self.module.bind(py), Some(self.holding.bind(py)), of).and_then(|made| {
        let bound = made.bind(py);
        if bound.hasattr(name)? { of_py(&bound.getattr(name)?).map(Held::Is) } else { Ok(Held::Nothing) }
      });
      held.unwrap_or(Held::Nothing)
    })
  }
}

impl Engine {
  /// What happens where a word awaits an act, which the engine itself says.
  ///
  /// The engine holds the laws of an await, so this asks the act, standing on the rung that waits: what the
  /// engine refuses it raises here, and what the life already holds under the name of the act it gives at once.
  fn awaiting(&self, py: Python<'_>, rung: &str, act: &str) -> PyResult<Awaited> {
    let module = self.module.bind(py);
    let site = module.getattr("site")?;
    let token = site.call_method1("set", (rung,))?;
    let asked = module.getattr("Act")?.call1((act,)).and_then(|one| one.call_method0("__await__"));
    let said = asked.and_then(|held| held.call_method0("__next__").map(|_| ()));
    site.call_method1("reset", (token,))?;
    if let Err(raised) = said
      && !raised.is_instance_of::<pyo3::exceptions::PyStopIteration>(py)
    {
      return Ok(Awaited::Refused(of_py(raised.value(py))?));
    }
    let outcomes = module.getattr("outcomes")?;
    match outcomes.get_item(act) {
      Ok(got) => Ok(Awaited::Over(of_py(&got)?)),
      Err(_) => Ok(Awaited::Waits),
    }
  }

  /// One call of a verb, made where the engine lives.
  ///
  /// A verb takes the chain of the rung when the call leaves it unsaid, and the word runs outside a rung as python
  /// reads it, so the chain of the run is said to every verb that takes one.
  fn calling(
    &self,
    py: Python<'_>,
    rung: &str,
    chain: &str,
    name: &str,
    args: &[Value],
    kwargs: &[(String, Value)],
  ) -> PyResult<Called> {
    let module = self.module.bind(py);
    let table = self.holding.bind(py);
    let verb = module.getattr(name)?;
    // What the word says is said by the rung that runs it, which is what the site of the engine holds.
    let site = module.getattr("site")?;
    let token = site.call_method1("set", (rung,))?;
    let held: Vec<Py<PyAny>> = args.iter().map(|one| into_py(py, module, Some(table), one)).collect::<PyResult<_>>()?;
    let named = PyDict::new(py);
    for (key, one) in kwargs {
      named.set_item(key, into_py(py, module, Some(table), one)?)?;
    }
    if takes_a_chain(py, &verb)? && !named.contains("on")? {
      named.set_item("on", chain)?;
    }
    let got = verb.call(PyTuple::new(py, held)?, Some(&named));
    site.call_method1("reset", (token,))?;
    let got = got?;
    if is_an_act(py, module, &got)? {
      return Ok(Called::Act(got.str()?.to_string()));
    }
    if got.is_callable() {
      return Ok(Called::Gave(self.holds_one(py, &got)?));
    }
    Ok(Called::Gave(of_py(&got)?))
  }

  /// A show the host holds for a word, under a name of its own, since nothing of a callable can cross.
  fn holds_one(&self, py: Python<'_>, got: &Bound<'_, PyAny>) -> PyResult<Value> {
    let table = self.holding.bind(py);
    let name = format!("held.{}", table.len() + 1);
    table.set_item(&name, got)?;
    Ok(Value::Held(name))
  }
}

/// Whether a verb takes the chain it is called on, which its signature says.
fn takes_a_chain(py: Python<'_>, verb: &Bound<'_, PyAny>) -> PyResult<bool> {
  let code = match verb.getattr("__code__") {
    Ok(code) => code,
    Err(_) => return Ok(false),
  };
  let names: Vec<String> = code.getattr("co_varnames")?.extract()?;
  let _ = py;
  Ok(names.iter().any(|one| one == "on"))
}

/// Whether what a verb gave is the name of an act, which is what a caller holds of one.
fn is_an_act(py: Python<'_>, module: &Bound<'_, PyAny>, got: &Bound<'_, PyAny>) -> PyResult<bool> {
  let _ = py;
  let kind = module.getattr("Act")?;
  got.is_instance(&kind)
}

/// One value of python, as the sandbox reads it.
pub fn of_py(any: &Bound<'_, PyAny>) -> PyResult<Value> {
  if any.is_none() {
    return Ok(Value::None);
  }
  if let Ok(held) = any.cast::<PyBool>() {
    return Ok(Value::Bool(held.is_true()));
  }
  if let Ok(held) = any.cast::<PyInt>() {
    return Ok(Value::Int(held.extract()?));
  }
  if let Ok(held) = any.cast::<PyFloat>() {
    return Ok(Value::Float(held.extract()?));
  }
  if let Ok(held) = any.cast::<PyString>() {
    return Ok(Value::Str(held.extract()?));
  }
  if let Ok(held) = any.cast::<PyTuple>() {
    return Ok(Value::Tuple(held.iter().map(|one| of_py(&one)).collect::<PyResult<_>>()?));
  }
  if let Ok(held) = any.cast::<PyList>() {
    return Ok(Value::List(held.iter().map(|one| of_py(&one)).collect::<PyResult<_>>()?));
  }
  if let Ok(held) = any.cast::<PyDict>() {
    let mut pairs = Vec::new();
    for (key, one) in held.iter() {
      pairs.push((key.str()?.to_string(), of_py(&one)?));
    }
    return Ok(Value::Map(pairs));
  }
  if any.is_instance_of::<pyo3::exceptions::PyBaseException>() {
    let args: Vec<Value> = any
      .getattr("args")?
      .cast::<PyTuple>()?
      .iter()
      .map(|one| of_py(&one))
      .collect::<PyResult<_>>()?;
    let name = any.get_type().name()?.to_string();
    return Ok(Value::Error { name, args });
  }
  if let Ok(fields) = any.getattr("__dataclass_fields__") {
    let mut held = Vec::new();
    for key in fields.cast::<PyDict>()?.keys().iter() {
      let name = key.str()?.to_string();
      let one = any.getattr(name.as_str())?;
      held.push((name, of_py(&one)?));
    }
    return Ok(Value::Shape { name: any.get_type().name()?.to_string(), fields: held });
  }
  Ok(Value::Show)
}

/// One value of the sandbox, as python holds it.
pub fn into_py(
  py: Python<'_>,
  module: &Bound<'_, PyAny>,
  table: Option<&Bound<'_, PyDict>>,
  value: &Value,
) -> PyResult<Py<PyAny>> {
  match value {
    Value::None | Value::Show => Ok(py.None()),
    Value::Bool(held) => Ok(held.into_pyobject(py)?.to_owned().into_any().unbind()),
    Value::Int(held) => Ok(held.into_pyobject(py)?.into_any().unbind()),
    Value::Float(held) => Ok(held.into_pyobject(py)?.into_any().unbind()),
    Value::Str(held) => Ok(held.into_pyobject(py)?.into_any().unbind()),
    Value::List(held) => {
      let made: Vec<Py<PyAny>> = held.iter().map(|one| into_py(py, module, table, one)).collect::<PyResult<_>>()?;
      Ok(PyList::new(py, made)?.into_any().unbind())
    }
    Value::Tuple(held) => {
      let made: Vec<Py<PyAny>> = held.iter().map(|one| into_py(py, module, table, one)).collect::<PyResult<_>>()?;
      Ok(PyTuple::new(py, made)?.into_any().unbind())
    }
    Value::Map(held) => {
      let made = PyDict::new(py);
      for (key, one) in held {
        made.set_item(key, into_py(py, module, table, one)?)?;
      }
      Ok(made.into_any().unbind())
    }
    Value::Shape { name, fields } => {
      let kind = module.getattr(name.as_str())?;
      let named = PyDict::new(py);
      for (key, one) in fields {
        named.set_item(key, into_py(py, module, table, one)?)?;
      }
      Ok(kind.call((), Some(&named))?.unbind())
    }
    Value::Held(name) => {
      let found = table.and_then(|one| one.get_item(name).ok()).flatten();
      match found {
        Some(one) => Ok(one.unbind()),
        None if module.hasattr(name.as_str())? => Ok(module.getattr(name.as_str())?.unbind()),
        None => Ok(py.None()),
      }
    }
    Value::Error { name, args } => {
      let kind = builtin(py, module, name)?;
      let made: Vec<Py<PyAny>> = args.iter().map(|one| into_py(py, module, table, one)).collect::<PyResult<_>>()?;
      Ok(kind.call1(PyTuple::new(py, made)?)?.unbind())
    }
  }
}

/// The name of an exception, as the engine holds it or as the interpreter does.
fn builtin<'py>(py: Python<'py>, module: &Bound<'py, PyAny>, name: &str) -> PyResult<Bound<'py, PyAny>> {
  if module.hasattr(name)? {
    return module.getattr(name);
  }
  py.import("builtins")?.getattr(name)
}

/// The Kernel of the crate, as a life of python hears it.
///
/// It is given the module of the engine, and it is handed to boot under the name kernel. From then it hears every
/// fact of the life and runs the word of every rung in the sandbox.
#[pyclass(unsendable)]
pub struct Kernel {
  /// The Kernel over the sandbox of monty.
  native: Native<Monty<Engine>>,
  /// What the Kernel still has to say, in order.
  said: Vec<Fact>,
  /// Whether the last send gave a fact, so the next one carries what the life made of it.
  resuming: bool,
}

#[pymethods]
impl Kernel {
  /// A Kernel for one life, over the engine of that life.
  #[new]
  fn new(engine: &Bound<'_, PyAny>) -> Self {
    Kernel {
      native: Native::new(Monty::new(Engine {
        module: engine.clone().unbind(),
        holding: PyDict::new(engine.py()).unbind(),
      })),
      said: Vec::new(),
      resuming: false,
    }
  }

  /// A generator is its own iterator, and a life drives this one by sending it facts.
  fn __iter__(this: PyRef<'_, Self>) -> PyRef<'_, Self> {
    this
  }

  /// The next fact the Kernel says, and nothing when it has nothing to say.
  fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
    self.send(py, py.None().into_bound(py))
  }

  /// One fact, heard; and the next fact the Kernel says, or nothing.
  ///
  /// A life sends a fact, takes the fact this gives back, says it, and sends what it made. While this has more to
  /// say it says the next one, and when it has nothing left it gives nothing, which is where the life stops.
  fn send(&mut self, py: Python<'_>, a: Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    if self.resuming {
      // The life said what this gave it, and hands it back. The fact is the life's now.
      if let Some(next) = self.pop(py)? {
        return Ok(Some(next));
      }
      self.resuming = false;
      return Ok(None);
    }
    if !a.is_none() {
      let heard = of_py(&a)?;
      let fact = Fact(heard.as_entries().map(<[Value]>::to_vec).unwrap_or_default());
      match self.native.hears(&fact) {
        Reply::Say(held) => {
          for one in held {
            if one.by() == "kernel" {
              self.said.push(one);
            } else {
              // A ran and a wants are the rung's own, said under its name, as the site of the engine holds it.
              self.say(py, &one)?;
            }
          }
        }
        Reply::Ask { .. } | Reply::Nothing => {}
      }
    }
    if let Some(next) = self.pop(py)? {
      self.resuming = true;
      return Ok(Some(next));
    }
    Ok(None)
  }

  /// A generator that is closed says nothing more, and this one holds nothing that must be unwound.
  fn close(&mut self) {
    self.said.clear();
  }

  /// What a life throws into a generator it drives, which ends this one.
  fn throw(&mut self, py: Python<'_>, _raised: Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    let _ = py;
    Err(PyStopIteration::new_err(()))
  }
}

impl Kernel {
  /// The next fact the Kernel has to say, as python holds it.
  ///
  /// A fact a generator yields says its kind, the act it is about and its words: who says it is whoever is
  /// speaking, which the bus of the engine fills in.
  fn pop(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
    if self.said.is_empty() {
      return Ok(None);
    }
    let fact = self.said.remove(0);
    let module = self.native.sandbox.reach.module.clone_ref(py);
    let bound = module.bind(py);
    let mut held: Vec<Py<PyAny>> = vec![into_py(py, bound, None, &Value::Str(fact.kind().to_owned()))?];
    held.push(into_py(py, bound, None, &Value::Str(fact.about().to_owned()))?);
    for one in fact.words() {
      held.push(into_py(py, bound, None, one)?);
    }
    Ok(Some(PyTuple::new(py, held)?.into_any().unbind()))
  }

  /// One fact of a rung, said through the bus under the name of that rung.
  fn say(&mut self, py: Python<'_>, fact: &Fact) -> PyResult<()> {
    let module = self.native.sandbox.reach.module.clone_ref(py);
    let bound = module.bind(py);
    let mut held: Vec<Py<PyAny>> = vec![into_py(py, bound, None, &Value::Str(fact.kind().to_owned()))?];
    held.push(into_py(py, bound, None, &Value::Str(fact.about().to_owned()))?);
    for one in fact.words() {
      held.push(into_py(py, bound, None, one)?);
    }
    let named = PyDict::new(py);
    named.set_item("by", fact.by())?;
    bound.getattr("send")?.call(PyTuple::new(py, held)?, Some(&named))?;
    Ok(())
  }
}

/// The crate, as python imports it.
#[pymodule]
fn furbc(module: &Bound<'_, PyModule>) -> PyResult<()> {
  module.add_class::<Kernel>()?;
  module.add("ENGINE", crate::ENGINE)?;
  module.add("PREAMBLE", crate::PREAMBLE)?;
  Ok(())
}
