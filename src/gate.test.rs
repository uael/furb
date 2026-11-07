use std::collections::HashSet;

use monty_types::NamedValues;

use super::{
  MODULE, checked,
  layout::{CHUNK, laid, program_lines, shortened},
  shallow::{DEEP, shallowed},
};
use crate::{
  ENGINE, SHEET as SOURCE,
  sand::{Nobody, Sand},
  value::{Object, entry},
};

/// The sheet of one word after a program, and how many lines stand above the word, as `furb.sheet`
/// writes it in the sandbox from the module of the engine, made as a life makes it.
fn sheet(program: &[&str], word: &str) -> (String, usize) {
  acted(program, &[], word)
}

/// The sheet of one word after a program, on a chain whose acts have the names it is given. Each
/// word of the program is of the rung `rung<n>`, its place in the program from 1.
fn acted(program: &[&str], acts: &[&str], word: &str) -> (String, usize) {
  let code = "__engine = {}\nexec(__source, __engine)\n__sheet = {}\nexec(__sheet_source, __sheet)\n__program = {f'rung{n}': one for n, one in enumerate(__words, 1)}\n__sheet['sheet'](__engine, __program, __word, __acts)";
  let mut named = NamedValues::new();
  named.push("__source", Object::string(ENGINE));
  named.push("__sheet_source", Object::string(SOURCE));
  named.push("__words", Object::list(program.iter().map(|one| Object::string(*one))));
  named.push("__word", Object::string(word));
  named.push("__acts", Object::list(acts.iter().map(|one| Object::string(*one))));
  let got =
    Sand::new().run(code, named, &mut Nobody("the sheet")).expect("furb.sheet writes the sheet");
  let got = got.as_ref();
  let text = entry(&got, 0).and_then(|one| one.as_str()).expect("a sheet is a text").to_owned();
  let above =
    entry(&got, 1).and_then(|one| one.as_int()).expect("a sheet counts the lines above the word");
  (text, usize::try_from(above).expect("a count of lines is no negative number"))
}

/// What the gate finds below a word that answers the step of a prompt of these shapes, which is a
/// prompt that wants a value when it is given one, each finding by its line in the word.
fn answering(word: &str, valued: &[&str]) -> Vec<(usize, String)> {
  let code = "__engine = {}\nexec(__source, __engine)\n__sheet = {}\nexec(__sheet_source, __sheet)\n__sheet['sheet'](__engine, {}, __word, [], __valued)";
  let mut named = NamedValues::new();
  named.push("__source", Object::string(ENGINE));
  named.push("__sheet_source", Object::string(SOURCE));
  named.push("__word", Object::string(word));
  named.push("__valued", Object::list(valued.iter().map(|one| Object::string(*one))));
  let got =
    Sand::new().run(code, named, &mut Nobody("the sheet")).expect("furb.sheet writes the sheet");
  let got = got.as_ref();
  let text = entry(&got, 0).and_then(|one| one.as_str()).expect("a sheet is a text").to_owned();
  let above = entry(&got, 1).and_then(|one| one.as_int()).expect("a sheet counts its lines");
  let above = usize::try_from(above).expect("a count of lines is no negative number");
  found(&text)
    .into_iter()
    .filter(|(line, _)| *line > above)
    .map(|(line, why)| (line - above, why))
    .collect()
}

#[test]
fn a_word_that_answers_a_prompt_with_a_value_by_a_bare_expression_is_refused_with_its_close() {
  let shape = ["tuple[list[str], str]"];
  let found = answering("ids = ['chain1']\n(ids, 'kept')", &shape);
  assert_eq!(found.len(), 1, "{found:?}");
  assert_eq!(found[0].0, 2, "{found:?}");
  assert!(
    found[0].1.contains("[bare-answer]") && found[0].1.contains("close((ids, 'kept'))"),
    "{found:?}"
  );
  // A word whose last line is a call, or an await, closes or waits as it says, and a prompt that
  // wants no value takes a bare expression as any word.
  for (word, valued) in [
    ("ids = ['chain1']\nclose((ids, 'kept'))", &shape[..]),
    ("got = bash('ls')\nawait got", &shape[..]),
    ("ids = ['chain1']\n(ids, 'kept')", &[][..]),
  ] {
    let found = answering(word, valued);
    assert!(!found.iter().any(|one| one.1.contains("bare-answer")), "{word}: {found:?}");
  }
}

