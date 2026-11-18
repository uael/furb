//! The names of the code of a thread, as an editor reads them: where each name stands in its line,
//! whether its line binds it, and what the contract says of each verb of the engine. The pointer on
//! a name shows what it holds, and a click on it goes to the line that bound it.

use std::{collections::HashMap, ops::Range, rc::Rc};

use crate::cells::Cells;
use opentui::syntax::tree_sitter::{Node, Parser};

/// A name in a line of code: its columns in the line, its text, and whether the line binds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Named {
  pub(crate) at: Range<usize>,
  pub(crate) name: String,
  pub(crate) binds: bool,
}

/// How many texts the names keep before they forget them all.
const KEPT: usize = 256;

/// The names of each text of python that the window read, so a frame that shows a block again
/// reads no grammar.
#[derive(Default)]
pub(crate) struct Names {
  kept: HashMap<String, Rc<Vec<Vec<Named>>>>,
}

impl Names {
  /// The names of each line of a text of python. A text that the grammar cannot read names what
  /// it reads.
  pub(crate) fn of(&mut self, text: &str) -> Rc<Vec<Vec<Named>>> {
    if let Some(kept) = self.kept.get(text) {
      return Rc::clone(kept);
    }
    let names = Rc::new(read(text));
    if self.kept.len() >= KEPT {
      self.kept.clear();
    }
    self.kept.insert(text.to_owned(), Rc::clone(&names));
    names
  }
}

fn read(text: &str) -> Vec<Vec<Named>> {
  let lines: Vec<&str> = text.split('\n').collect();
  let mut names = vec![Vec::new(); lines.len()];
  let mut parser = Parser::new();
  if parser.set_language(&tree_sitter_python::LANGUAGE.into()).is_err() {
    return names;
  }
  let Some(tree) = parser.parse(text, None) else { return names };
  let mut stack = vec![tree.root_node()];
  while let Some(node) = stack.pop() {
    let mut walk = node.walk();
    stack.extend(node.children(&mut walk));
    let start = node.start_position();
    if node.kind() != "identifier" || node.end_position().row != start.row || !named(node) {
      continue;
    }
    let Some(line) = lines.get(start.row) else { continue };
    let (from, to) = (start.column, node.end_position().column);
    let (Some(before), Some(name)) = (line.get(..from), line.get(from..to)) else { continue };
    let at = before.width()..before.width() + name.width();
    if let Some(row) = names.get_mut(start.row) {
      row.push(Named { at, name: name.to_owned(), binds: binds(node) });
    }
  }
  for line in &mut names {
    line.sort_by_key(|one| one.at.start);
  }
  names
}

/// Whether an identifier names a value of the module: not the attribute after a dot, nor the key
/// of a word of a call.
fn named(node: Node<'_>) -> bool {
  let Some(parent) = node.parent() else { return true };
  let field = |name| parent.child_by_field_name(name).is_some_and(|one| one.id() == node.id());
  !(parent.kind() == "attribute" && field("attribute")
    || parent.kind() == "keyword_argument" && field("name"))
}

/// Whether the statement of an identifier binds it: the target of an assignment or of a loop, the
/// name of a def or of a class, a parameter, or the name after an `as`.
fn binds(node: Node<'_>) -> bool {
  let mut at = node;
  while let Some(parent) = at.parent() {
    let field = |name| parent.child_by_field_name(name).is_some_and(|one| one.id() == at.id());
    match parent.kind() {
      "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern" => at = parent,
      "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
        return field("left");
      }
      "function_definition" | "class_definition" => return field("name"),
      "parameters" | "default_parameter" | "typed_parameter" | "typed_default_parameter" => {
        return true;
      }
      "as_pattern_target" | "named_expression" => return true,
      "aliased_import" => return field("alias"),
      _ => return false,
    }
  }
  false
}

/// What the contract says of a verb of the engine: its call, with the name and the default of each
/// parameter, and its first sentence.
pub(crate) fn verb(name: &str) -> Option<(String, String)> {
  let head = format!("def {name}");
  let start = furb::CONTRACT.match_indices(&head).map(|(at, _)| at).find(|at| {
    let rest = furb::CONTRACT.get(at + head.len()..).unwrap_or_default();
    let line = *at == 0 || furb::CONTRACT.get(..*at).is_some_and(|before| before.ends_with('\n'));
    line && (rest.starts_with('(') || rest.starts_with('['))
  })?;
  let def = furb::CONTRACT.get(start..)?;
  let open = def.find('(')?;
  let mut depth = 0;
  let close = def.get(open..)?.char_indices().find_map(|(at, one)| {
    depth += match one {
      '(' | '[' => 1,
      ')' | ']' => -1,
      _ => 0,
    };
    (depth == 0).then_some(open + at)
  })?;
  let parameters: Vec<String> =
    split(def.get(open + 1..close)?).into_iter().filter_map(parameter).collect();
  let body = def.get(close..)?;
  let said = body.split_once("\"\"\"").and_then(|(_, after)| after.lines().next());
  let said = said.map(|one| one.trim_end_matches("\"\"\"").trim().to_owned()).unwrap_or_default();
  Some((format!("{name}({})", parameters.join(", ")), said))
}

/// The parameters of a signature, at the commas outside its brackets.
fn split(text: &str) -> Vec<&str> {
  let (mut parts, mut depth, mut from) = (Vec::new(), 0, 0);
  for (at, one) in text.char_indices() {
    match one {
      '(' | '[' | '{' => depth += 1,
      ')' | ']' | '}' => depth -= 1,
      ',' if depth == 0 => {
        parts.push(text.get(from..at).unwrap_or_default());
        from = at + 1;
      }
      _ => {}
    }
  }
  parts.push(text.get(from..).unwrap_or_default());
  parts
}

/// One parameter as a call writes it: its name, with its default when it has one.
fn parameter(text: &str) -> Option<String> {
  let text = text.trim();
  let name = text.split([':', '=']).next()?.trim();
  if name.is_empty() {
    return None;
  }
  let default = text.split_once('=').map(|one| one.1.trim());
  Some(default.map_or_else(|| name.to_owned(), |default| format!("{name}={default}")))
}
