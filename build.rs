//! The build: the contract is read here, and every verb of it becomes a method of the engine and of
//! each door.
//!
//! [`verbs`] reads the contract `src/furb/engine.pyi` and writes the methods of the engine, the
//! structs of the defaults of its verbs, the methods of each door, and the constants. [`minify`]
//! reads the engine `src/furb/engine.py` and writes its minified text, which is the system prompt
//! of every model.

#![expect(clippy::expect_used, reason = "debt: an expect where a type can hold the invariant")]
#![expect(clippy::map_unwrap_or, reason = "debt: map then unwrap_or, where map_or serves")]
#![expect(
  clippy::semicolon_if_nothing_returned,
  reason = "debt: a unit expression with no semicolon"
)]
#![expect(clippy::too_many_lines, reason = "debt: a function longer than 60 lines")]

use std::{env, fs, path::Path};

/// Writes a formatted line to a `String`, which cannot fail.
macro_rules! out {
  ($to:expr, $($say:tt)*) => {{
    use ::std::fmt::Write as _;
    writeln!($to, $($say)*).unwrap_or_default()
  }};
}

#[path = "src/minify.rs"]
mod minify;
#[path = "build/verbs.rs"]
mod verbs;

fn main() {
  #[cfg(feature = "typescript")]
  napi_build::setup();
  println!("cargo::rerun-if-changed=src/furb/engine.pyi");
  println!("cargo::rerun-if-changed=src/furb/engine.py");
  let source =
    fs::read_to_string("src/furb/engine.pyi").expect("the contract is beside the engine");
  let all = verbs::verbs(&source);
  let out = env::var("OUT_DIR").expect("cargo gives the build a directory");
  let written = |name: &str, text: String| {
    fs::write(Path::new(&out).join(name), text).expect("the build writes")
  };
  written("constants.rs", verbs::constants(&source));
  written("methods.rs", verbs::methods(&all));
  written("verbs.rs", verbs::structs(&all));
  written("ts.rs", verbs::scripted(&all));
  written("py.rs", verbs::pythonic(&all));
  let engine = fs::read_to_string("src/furb/engine.py").expect("the engine is beside the contract");
  written(
    "system.py",
    minify::minified(&engine).expect("the minified engine is the program on disk"),
  );
}