fn found(text: &str) -> Vec<(usize, String)> {
  let found = checked(text).expect("the gate reads the sheet");
  found.into_iter().map(|one| (one.line, one.to_string())).collect()
}

/// What the checker found below the word, each by its line in the word.
fn said(program: &[&str], word: &str) -> Vec<(usize, String)> {
  let (text, above) = sheet(program, word);
  found(&text)
    .into_iter()
    .filter(|(line, _)| *line > above)
    .map(|(line, why)| (line - above, why))
    .collect()
}

#[test]
fn the_sheet_imports_the_engine_under_the_name_the_gate_gives_it() {
  assert!(sheet(&[], "close(1)").0.starts_with(&format!("import {MODULE}\n")));
}

#[test]
fn the_names_of_the_engine_bound_from_its_module_give_no_finding_of_their_own() {
  let (text, above) = sheet(&["k = 1", "x = read('a')"], "close(k)");
  assert_eq!(
    found(&text),
    Vec::<(usize, String)>::new(),
    "the engine reads clean on the typeshed of the sandbox"
  );
  assert_eq!(text.lines().count(), above + 1);
}

#[test]
fn an_import_the_sandbox_cannot_run_is_a_finding_of_its_line() {
  // The typeshed of monty holds `types` for the annotations of `typing`, and the sandbox runs no
  // `types`.
  let found = said(&["k = 1"], "k = 2\nimport types\nclose(types.SimpleNamespace(k=k))");
  assert_eq!(
    found,
    vec![(
      2,
      "error[unresolved-import] (column 3) ModuleNotFoundError: No module named 'types'".to_owned()
    )]
  );
}

#[test]
fn a_name_the_module_of_the_sandbox_lacks_is_a_finding_of_its_import() {
  let found = said(&[], "from typing import NoSuchName\nclose(1)");
  assert_eq!(found.len(), 1, "{found:?}");
  assert_eq!(found[0].0, 1);
  assert!(found[0].1.starts_with("error[unresolved-import] (column 3) ImportError: "), "{found:?}");
}

#[test]
fn an_import_the_sandbox_runs_is_no_finding_though_the_typeshed_lacks_it() {
  assert_eq!(said(&[], "import ast\nclose(ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)"), vec![]);
}

#[test]
fn a_word_that_reads_the_list_of_filter_as_an_iterator_is_refused() {
  // The sandbox gives a list for filter, map, zip, enumerate and reversed, where python gives an
  // iterator.
  let found = said(&[], "got = next(filter(None, [1]))\nclose(got)");
  assert_eq!(found.iter().map(|(line, _)| *line).collect::<Vec<_>>(), vec![1], "{found:?}");
  assert!(found[0].1.starts_with("error[invalid-argument-type]"), "{found:?}");
}

#[test]
fn a_word_that_imports_the_engine_by_the_name_of_the_checker_is_refused() {
  let found = said(&[], &format!("import {MODULE}\nclose(1)"));
  assert_eq!(
    found,
    vec![(
      1,
      format!(
        "error[unresolved-import] (column 3) ModuleNotFoundError: No module named '{MODULE}'"
      )
    )]
  );
}

#[test]
fn a_read_of_asyncio_in_the_word_is_a_finding_of_its_line_but_its_cancelled_error() {
  let found = said(
    &[],
    "import asyncio as aio\nfrom asyncio import gather, CancelledError\ntry:\n  await aio.sleep(1)\nexcept aio.CancelledError:\n  pass",
  );
  let why = |column: usize, name: &str| {
    format!(
      "error[asyncio-in-word] (column {column}) `asyncio.{name}` cannot run in a word, which runs in a call of the host: await wait(seconds) to pause, and await each act by itself"
    )
  };
  assert_eq!(found, vec![(2, why(23, "gather")), (4, why(11, "sleep"))]);
}

