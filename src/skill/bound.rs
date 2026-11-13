//! Each name of the engine that the word of a skill binds at its top, which a skill that loads
//! refuses, since each word of a chain reads the names of the engine in the module of the chain.

use ruff_python_ast::{
  ExceptHandler, Expr, Pattern, Stmt,
  statement_visitor::{StatementVisitor, walk_stmt},
  visitor::{self, Visitor},
};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

/// The names that a chain binds of its own, though the engine binds each first: the actor, the last
/// raise and the policy of the codes of a command, which a word binds like any name.
const OWN: [&str; 3] = ["actor", "raised", "accepts"];

/// Each name of the engine that a statement at the top of a word binds or deletes, once and in its
/// order: in a block, by a def, a class, a walrus, a capture or a `global` too. A word may import a
/// name that the engine imports under that name, since it binds the same thing, and it binds the
/// names that a chain binds as its own.
pub(super) fn engine_bound(word: &str, engine: &str) -> Vec<String> {
  let (Ok(word), Ok(held)) = (parse_module(word), parse_module(engine)) else { return vec![] };
  let names: Vec<&str> =
    held.syntax().body.iter().flat_map(binds).filter(|name| !OWN.contains(name)).collect();
  let imports = imported(engine);
  let mut found: Vec<String> = vec![];
  for (_, name, import) in bindings(&word.syntax().body) {
    let rebinds = names.contains(&name) && !(import && imports.iter().any(|one| one == name));
    if rebinds && !found.iter().any(|one| one == name) {
      found.push(name.to_owned());
    }
  }
  found
}

/// The names that a statement binds when it runs to its end, though it may read them first: an
/// assignment, an annotated assignment with a value, a def, a class, a type alias and an import.
fn binds(stmt: &Stmt) -> Vec<&str> {
  match stmt {
    Stmt::Assign(one) => one.targets.iter().flat_map(named).collect(),
    Stmt::AnnAssign(one) if one.value.is_some() => named(&one.target),
    Stmt::FunctionDef(one) => vec![one.name.as_str()],
    Stmt::ClassDef(one) => vec![one.name.as_str()],
    Stmt::TypeAlias(one) => named(&one.name),
    Stmt::Import(one) => one
      .names
      .iter()
      .map(|alias| match &alias.asname {
        Some(name) => name.as_str(),
        None => alias.name.as_str().split('.').next().unwrap_or_default(),
      })
      .collect(),
    Stmt::ImportFrom(one) => {
      one.names.iter().map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()).collect()
    }
    _ => vec![],
  }
}

/// The names that a target of an assignment binds.
fn named(target: &Expr) -> Vec<&str> {
  match target {
    Expr::Name(one) => vec![one.id.as_str()],
    Expr::Tuple(one) => one.elts.iter().flat_map(named).collect(),
    Expr::List(one) => one.elts.iter().flat_map(named).collect(),
    Expr::Starred(one) => named(&one.value),
    _ => vec![],
  }
}

/// The names that an engine imports, which a word may import too under the same name, since each
/// is no name of the contract. The imports of an engine stand at its head, before its first other
/// statement, and one may hold more than one line.
fn imported(engine: &str) -> Vec<String> {
  let head: String = engine
    .split_inclusive('\n')
    .take_while(|line| {
      line.trim().is_empty()
        || ["import ", "from ", " ", ")"].iter().any(|start| line.starts_with(start))
    })
    .collect();
  let Ok(parsed) = parse_module(&head) else { return vec![] };
  parsed.syntax().body.iter().flat_map(binds).map(str::to_owned).collect()
}

/// Each name that a statement at the top of a body binds or deletes, by where it starts, and
/// whether an import with no alias binds it, in the order of the body.
fn bindings(body: &[Stmt]) -> Vec<(usize, &str, bool)> {
  let mut found = vec![];
  topped(body, &mut found);
  let mut reached = Reached(vec![]);
  for stmt in body {
    reached.visit_stmt(stmt);
  }
  found.extend(reached.0.into_iter().map(|(at, name)| (at, name, false)));
  found.sort_by_key(|(at, ..)| *at);
  found
}

