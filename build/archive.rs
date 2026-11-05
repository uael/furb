//! The archive of the engines of the major: each engine that a record may name, which the build
//! keeps as it shipped, by its digest, in `src/furb/engines`, and gives with the adapters that make
//! its replay exact. `script/engines.py` fills the archive from git.

use std::{fs, path::Path};

/// A fix of a drift of the replay of old engines, which their text takes at the build: each
/// replacement of it stands once in the text of each engine that it names by the head of its
/// digest, and the build stops otherwise.
struct Adapter {
  engines: &'static [&'static str],
  replaces: &'static [(&'static str, &'static str)],
}

/// The adapters, in the order the build applies them.
///
/// A raise of an ear while the journal says a fact of the record: the live life heard that fact
/// from an ear of the outside, so the raise went to the host, which went on, and the bus held what
/// it still had to deliver until the next fact. The replay of these engines let the raise end the
/// journal, and the life drifted at that fact. With the adapter, the bus of a replay holds what it
/// still has to deliver until the journal says the next fact of the record, as the live life did.
const ADAPTERS: &[Adapter] = &[Adapter {
  engines: &[
    "c4dba9e9fafbc880",
    "898440b48887c7b9",
    "3a260b19a45e1c4e",
    "5fe06c38cc187a9e",
    "a0781e20ed2c8905",
    "1739d93dd59235d5",
  ],
  replaces: &[
    (
      "  saved, again = {e[1]: e for (e,) in record if question(e)}, {}\n",
      "  saved, again = {e[1]: e for (e,) in record if question(e)}, {}\n  stalled = []\n",
    ),
    (
      "        while owed() and (\n          not (log or left)\n",
      "        while owed() and (\n          not (log or left)\n          or (stalled and stalled.pop())\n",
    ),
    (
      "      if left:\n        hear(*left.pop(0))\n      elif log:",
      "      if left and not stalled:\n        try:\n          hear(*left.pop(0))\n        except Exception as fault:\n          \
       if isinstance(fault, Drift) or not owed():\n            raise\n          stalled.append(fault)\n      elif log and not stalled:",
    ),
    (
      "      elif ((g := alive.get(\"journal\")) and hear(\"journal\", g, None)) or not log:\n        return",
      "      elif ((g := alive.get(\"journal\")) and hear(\"journal\", g, None)) or not (\n        log or (stalled and stalled.pop())\n      ):\n        return",
    ),
  ],
}];

/// The text of an archived engine with each adapter that names its digest applied.
fn adapted(digest: &str, mut engine: String) -> String {
  for adapter in
    ADAPTERS.iter().filter(|one| one.engines.iter().any(|head| digest.starts_with(head)))
  {
    for (from, to) in adapter.replaces {
      assert_eq!(engine.matches(from).count(), 1, "the engine {digest} holds {from:?} once");
      engine = engine.replacen(from, to, 1);
    }
  }
  engine
}

/// The table of the archive as rust: one `Build` for each folder of the archive, in the order of
/// their digests, whose engine and system prompt the build writes in `out` from the text that the
/// adapters made.
pub(crate) fn archived(root: &Path, out: &Path) -> String {
  let folder = root.join("src").join("furb").join("engines");
  let mut digests: Vec<String> = fs::read_dir(&folder)
    .map(|all| all.filter_map(Result::ok).filter(|one| one.path().is_dir()))
    .into_iter()
    .flatten()
    .map(|one| one.file_name().to_string_lossy().into_owned())
    .collect();
  digests.sort();
  let mut table = String::from("&[\n");
  for digest in digests {
    let at = folder.join(&digest);
    let engine =
      fs::read_to_string(at.join("engine.py")).expect("each engine of the archive is a file");
    let engine = adapted(&digest, engine);
    let system = super::minify::minified(&engine);
    let written = |name: &str, text: &str| {
      let path = out.join(name);
      fs::write(&path, text).expect("the build writes");
      path.display().to_string()
    };
    let engine = written(&format!("engine-{digest}.py"), &engine);
    let system = written(&format!("system-{digest}.py"), &system);
    let kernel = at.join("kernel.py").display().to_string();
    // A folder that goes leaves no file newer than the build, so each file of the archive is named.
    println!("cargo::rerun-if-changed={}", at.join("engine.py").display());
    println!("cargo::rerun-if-changed={kernel}");
    out!(
      table,
      "  Build {{ digest: {digest:?}, engine: include_str!({engine:?}), kernel: include_str!({kernel:?}), \
       system: include_str!({system:?}) }},"
    );
  }
  table.push(']');
  table
}