#[test]
fn an_await_in_a_function_that_is_not_async_is_a_finding_of_its_line() {
  let found = said(&[], "def fix():\n  got = await wait(0, __name__)\n  return got\nfix()");
  let why = "`await` stands in `def fix`, which is not async: write `async def fix`, and await each call of it";
  assert_eq!(found, vec![(2, format!("error[await-outside-async] (column 11) {why}"))]);
}

#[test]
fn an_await_in_a_function_that_is_not_async_in_an_async_one_is_a_finding() {
  let found = said(
    &[],
    "async def outer():\n  def inner():\n    return await wait(0, __name__)\n  return inner",
  );
  assert_eq!(found.iter().map(|(line, _)| *line).collect::<Vec<_>>(), vec![3], "{found:?}");
  assert!(found[0].1.contains("`def inner`"), "{found:?}");
}

#[test]
fn an_await_in_an_async_function_or_at_the_top_of_the_word_is_no_finding() {
  let word = "async def fix():\n  return await wait(0, __name__)\nawait fix()\nawait wait(0, __name__)\nclose(1)";
  assert_eq!(said(&[], word), vec![]);
}

#[test]
fn a_call_of_an_async_function_that_nothing_awaits_is_a_finding_of_its_line() {
  let why = |column: usize| {
    format!(
      "error[unawaited-coroutine] (column {column}) `hire` is an async function, which runs nothing until it is awaited: write `await hire(...)`"
    )
  };
  assert_eq!(said(&[], "async def hire():\n  return 1\nhire()"), vec![(3, why(3))]);
  assert_eq!(said(&["async def hire():\n  return 1"], "hire()"), vec![(1, why(3))]);
  assert_eq!(
    said(&["async def hire():\n  return 1"], "async def go():\n  hire()\nawait go()"),
    vec![(2, why(5))]
  );
}

#[test]
fn a_call_of_an_async_function_in_an_argument_a_container_or_a_return_is_a_finding() {
  let hire = "async def hire():\n  return 1";
  let found =
    |word: &str| said(&[hire], word).into_iter().map(|(line, _)| line).collect::<Vec<_>>();
  assert_eq!(found("close(hire())"), vec![1]);
  assert_eq!(found("x = [hire(), 2]"), vec![1]);
  assert_eq!(found("x = {'a': hire()}"), vec![1]);
  assert_eq!(found("async def go():\n  return hire()\nawait go()"), vec![2]);
  assert_eq!(found("close(await hire())\ny = (hire() or 1)"), vec![2]);
  assert_eq!(found("close(await hire())\ng = hire()\nh = (k := hire())"), Vec::<usize>::new());
  assert_eq!(
    found("async def g():\n  yield 1\nasync for x in g():\n  close([y async for y in g()])"),
    Vec::<usize>::new()
  );
}

#[test]
fn a_call_that_is_awaited_or_bound_or_of_another_function_is_no_finding() {
  let hire = "async def hire():\n  return 1";
  assert_eq!(said(&[hire], "await hire()\ngot = hire()\nclose(await got)"), vec![]);
  assert_eq!(said(&[hire, "def hire():\n  return 1"], "hire()"), vec![]);
  assert_eq!(said(&[hire], "def go(hire):\n  hire()\ngo(len)"), vec![]);
  assert_eq!(said(&[], "class K:\n  async def read(self):\n    return 1\nread('a')"), vec![]);
}

#[test]
fn a_call_of_an_async_function_that_a_full_module_of_the_program_declares_is_a_finding() {
  let mut program = vec!["async def hire():\n  return 1".to_owned()];
  program.extend((0..2 * CHUNK).map(|n| format!("k = {n}")));
  let mut program: Vec<&str> = program.iter().map(String::as_str).collect();
  let found = said(&program, "hire()");
  assert_eq!(found.len(), 1, "{found:?}");
  assert!(found[0].1.starts_with("error[unawaited-coroutine] (column 3)"), "{found:?}");
  program.push("hire = 1");
  assert_eq!(said(&program, "hire()"), vec![]);
}