/// Each name that a statement at the top of a body binds or deletes, by where it starts, and
/// whether an import binds it, into the bodies of an if, a for, a while, a with, a try and a match,
/// since python binds each name of those bodies in the module too. Nothing inside a def or a class
/// stands at the top.
fn topped<'a>(body: &'a [Stmt], found: &mut Vec<(usize, &'a str, bool)>) {
  for stmt in body {
    let names = match stmt {
      Stmt::AugAssign(one) => named(&one.target),
      Stmt::For(one) => named(&one.target),
      Stmt::With(one) => {
        one.items.iter().filter_map(|item| item.optional_vars.as_deref()).flat_map(named).collect()
      }
      Stmt::Delete(one) => one.targets.iter().flat_map(named).collect(),
      _ => binds(stmt),
    };
    let plain: Vec<bool> = match stmt {
      Stmt::Import(one) => one.names.iter().map(|alias| alias.asname.is_none()).collect(),
      Stmt::ImportFrom(one) => one.names.iter().map(|alias| alias.asname.is_none()).collect(),
      _ => vec![],
    };
    found.extend(
      names
        .into_iter()
        .enumerate()
        .map(|(at, name)| (stmt.start().to_usize(), name, plain.get(at).copied().unwrap_or(false))),
    );
    match stmt {
      Stmt::If(one) => {
        topped(&one.body, found);
        for clause in &one.elif_else_clauses {
          topped(&clause.body, found);
        }
      }
      Stmt::For(one) => {
        topped(&one.body, found);
        topped(&one.orelse, found);
      }
      Stmt::While(one) => {
        topped(&one.body, found);
        topped(&one.orelse, found);
      }
      Stmt::With(one) => topped(&one.body, found),
      Stmt::Try(one) => {
        topped(&one.body, found);
        for handler in &one.handlers {
          let ExceptHandler::ExceptHandler(caught) = handler;
          found.extend(
            caught.name.iter().map(|name| (caught.start().to_usize(), name.as_str(), false)),
          );
          topped(&caught.body, found);
        }
        topped(&one.orelse, found);
        topped(&one.finalbody, found);
      }
      Stmt::Match(one) => {
        for case in &one.cases {
          topped(&case.body, found);
        }
      }
      _ => {}
    }
  }
}

/// The names that a statement binds in the module by a way that `binds` does not read: the target
/// of a walrus (also inside a comprehension, which binds it in the scope around it), each capture
/// of a pattern of a match, and each name that a def or a class declares `global`, at any depth. A
/// lambda binds its walrus apart, so nothing inside it counts. A def or a class runs its
/// decorators, the defaults of its parameters, and its bases and keywords in the module, so a
/// walrus there binds too, but a walrus in its body binds apart. Python refuses a walrus in an
/// annotation.
struct Reached<'a>(Vec<(usize, &'a str)>);

impl<'a> Visitor<'a> for Reached<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
      let mut globals = Globals(vec![]);
      StatementVisitor::visit_stmt(&mut globals, stmt);
      self.0.extend(globals.0);
      match stmt {
        Stmt::FunctionDef(one) => {
          one.decorator_list.iter().for_each(|one| self.visit_decorator(one));
          self.visit_parameters(&one.parameters);
        }
        Stmt::ClassDef(one) => {
          one.decorator_list.iter().for_each(|one| self.visit_decorator(one));
          one.arguments.iter().for_each(|one| self.visit_arguments(one));
        }
        _ => {}
      }
    } else {
      visitor::walk_stmt(self, stmt);
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    match expr {
      Expr::Lambda(_) => {}
      Expr::Named(one) => {
        if let Expr::Name(target) = &*one.target {
          self.0.push((target.start().to_usize(), target.id.as_str()));
        }
        visitor::walk_expr(self, expr);
      }
      _ => visitor::walk_expr(self, expr),
    }
  }

  fn visit_pattern(&mut self, pattern: &'a Pattern) {
    let name = match pattern {
      Pattern::MatchAs(one) => one.name.as_ref(),
      Pattern::MatchStar(one) => one.name.as_ref(),
      Pattern::MatchMapping(one) => one.rest.as_ref(),
      _ => None,
    };
    self.0.extend(name.map(|name| (name.start().to_usize(), name.as_str())));
    visitor::walk_pattern(self, pattern);
  }
}

/// Each name that a `global` statement declares, at any depth of a def or a class.
struct Globals<'a>(Vec<(usize, &'a str)>);

impl<'a> StatementVisitor<'a> for Globals<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    if let Stmt::Global(one) = stmt {
      self.0.extend(one.names.iter().map(|name| (name.start().to_usize(), name.as_str())));
    }
    walk_stmt(self, stmt);
  }
}

#[cfg(test)]
#[path = "bound.test.rs"]
mod test;
