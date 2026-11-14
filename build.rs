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

use std::{env, fs, path::Path};

use ruff_python_ast::token::TokenKind;
use ruff_python_parser::parse_module;
use sha2::{Digest, Sha256};

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

/// What an image of a life depends on in the build, beside the texts of its life, as
/// `src/image.rs` says: the Rust that makes, holds or reads the half of the host of an image and
/// that the session calls, the python that the build puts in a session beside the engine, the
/// Kernel and the checkpointer of its life, and the view, whose acts a host keeps in its part of an
/// image. A folder stands for each file in it but the tests.
const IMAGED: [&str; 10] = [
  "src/engine.rs",
  "src/engine",
  "src/sand.rs",
  "src/ear.rs",
  "src/value.rs",
  "src/image.rs",
  "src/view.rs",
  "src/view",
  "src/furb/record.py",
  "src/furb/adapters.py",
];

/// The digest of what an image depends on in the build: each file of [`IMAGED`] under its path,
/// then the typed surface of the contract, by which a host calls the engine of a life and reads
/// what an ear answers. The key of an image names it, so no person raises a number when one of
/// them changes.
fn imaged(contract: &str) -> String {
  let mut hash = Sha256::new();
  for path in IMAGED {
    println!("cargo::rerun-if-changed={path}");
    let mut files = if Path::new(path).is_dir() {
      let folder =
        fs::read_dir(path).expect("the build reads each folder that an image depends on");
      // The tests of a module stand beside it, and no image depends on them.
      let paths = folder.map(|one| one.expect("the build reads the folder").path());
      paths.filter(|one| !one.to_string_lossy().ends_with(".test.rs")).collect()
    } else {
      vec![Path::new(path).to_owned()]
    };
    files.sort();
    for file in files {
      hash.update(file.to_string_lossy().replace('\\', "/"));
      hash.update(fs::read(&file).expect("the build reads each file that an image depends on"));
    }
  }
  hash.update(surface(contract));
  format!("{:x}", hash.finalize())
}

/// The typed surface of a contract: each logical line of it at its depth, but each docstring,
/// which is a string alone on its line. It holds no comment and no layout, so a sentence of the
/// contract that changes leaves it as it is.
fn surface(contract: &str) -> String {
  let parsed = parse_module(contract).expect("the contract parses");
  let lines = minify::logical(contract, parsed.tokens());
  let mut typed = String::new();
  for line in lines.iter().filter(|line| !matches!(line.parts[..], [(TokenKind::String, _)])) {
    out!(typed, "{} {}", line.depth, line.text());
  }
  typed
}

fn main() {
  #[cfg(feature = "typescript")]
  napi_build::setup();
  println!("cargo::rerun-if-changed=src/furb/engine.pyi");
  println!("cargo::rerun-if-changed=src/furb/engine.py");
  let source =
    fs::read_to_string("src/furb/engine.pyi").expect("the contract is beside the engine");
  println!("cargo::rustc-env=FURB_IMAGED={}", imaged(&source));
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