#[test]
fn a_name_bound_again_has_the_type_of_its_last_binding() {
  assert_eq!(
    said(&["x = 1", "def x() -> int:\n  return 1"], "y: int = x()\ndef z() -> int:\n  return x()"),
    vec![]
  );
  let found = said(&["x = ''", "x = x + 'a'\nx = 1"], "y: str = x");
  assert_eq!(found.len(), 1, "{found:?}");
  assert!(found[0].1.contains("`Literal[1]` is not assignable to `str`"), "{found:?}");
}

#[test]
fn a_name_that_a_word_deletes_falls_back_to_its_builtin_or_to_no_binding() {
  assert_eq!(said(&["sum = bash('ls')", "del sum"], "close(sum([1, 2]))"), vec![]);
  assert_eq!(said(&["sum = bash('ls')", "sum = 1\ndel sum"], "close(sum([1, 2]))"), vec![]);
  let found = said(&["sum = 1", "del sum\nsum = 'a'"], "y: int = sum");
  assert_eq!(found.len(), 1, "{found:?}");
  assert!(found[0].1.contains("`Literal[\"a\"]` is not assignable to `int`"), "{found:?}");
  let found = said(&["x = 1", "del x"], "close(x)");
  assert_eq!(found.len(), 1, "{found:?}");
  assert!(found[0].1.starts_with("error[unresolved-reference]"), "{found:?}");
  let mut program = vec!["sum = bash('ls')".to_owned(), "del sum".to_owned()];
  program.extend((0..2 * CHUNK).map(|n| format!("k = {n}")));
  let program: Vec<&str> = program.iter().map(String::as_str).collect();
  assert_eq!(said(&program, "close(sum([1, 2]))"), vec![]);
}

#[test]
fn a_name_a_nested_statement_binds_again_keeps_each_type() {
  let found = said(&["x = 1", "if chance() > 0.5:\n  def x() -> int:\n    return 1"], "x()");
  assert_eq!(found.len(), 1, "{found:?}");
  assert!(found[0].1.starts_with("error[call-non-callable]"), "{found:?}");
}

#[test]
fn a_name_that_the_transcript_binds_is_a_finding_of_the_line_that_binds_it() {
  let word = "x = bash1\nbash1_stdout = 'made up'\nfor prompt2 in []:\n  pass\ndef f(bash12: int) -> int:\n  return bash12\nprompt21 = 2\nimport json as prompt2_value\nclose(x)";
  let (text, above) = acted(&["bash1 = 1"], &["bash1", "prompt2"], word);
  let got: Vec<_> = found(&text)
    .into_iter()
    .filter(|(line, _)| *line > above)
    .map(|(line, why)| (line - above, why))
    .collect();
  let told = |column: usize, name: &str| {
    format!(
      "error[transcript-name] (column {column}) the transcript binds `{name}`: a word reads it and never binds it"
    )
  };
  assert_eq!(
    got,
    vec![(2, told(3, "bash1_stdout")), (3, told(7, "prompt2")), (8, told(3, "prompt2_value"))]
  );
}

