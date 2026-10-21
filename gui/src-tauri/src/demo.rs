//! The demo life: a small project of notes, and models that answer from a script, so the app shows a whole life
//! and asks no model. It is a real life of the crate: the engine runs each word, the gate reads it, and the World runs
//! each command on this machine. Only the turns of the models come from the script below.

use std::{
  path::{Path, PathBuf},
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use furb::{
  Engine, Fault, Object,
  engine::verbs,
  life::{Answer, Opening},
  world::Told,
};
use serde_json::Value;

/// The files of the demo project: a small notes app, which the answers of the script change.
const FILES: [(&str, &str); 5] = [
  (
    "README.md",
    "# Fieldnotes\n\nA small place to keep ideas.\n\n- Capture a thought\n- Find it when it matters\n",
  ),
  (
    "package.json",
    "{\n  \"name\": \"fieldnotes\",\n  \"scripts\": { \"test\": \"bun test\" }\n}\n",
  ),
  (
    "src/capture.ts",
    "import { save } from \"./storage.ts\";\n\nexport const capture = (text: string) => save({ text, at: Date.now() });\n",
  ),
  (
    "src/search.ts",
    "import { load } from \"./storage.ts\";\n\nexport const search = (query: string) => load().filter((note) => note.text.includes(query));\n",
  ),
  (
    "src/storage.ts",
    "export interface Note {\n  text: string;\n  at: number;\n}\n\nconst notes: Note[] = [];\n\nexport const save = (note: Note) => notes.push(note);\nexport const load = () => notes;\n",
  ),
];

/// The models of the demo, which the catalog names and the script answers for.
pub const ROSTER: [&str; 3] = ["claude-cli:sonnet", "claude-cli:opus", "claude-cli:haiku"];

/// The task that the operator gives the demo.
pub const TASK: &str =
  "Add a search shortcut to Fieldnotes, with a test. Give each part to a worker of its own.";

/// A new directory for one demo, with the files of the project in it.
pub fn directory() -> Result<PathBuf, String> {
  let stamp =
    SystemTime::now().duration_since(UNIX_EPOCH).map(|one| one.as_nanos()).unwrap_or_default();
  let root = std::env::temp_dir().join(format!("furb-gui-{}-{stamp}", std::process::id()));
  let directory = root.join("fieldnotes");
  for (path, content) in FILES {
    let path = directory.join(path);
    std::fs::create_dir_all(path.parent().unwrap_or(&directory)).map_err(|no| no.to_string())?;
    std::fs::write(&path, content).map_err(|no| no.to_string())?;
  }
  Ok(directory)
}

/// The opening of the demo life in a directory: its record beside the project, no extension, and the script in the
/// place of the models. `pace` is how long a model thinks before it writes, which the captures make short.
pub fn opening(directory: &Path, pace: Duration) -> Opening {
  let config = directory.parent().unwrap_or(directory).join("config");
  Opening {
    directory: directory.display().to_string(),
    record: Some(directory.join("demo.jsonl").display().to_string()),
    extensions: Some(false),
    config: Some(config.display().to_string()),
    roster: Some(ROSTER.map(str::to_owned).to_vec()),
    answer: Some(answer(pace)),
    ..Opening::default()
  }
}

/// What the operator does first in the demo: a ceiling of two dollars on the root, and the task as a prompt.
pub fn seed(engine: &mut Engine, root: &str) -> Result<(), Fault> {
  let on = Some(root.to_owned());
  engine.grant(verbs::Grant { usd: Some(2.0), on: on.clone(), ..Default::default() })?;
  let with = verbs::Prompt { message: Some(TASK.to_owned()), on, ..Default::default() };
  engine.prompt(Object::string("str"), with)?;
  Ok(())
}

/// The script in the place of the models: it thinks for a while, writes its word a few characters at a time, and
/// gives the turn with what it read and what it cost.
fn answer(pace: Duration) -> Answer {
  Answer(Arc::new(move |request: Value, told: Told| {
    let (word, tokens) = scripted(&request);
    Box::pin(async move {
      told("", "I read what the chain told me, then I write the next step.");
      tokio::time::sleep(pace).await;
      let characters: Vec<char> = word.chars().collect();
      for piece in characters.chunks(14) {
        told(&piece.iter().collect::<String>(), "");
        tokio::time::sleep(pace / 24).await;
      }
      let cost = tokens as f64 * 3.0 / 1_000_000.0;
      let usage = Object::list([
        Object::int(tokens as i64),
        Object::int(180),
        Object::int((tokens / 2) as i64),
        Object::int(0),
        Object::float(cost),
      ]);
      Ok(Object::tuple([Object::string("assistant"), Object::string(word), usage, Object::none()]))
    })
  }))
}

/// The lanes of the demo, which the label of a chain says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
  Root,
  Shortcut,
  Review,
  Test,
  Docs,
  Readme,
  Changelog,
}

/// The lane of a request: the label of its chain, which the first label that the transcript binds names.
fn lane(text: &str) -> Lane {
  let label =
    text.split("_label = '").nth(1).and_then(|rest| rest.split('\'').next()).unwrap_or_default();
  match label {
    "w shortcut" => Lane::Shortcut,
    "w review" => Lane::Review,
    "w test" => Lane::Test,
    "w docs" => Lane::Docs,
    "w readme" => Lane::Readme,
    "w changelog" => Lane::Changelog,
    _ => Lane::Root,
  }
}

