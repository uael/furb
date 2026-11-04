//! The word of a reply as its model writes it: what the gate finds against it, and the part of it
//! that runs.

#![expect(
  clippy::format_collect,
  reason = "debt: a format! for each item, collected into a String"
)]

use rig_core::completion::CompletionError;

use super::{
  answer::cut,
  hearer::{Replied, Reply, noted},
  retry::failure,
  watch::{self, Watch},
};
use crate::{
  ear::{Co, say},
  fact::Fact,
  value::Object,
};

/// What the gate finds against the word that a model wrote so far.
pub(super) enum Found {
  /// Nothing, so its new statements run.
  Nothing,
  /// A fault that a later line of the word may still settle.
  Unsettled,
  /// A statement that cannot be python, or that the gate refuses for good, at this line, and why.
  Refused(usize, String),
}

/// What the gate finds against the word that a model wrote so far, on its chain.
pub(super) async fn judged(co: &mut Co<Replied>, chain: &str, word: &str) -> Found {
  if let Some(line) = watch::broken(word) {
    return Found::Refused(line, format!("line {line}: the statement there cannot be python"));
  }
  let found = findings(co, chain, word).await;
  if found.is_empty() {
    return Found::Nothing;
  }
  match watch::refused(&watch::settled(word), &found) {
    Some(line) => Found::Refused(line, found.join("\n")),
    None => Found::Unsettled,
  }
}

/// A word as python, each quote in it bound as a string by the Kernel, and nothing when the
/// Kernel binds none.
pub(super) async fn unquoted(co: &mut Co<Replied>, word: &str) -> Option<String> {
  if !word.contains(watch::QUOTE) {
    return Some(word.to_owned());
  }
  let got = co.call("unquoted", vec![Object::string(word)], vec![]).await;
  got.ok().and_then(|one| one.as_ref().as_str().map(str::to_owned))
}

/// What a reply came to once a part of its word ran. Its turn when the turn holds the word that
/// ran and the gate finds nothing against it, so the chain runs it whole; any other end of the
/// reply is the word that ran, after a note that tells the chain why, and nothing when that word
/// is no turn.
pub(super) async fn kept(
  co: &mut Co<Replied>,
  reply: &Reply,
  got: Result<Object, CompletionError>,
) -> Option<Result<Object, CompletionError>> {
  let Some(ran) = reply.call.watch.as_ref().and_then(Watch::ran) else { return Some(got) };
  let word =
    got.as_ref().ok().and_then(|turn| Some(turn.as_ref().items()?.get(1)?.as_str()?.to_owned()));
  let why = match &word {
    Some(word) if word.starts_with(ran.trim()) => match unquoted(co, word).await {
      Some(python) => findings(co, &reply.chain, &python).await.join("\n"),
      None => "the word holds a quote that is not whole".to_owned(),
    },
    Some(_) => "the turn does not hold the word that ran".to_owned(),
    None => match &got {
      Err(no) => format!("{STOPPED}: {}", failure(no)),
      Ok(_) => "the turn holds no word".to_owned(),
    },
  };
  if why.is_empty() {
    return Some(got);
  }
  let turn = cut(ran).ok()?;
  let (what, fields) = match &got {
    Err(no) => (STOPPED, [("why", failure(no)), ("ran", ending(ran))]),
    Ok(_) => (AFTER, [("findings", afterward(&why)), ("ran", ending(ran))]),
  };
  noted(co, &reply.chain, &reply.rung, what, &fields).await;
  Some(Ok(turn))
}

/// What the chain is told after the rung of a turn whose model wrote a fault after the part of its
/// word that ran.
pub(super) const AFTER: &str = "ran a part of its word, whose last lines its ran shows, then wrote a statement that the chain \
  would refuse, so its turn is the word that ran: each act, close and name of that part stands, and no later statement \
  runs";

/// How many lines of the end of the word that ran a note shows.
const ENDING: usize = 3;

/// The last lines of the word that ran, each with its number, which a note shows so that a model
/// knows where its run stopped.
pub(super) fn ending(ran: &str) -> String {
  let lines: Vec<(usize, &str)> =
    ran.trim_end().lines().enumerate().filter(|(_, line)| !line.trim().is_empty()).collect();
  let shown = lines[lines.len().saturating_sub(ENDING)..].iter();
  shown.map(|(at, line)| format!("line {}: {line}\n", at + 1)).collect()
}

/// The findings of the gate on a word whose part ran. The gate says of a word that is not python
/// that nothing of it ran, which holds only while no part of it ran.
pub(super) fn afterward(findings: &str) -> String {
  findings.replace("nothing of this word ran", "no statement after the part that ran runs")
}

/// What the chain is told after the rung of a reply whose call ended after a part of its word ran.
pub(super) const STOPPED: &str =
  "is the word that ran, since the call of its model ended after a part of it ran";

/// What the gate finds against a word, as python, on a chain, and nothing when the gate gives no
/// answer.
pub(super) async fn findings(co: &mut Co<Replied>, chain: &str, word: &str) -> Vec<String> {
  let words = vec![Object::string(chain), Object::string(word)];
  let found = co.call("against", words, vec![]).await.ok();
  let found = found.and_then(|one| {
    let items = one.as_ref().items()?;
    Some(items.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
  });
  found.unwrap_or_default()
}

/// The kind of the fact that the Kernel says, as a rung, once the part of a writing of that rung is
/// over: `("ran", rung, rung, before, raised)`, with how many characters of the word stand before
/// the part.
pub(super) const RAN: &str = "ran";

/// The part of the word of a rung that is new, said as a writing of that rung, `("writing", rung,
/// by, part, before)`, which the Kernel hears and runs as that rung; and false when a pause holds
/// the rung, since no part of a word runs while a pause holds its chain. The writing is a fact that
/// the ear says, and no act that it awaits, so the ear stands at its next yield when the Kernel
/// runs the part, and takes each reply that the part makes a chain ask, and the record keeps the
/// writing, so a later life runs the part where this one ran it.
pub(super) async fn writing(co: &mut Co<Replied>, rung: &str, part: &str, before: usize) -> bool {
  let paused = co.call("paused", vec![Object::string(rung)], vec![]).await;
  if paused.ok().and_then(|one| one.as_ref().as_bool()) != Some(false) {
    return false;
  }
  let before = Object::int(i64::try_from(before).unwrap_or(i64::MAX));
  say(co, Fact::says("writing", rung, [Object::string(part), before]));
  true
}