#[test]
fn a_word_that_binds_a_name_of_the_engine_at_its_top_is_a_finding_and_a_def_of_one_overrides_it() {
  let word = "wait = 5\nfor chance in []:\n  pass\nimport json as cd\ndel turns\ndef read(path: str) -> str:\n  return path\ndef f(grep: int) -> int:\n  return grep\nactor = 'm/low'\naccepts = None\nraised = None\nk = [over for over in ()]\nimport re, json as wait_\nfrom json import loads as re\nCancelledError = 5\ntry:\n  scope = 1\nexcept Refused as span:\n  pass\nclose(1)";
  let (text, above) = acted(&[], &[], word);
  let got: Vec<_> = found(&text)
    .into_iter()
    .filter(|(line, why)| *line > above && why.starts_with("error[engine-name]"))
    .map(|(line, why)| (line - above, why))
    .collect();
  let bound = |line: usize, name: &str| {
    (
      line,
      format!(
        "error[engine-name] (column 3) `{name}` is a name of the engine, which each later word reads: give the value a name of its own"
      ),
    )
  };
  assert_eq!(
    got,
    vec![
      bound(1, "wait"),
      bound(2, "chance"),
      bound(4, "cd"),
      bound(5, "turns"),
      bound(15, "re"),
      bound(16, "CancelledError"),
      (18, bound(18, "scope").1.replace("column 3", "column 5")),
      bound(19, "span"),
    ]
  );
}

#[test]
fn a_walrus_at_the_top_of_a_word_binds_a_name_of_the_engine_but_not_inside_a_lambda() {
  let word = "(wait := 5)\nif (chance := 1):\n  pass\nk = [(cd := x) for x in ()]\nf = lambda: (turns := 1)\nclose(1)";
  assert_eq!(
    engine_names(word),
    vec![(1, 4, "wait".to_owned()), (2, 7, "chance".to_owned()), (4, 9, "cd".to_owned())]
  );
}

#[test]
fn a_capture_of_a_match_at_the_top_of_a_word_binds_a_name_of_the_engine() {
  let word = "match [1]:\n  case [wait, *chance]:\n    pass\n  case {'k': 1, **cd}:\n    pass\n  case int() as turns:\n    pass\n  case other:\n    pass\nclose(1)";
  assert_eq!(
    engine_names(word),
    vec![
      (2, 11, "wait".to_owned()),
      (2, 18, "chance".to_owned()),
      (4, 21, "cd".to_owned()),
      (6, 19, "turns".to_owned()),
    ]
  );
}

#[test]
fn a_global_of_a_def_or_a_class_binds_a_name_of_the_engine_in_the_module() {
  let word = "def f():\n  global wait, chance, mine\n  wait = 1\ndef g():\n  def h():\n    global cd\n    cd = 1\nclass C:\n  def m(self):\n    global turns\nclose(1)";
  assert_eq!(
    engine_names(word),
    vec![
      (2, 12, "wait".to_owned()),
      (2, 18, "chance".to_owned()),
      (6, 14, "cd".to_owned()),
      (10, 14, "turns".to_owned()),
    ]
  );
}

#[test]
fn a_walrus_in_a_default_or_a_decorator_of_a_def_binds_a_name_of_the_engine() {
  let word = "def f(a=(wait := 1), b=(chance := 2)):\n  (turns := 4)\n@(span := 5)\ndef g():\n  pass\nclose(1)";
  assert_eq!(
    engine_names(word),
    vec![(1, 12, "wait".to_owned()), (1, 27, "chance".to_owned()), (3, 5, "span".to_owned())]
  );
}

#[test]
fn a_walrus_in_a_base_a_keyword_or_a_decorator_of_a_class_binds_a_name_of_the_engine() {
  let word = "@(turns := 0)\nclass C((span := 5), k=(wait := 6)):\n  (cd := 7)\nclose(1)";
  assert_eq!(
    engine_names(word),
    vec![(1, 5, "turns".to_owned()), (2, 12, "span".to_owned()), (2, 27, "wait".to_owned())]
  );
}

#[test]
fn a_walrus_in_the_body_of_a_def_or_a_class_or_in_a_lambda_of_a_default_binds_nothing() {
  let word = "def f(a=lambda: (wait := 1)):\n  (chance := 2)\nclass C:\n  (cd := 3)\nclose(1)";
  assert_eq!(engine_names(word), vec![]);
}

