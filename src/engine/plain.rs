//! What a verb gave or an act came to, read as a value of rust, and an act to await.

use std::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use super::Engine;
use crate::{
  fact::Fact,
  value::{Exit, Fault, Given, Object, Text},
};

/// What a verb gave or an act came to, read as the value a host of rust is given.
pub trait Plain: Sized {
  /// The value, or why it is not one.
  fn plain(got: Object) -> Result<Self, Fault>;
}

impl Plain for Object {
  fn plain(got: Object) -> Result<Self, Fault> {
    Ok(got)
  }
}

impl Plain for () {
  fn plain(_: Object) -> Result<Self, Fault> {
    Ok(())
  }
}

impl Plain for String {
  fn plain(got: Object) -> Result<Self, Fault> {
    got
      .as_ref()
      .as_str()
      .map(str::to_owned)
      .ok_or_else(|| Fault::refused(format!("{} is no text", got.py_repr())))
  }
}

impl Plain for f64 {
  #[expect(
    clippy::cast_precision_loss,
    reason = "python reads an int as a float with the same loss"
  )]
  fn plain(got: Object) -> Result<Self, Fault> {
    let one = got.as_ref();
    one
      .as_float()
      .or_else(|| one.as_int().map(|n| n as f64))
      .ok_or_else(|| Fault::refused(format!("{} is no number", got.py_repr())))
  }
}

impl Plain for i64 {
  fn plain(got: Object) -> Result<Self, Fault> {
    got.as_ref().as_int().ok_or_else(|| Fault::refused(format!("{} is no int", got.py_repr())))
  }
}

/// A value read as a type of the crate that it makes again, or why it is none.
macro_rules! plain_of {
  ($($kind:ty => $what:literal),*) => {$(
    impl Plain for $kind {
      fn plain(got: Object) -> Result<Self, Fault> {
        let no = || Fault::refused(format!("{} is no {}", got.py_repr(), $what));
        <$kind>::of(got.as_ref()).ok_or_else(no)
      }
    }
  )*};
}

plain_of!(Text => "text", Exit => "exit", Fact => "fact");

impl<T: Plain> Plain for Option<T> {
  fn plain(got: Object) -> Result<Self, Fault> {
    if got.as_ref().is_none() { Ok(None) } else { T::plain(got).map(Some) }
  }
}

impl<T: Plain> Plain for Vec<T> {
  fn plain(got: Object) -> Result<Self, Fault> {
    let items = got
      .as_ref()
      .items()
      .ok_or_else(|| Fault::refused(format!("{} is no list", got.py_repr())))?;
    items.into_iter().map(|one| T::plain(one.to_owned())).collect()
  }
}

/// One act of an engine, awaited for what it comes to: `Act` of the contract, a name that is
/// awaitable.
///
/// It borrows the engine for as long as it is awaited, since awaiting it drives the engine: what
/// the ears say of their own accord is said into it until the act is done. An act that completed
/// with an exception gives that fault. Dropping it leaves the act living; the engine holds what it
/// comes to under its name.
pub struct Act<'a, T> {
  engine: &'a mut Engine,
  id: String,
  came: PhantomData<T>,
}

impl<'a, T> Act<'a, T> {
  /// The act of this name, to await.
  pub fn of(engine: &'a mut Engine, id: impl Into<String>) -> Self {
    Act { engine, id: id.into(), came: PhantomData }
  }

  /// The name of the act, which is what the engine knows it by.
  pub fn id(&self) -> &str {
    &self.id
  }
}

impl<T> std::fmt::Debug for Act<'_, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Act").field(&self.id).finish()
  }
}

impl<T: Plain + Unpin> Future for Act<'_, T> {
  type Output = Result<T, Fault>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    if let Err(fault) = this.engine.pump(cx.waker()) {
      return Poll::Ready(Err(fault));
    }
    match this.engine.outcome(&this.id) {
      Ok(Some(got)) => Poll::Ready(Fault::of(got.as_ref()).map_or_else(|| T::plain(got), Err)),
      Ok(None) => Poll::Pending,
      Err(fault) => Poll::Ready(Err(fault)),
    }
  }
}