/// The word that the script answers a request with, and the tokens that the turn read: by the lane of its chain and
/// by how many turns the model gave on that chain before.
pub fn scripted(request: &Value) -> (String, u64) {
  let messages = request["messages"].as_array().cloned().unwrap_or_default();
  let text = request["messages"].to_string();
  let step = messages.iter().filter(|one| one["role"] == "assistant").count();
  let lane = lane(&text);
  let tokens = 14_000 + 21_000 * step as u64 + if lane == Lane::Root { 6_000 } else { 0 };
  if acknowledges(&text) {
    return ("close(None)".to_owned(), tokens);
  }
  let step = match lane {
    Lane::Root => {
      ["first = await wait", "parts = [await"].iter().filter(|one| text.contains(**one)).count()
    }
    _ => step,
  };
  (word(lane, step).to_owned(), tokens)
}

/// Whether the last advance of a request asks for the answer of an acknowledgment: a prompt whose message says
/// that an act is done.
fn acknowledges(text: &str) -> bool {
  let Some(at) = text.rfind("advance on prompt") else { return false };
  let id: String =
    text[at + "advance on ".len()..].chars().take_while(char::is_ascii_alphanumeric).collect();
  let message = format!("{id}_message = 'prompt");
  text.match_indices(&message).any(|(at, _)| {
    text[at + message.len()..].split('\'').next().is_some_and(|rest| rest.ends_with(" done"))
  })
}

/// The word of one step of a lane.
pub fn word(lane: Lane, step: usize) -> &'static str {
  match (lane, step) {
    (Lane::Root, 0) => ROOT_FIRST,
    (Lane::Root, 1) => ROOT_SECOND,
    (Lane::Shortcut, 0) => SHORTCUT_FENCED,
    (Lane::Shortcut, 1) => SHORTCUT,
    (Lane::Review, 0) => REVIEW,
    (Lane::Test, 0) => TEST_FIRST,
    (Lane::Test, 1) => TEST,
    (Lane::Docs, 0) => DOCS,
    (Lane::Readme, 0) => README,
    (Lane::Changelog, 0) => CHANGELOG,
    _ => "close(\"Done.\")",
  }
}

const ROOT_FIRST: &str = r#"notes = read("README.md")
shortcut = prompt(str, "Add SHORTCUT = 'Ctrl+K' to src/search.ts, and close with what you changed.", on=chain("w shortcut"))
test = prompt(str, "Write test/shortcut.test.ts for SHORTCUT, run it, and close with the result.", on=chain("w test"))
docs = prompt(str, "Tell the users of Fieldnotes about the search shortcut, and close with what you wrote.", on=chain("w docs"))
first = await wait(120, shortcut, test, docs)
debug(t"{first=}")"#;

const ROOT_SECOND: &str = r#"parts = [await shortcut, await docs, await test]
check = await bash("sleep 1; grep -c SHORTCUT src/search.ts test/shortcut.test.ts")
close("All parts landed.\n" + "\n".join(parts) + "\n" + check.stdout.content)"#;

const SHORTCUT_FENCED: &str = "```python\nsource = read(\"src/search.ts\")\n```";

const SHORTCUT: &str = r#"source = read("src/search.ts")
write(source.append('export const SHORTCUT = "Ctrl+K";'))
check = await bash("sleep 2; grep -n SHORTCUT src/search.ts")
review = await prompt(str, "Review the export of SHORTCUT in src/search.ts, and close with your verdict.", on=chain("w review"))
close("src/search.ts exports SHORTCUT at line " + check.stdout.content.split(":")[0] + ". " + review)"#;

const REVIEW: &str = r#"source = read("src/search.ts")
found = await bash("sleep 1; grep -c 'export const SHORTCUT' src/search.ts")
close("The review passes: one export of SHORTCUT, " + found.stdout.content.strip() + " line.")"#;

const TEST_FIRST: &str = r#"spec = read("test/shortcut.test.ts")"#;

const TEST: &str = r#"runner = await bash("sleep 1; printf 'bun test: ready\n'")
key = await prompt(str, "The test needs the key. Which key does SHORTCUT hold?", to=OPERATOR)
spec = 'import { expect, test } from "bun:test";\nimport { SHORTCUT } from "../src/search.ts";\n\ntest("shortcut", () => expect(SHORTCUT).toBe("' + key + '"));\n'
write(Text("test/shortcut.test.ts", spec))
run = await bash("sleep 3; printf '1 pass\n'")
close("test/shortcut.test.ts expects " + key + ": " + run.stdout.content.strip())"#;

const DOCS: &str = r#"readme = prompt(str, "Add a line on the search shortcut to README.md, and close with the line.", on=chain("w readme"))
changelog = prompt(str, "Start CHANGELOG.md with the search shortcut, and close with what you wrote.", on=chain("w changelog"))
lines = [await readme, await changelog]
close("The docs name the shortcut.\n" + "\n".join(lines))"#;

const README: &str = r#"notes = read("README.md")
write(notes.append("- Press Ctrl+K to search"))
close("README.md now says: Press Ctrl+K to search")"#;

const CHANGELOG: &str = r##"write(Text("CHANGELOG.md", "# Changelog\n\n- Add the search shortcut, Ctrl+K.\n"))
check = await bash("sleep 1; wc -l < CHANGELOG.md")
close("CHANGELOG.md holds " + check.stdout.content.strip() + " lines")"##;

#[cfg(test)]
#[path = "demo.test.rs"]
mod test;