/// The line and the column of the word and the name of each finding of the engine-name rule.
fn engine_names(word: &str) -> Vec<(usize, usize, String)> {
  let (text, above) = acted(&[], &[], word);
  found(&text)
    .into_iter()
    .filter(|(line, why)| *line > above && why.starts_with("error[engine-name]"))
    .map(|(line, why)| {
      let column = why.split("(column ").nth(1).and_then(|r| r.split(')').next());
      let name = why.split('`').nth(1).unwrap_or_default();
      (line - above, column.and_then(|c| c.parse().ok()).unwrap_or(0), name.to_owned())
    })
    .collect()
}

#[test]
fn a_value_of_another_type_for_a_name_that_a_rung_declares_names_that_rung() {
  let program = ["def pr() -> int:\n  return 1", "import json\nmerged: set[int] = set()\nk = 1"];
  let word = "pr = 'a'\nfor merged in [[1]]:\n  pass\nk = 'b'\ndef f() -> int:\n  json = 1\n  return json\nclose(pr)";
  let (text, above) = sheet(&program, word);
  let got: Vec<_> = found(&text)
    .into_iter()
    .filter(|(line, _)| *line > above)
    .map(|(line, why)| (line - above, why))
    .collect();
  assert_eq!(got.iter().map(|(line, _)| *line).collect::<Vec<_>>(), vec![1, 2], "{got:?}");
  assert!(got[0].1.starts_with("error[invalid-assignment]"), "{got:?}");
  assert!(
    got[0].1.ends_with(
      "; the word of rung1 declares `pr` with that type, so give this value another name"
    ),
    "{got:?}"
  );
  assert!(
    got[1].1.ends_with(
      "; the word of rung2 declares `merged` with that type, so give this value another name"
    ),
    "{got:?}"
  );
  let (text, above) = sheet(&program, "pr = 'a'");
  let unnamed = found(&text.replace("__words = ('rung1', 'rung2')", "__words = ()"));
  let tail = "; a word of the program of this chain declares `pr` with that type, so give this value another name";
  assert_eq!(
    unnamed.iter().map(|(line, _)| *line).collect::<Vec<_>>(),
    vec![above + 1],
    "{unnamed:?}"
  );
  assert!(unnamed[0].1.ends_with(tail), "{unnamed:?}");
  assert_eq!(said(&program, "pr: str = 'a'\nclose(pr)"), vec![]);
}

/// The factor by which a test of speed lets one time pass another. Work that grows in a line passes
/// it at 4, and work that grows in a square fails it at 16.
const SLACK: u32 = 10;

/// The least time of five runs of `work`, which a pause of the machine in some of them cannot
/// raise. A bound in seconds fails on a machine under load, so a test of speed compares two times
/// that one process measures. The gate keeps what it read of a text, so each run reads a text of
/// its own, which `make` gives from the number of the run, and a run before them all, which is not
/// timed, reads the typeshed and the engine.
fn quickest<T>(make: impl Fn(usize) -> T, work: impl Fn(T)) -> std::time::Duration {
  work(make(0));
  (1..=5)
    .map(|run| {
      let given = make(run);
      let start = std::time::Instant::now();
      work(given);
      start.elapsed()
    })
    .min()
    .expect("five runs")
}

/// A name that a tell binds to a value of lists, `depth` levels deep, as monty writes it.
fn lists(depth: usize) -> String {
  format!("#bash1 exited 0\nbash1_value = {}{}", "[".repeat(depth), "]".repeat(depth))
}

/// A literal `depth` levels deep, each level a list that holds an int, the level below, and a dict,
/// with an empty list at the bottom, which the checker reads the slowest of the shapes that the
/// gate measured.
fn mixed(depth: usize) -> String {
  (1..depth).fold("[]".to_owned(), |below, _| format!("[1, {below}, {{'k': (1, 'a')}}]"))
}

#[test]
fn a_literal_past_the_bound_is_cut_whole_to_an_empty_one_that_keeps_its_lines() {
  let (open, close) = ("[".repeat(DEEP), "]".repeat(DEEP));
  assert_eq!(shallowed(&format!("x = {open}{{1: [\n2]}}{close}\ny = 1")), "x = [\n]\ny = 1");
  let deep = mixed(DEEP + 1);
  let items = &deep[1..deep.len() - 1];
  let word = format!("x = ({deep},)\ny = {{1: {deep}}}\nz = {{{items}}}");
  assert_eq!(shallowed(&word), "x = tuple([])\ny = {}\nz = set()");
}

#[test]
fn a_literal_within_the_bound_a_call_and_a_subscript_stay_as_they_stand() {
  let deep = "f(".repeat(DEEP + 1) + &")".repeat(DEEP + 1);
  let word = format!(
    "{}\nx = {{'a': [(1, {{2}})]}}\n{deep}\ny = x{}\nz = f'{{x}}'",
    lists(DEEP),
    "[0]".repeat(DEEP + 1)
  );
  assert_eq!(shallowed(&word), word);
}

#[test]
fn a_value_that_a_tell_wrote_as_a_deep_literal_stalls_no_word() {
  let word = "y: int = len(bash1_value)\nclose(bash1_value[0])";
  let checked = |program: String| assert_eq!(said(&[&program], word), vec![]);
  let shallow = quickest(|run| format!("{}\n# {run}", lists(1)), checked);
  for deep in
    [lists(990), format!("bash1_value = {}", mixed(100)), format!("bash1_value = {}", mixed(DEEP))]
  {
    let took = quickest(|run| format!("{deep}\n# {run}"), checked);
    assert!(
      took < shallow * SLACK,
      "{} bytes took {took:?}, a shallow value {shallow:?}",
      deep.len()
    );
  }
}

#[test]
fn a_word_reads_a_value_that_was_cut_as_a_collection_of_its_kind() {
  let deep = mixed(DEEP + 1);
  let program = format!("a = ({deep},)\nb = {{'k': {deep}}}\nc = {deep}");
  assert_eq!(said(&[&program], "x = a[3][0]\ny = b['j'][1]\nz: int = len(c[5])"), vec![]);
  let found = said(&[&program], "w: str = len(c)");
  assert_eq!(found.len(), 1, "{found:?}");
}

/// A word of the program of a life of many hours, which binds the same names in a loop and under an
/// if, as a model writes a word again and again.
fn looped(n: usize) -> String {
  format!(
    "for one in read('a{n}').lines:\n  if one.startswith('x'):\n    got = bash(one)\n    pr = {{'labels': [one], 'n': ['{n}']}}\nlabel = pr['labels'][0] if pr else ''\ndebug(t'{{label=}} {{got=}} {{one=}}')"
  )
}

/// The lines that the gate hands to the checker for a sheet: each module that `seen` lacks, and the
/// text of the sheet. It adds the paths of those modules to `seen`.
fn handed(text: &str, seen: &mut HashSet<String>) -> usize {
  let lines: Vec<&str> = text.split_inclusive('\n').collect();
  let (first, tail) = program_lines(&lines).expect("a sheet laid out as furb.sheet writes it");
  let short = shallowed(&shortened(&lines, first, tail));
  let laid = laid(&lines, first, tail, &short).expect("a sheet that stands on modules");
  let modules: usize = laid
    .modules
    .iter()
    .filter(|(path, _)| seen.insert(path.clone()))
    .map(|(_, module)| module.lines().count())
    .sum();
  modules + laid.main.lines().count()
}

#[test]
fn a_name_that_each_word_binds_in_a_loop_stalls_no_word() {
  let mut seen = HashSet::new();
  let mut reading = |words: usize| {
    let program: Vec<String> = (0..words).map(looped).collect();
    let (text, _) =
      sheet(&program.iter().map(String::as_str).collect::<Vec<_>>(), "close(one + label)");
    assert_eq!(found(&text), vec![]);
    handed(&text, &mut seen)
  };
  let (_, early) = (reading(99), reading(100));
  let (_, late) = (reading(399), reading(400));
  assert!(late <= early, "the word 400 hands {late} lines to the checker, the word 100 {early}");
}

#[test]
fn a_name_that_a_nested_statement_binds_again_in_each_word_keeps_each_type() {
  let program = [
    "x = 1",
    "for x in ['a']:\n  pass",
    "if chance() > 0.5:\n  x = b'a'",
    "for x in [1.5]:\n  pass",
  ];
  let found = said(&program, "y: None = x");
  assert_eq!(found.len(), 1, "{found:?}");
  for kind in ["1", "\"a\"", "b\"a\"", "float"] {
    assert!(found[0].1.contains(kind), "{found:?}");
  }
}

#[test]
fn a_name_keeps_its_type_and_its_declaration_across_the_modules_of_the_program() {
  let mut program = vec![
    "x = 1".to_owned(),
    "def pr() -> int:\n  return 1".to_owned(),
    "merged: set[int] = set()".to_owned(),
  ];
  program.extend((0..2 * CHUNK).map(|n| format!("k = {n}")));
  program.push("for x in ['a']:\n  pass".to_owned());
  let program: Vec<&str> = program.iter().map(String::as_str).collect();
  let found = said(&program, "y: None = x\npr = 'a'\nz: None = merged\nw: None = k");
  let lines: Vec<usize> = found.iter().map(|(line, _)| *line).collect();
  assert_eq!(lines, vec![1, 2, 3, 4], "{found:?}");
  assert!(found[0].1.contains("Literal[1, \"a\"]"), "{found:?}");
  assert!(
    found[1]
      .1
      .ends_with("the word of rung2 declares `pr` with that type, so give this value another name"),
    "{found:?}"
  );
  assert!(found[2].1.contains("set[int]"), "{found:?}");
  assert!(found[3].1.contains(&format!("Literal[{}]", 2 * CHUNK - 1)), "{found:?}");
}

#[test]
fn a_gate_after_one_more_word_reads_the_words_after_the_last_module_alone() {
  let program: Vec<String> = (0..2000).map(looped).collect();
  let program: Vec<&str> = program.iter().map(String::as_str).collect();
  let (text, _) = sheet(&program[..1999], "close(one + label)");
  let start = std::time::Instant::now();
  found(&text);
  let cold = start.elapsed();
  let (text, _) = sheet(&program, "close(one + label)");
  let start = std::time::Instant::now();
  found(&text);
  let warm = start.elapsed();
  assert!(warm * 4 < cold, "cold {cold:?}, warm {warm:?}");
}

#[test]
fn a_warning_refuses_no_word() {
  // A name that may be unbound is a warning of the checker, and a word that reads it is no word the
  // gate refuses.
  assert_eq!(said(&[], "if chance() > 0.5:\n  maybe = 1\nclose(maybe)"), vec![]);
}

#[test]
fn a_word_binds_no_builtin_that_a_def_of_the_program_reads() {
  let program =
    ["def kind(got: object) -> str:\n  return 'many' if isinstance(got, list) else 'one'"];
  let found = said(&program, "list = 'ls -la'");
  assert_eq!(found.len(), 1, "{found:?}");
  assert!(found[0].1.starts_with("error[builtin-name]"), "{found:?}");
  assert!(found[0].1.contains("`list` is a builtin that a def of this chain reads"), "{found:?}");
  assert_eq!(said(&program, "sum = 'ls'"), vec![], "no def reads sum");
  assert_eq!(said(&[], "list = 'ls'"), vec![], "no def reads list");
}

#[test]
fn a_word_of_a_skill_that_binds_a_name_of_the_engine_in_a_block_a_walrus_a_capture_or_a_global_binds_it()
 {
  let bound = |word: &str| super::engine_bound(word, ENGINE);
  assert_eq!(bound("if True:\n  over = 1"), vec!["over"]);
  assert_eq!(bound("(question := 1)"), vec!["question"]);
  assert_eq!(bound("match 1:\n  case scope:\n    pass"), vec!["scope"]);
  assert_eq!(bound("def f():\n  global under\n  under = 1"), vec!["under"]);
  assert_eq!(bound("accepts = None\nactor = 'x'\nimport re"), Vec::<String>::new());
}
