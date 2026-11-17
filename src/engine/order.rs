//! The order in which a replay on an engine of an earlier build of the major says an entry of its
//! record that the outside said, made the order of the life that said it.
//!
//! Live, each ear of the engine hears a fact before each ear of the outside, and the facts that a
//! word says while an ear is busy wait in the log, so an ear of the outside answers as it hears a
//! fact, once each ear of the engine heard it and each fact before it. A record of the schema 3.1
//! holds the count of the facts that waited as the life said each entry, from which the engine of
//! this build says an answer where the life said it. The engines before it said an answer as soon
//! as its place came, before the ears of the engine heard the facts that wait, and an ear of a word
//! that reads what an act came to read the answer early, and the replay drifted. Their records hold
//! no count, so this build gives the dispatch of those texts the rule by which the ears of the
//! outside of the major answered: a provider ends a reply, and tells about its rung, as it hears
//! the `ran` of that rung, after each fact that the run said. So an answer comes as the first ear
//! of the outside is about to hear the `ran` of the act that it is about, or of the act that made
//! that act, when the log holds one, and else as it is about to hear the head of the log. A record
//! names the text that it ran, and each chain reads that engine as its system prompt, so the host
//! changes only the dispatch of the text that the sandbox runs for a segment, and only for the
//! texts that this file lists: a live life of that text does the same as before, since nothing
//! waits in its record.

use sha2::{Digest, Sha256};

use crate::{CHECKPOINTER, value::Fault};

/// The dispatch of each engine of the major that says a due answer first, and the dispatch that
/// says it where the ear of the outside said it.
const FORMS: [(&str, &str); 2] = [
  (
    r"  def dispatch():
    while not busy:
      if broke and broke[-1]:
        broke.append(None)
        raise broke[0]
      if log and due and ready(*due[0]) and not question(due[0][0]):
        play()
      elif log:
        a, heard, k, entry = log[0]
        for m, g in ears(k):
          m in heard or heard.add(m) or hear(m, g, a)
        log.pop(0)
        keep(entry)
      else:
        play(True)
        return
",
    r#"  def dispatch():
    while not busy:
      if broke and broke[-1]:
        broke.append(None)
        raise broke[0]
      if log:
        a, heard, k, entry = log[0]
        for m, g in ears(k):
          if m in outside and due and ready(*due[0]) and not question(due[0][0]):
            of = (get(due[0][0][1]) or due[0][0])[1:3]
            if a[0] == "ran" and a[1] in of or not any(x[0][0] == "ran" and x[0][1] in of for x in log):
              play()
              break
          m in heard or heard.add(m) or hear(m, g, a)
        else:
          log.pop(0)
          keep(entry)
      else:
        play(True)
        return
"#,
  ),
  (
    r"  def dispatch():
    while not busy:
      if broke and broke[-1]:
        broke.append(None)
        raise broke[0]
      if log and not (due and ready(*due[0]) and not question(due[0][0])):
        a, heard, k, entry = log[0]
        for m, g in ears(k):
          m in heard or heard.add(m) or hear(m, g, a)
        log.pop(0)
        keep(entry)
      elif not play(not log):
        return
",
    r#"  def dispatch():
    while not busy:
      if broke and broke[-1]:
        broke.append(None)
        raise broke[0]
      if log:
        a, heard, k, entry = log[0]
        for m, g in ears(k):
          if m in outside and due and ready(*due[0]) and not question(due[0][0]):
            of = (get(due[0][0][1]) or due[0][0])[1:3]
            if a[0] == "ran" and a[1] in of or not any(x[0][0] == "ran" and x[0][1] in of for x in log):
              break
          m in heard or heard.add(m) or hear(m, g, a)
        else:
          log.pop(0)
          keep(entry)
          continue
      if not play(not log):
        return
"#,
  ),
];

/// The sha256 of each text of an engine of the major whose dispatch says a due answer first, from
/// the stable cut on, with the form of [`FORMS`] that it holds.
const TEXTS: [(&str, usize); 17] = [
  ("07b8c9a730424ee71a481a82fb0639d58e78a219460283c472f49abb540a3f2c", 0),
  ("2b10a7b9fe195555c08d87a21c1ef200e487303c4b9da88557db02d50cf31141", 0),
  ("30be68d4ed4805d6847f12cd6a7b1417be959d052bce57327a9dbc1f96d8f339", 0),
  ("34458e72b7cb56ed1c6bcc818a077c457e648911f9db1dc933db22252aaa1476", 0),
  ("4686c53240aabe7b06d712bc9ee96de903ecda82489a38e80cdd5ca4e712e20c", 0),
  ("4906d3333f4b671ba7f86c2b0c5cc0be1c2c86b9b2266a1f74217de476db7474", 0),
  ("4aa0219548067a2c34d92ba654bc174adbeafcab89df862ac589a00f3069fcbd", 0),
  ("6adfb0d10bfeea1578d0055f30d805a56ac6d35ad3ce47a73a22fafa0d06680d", 0),
  ("71ce8656f3abe5f18d3e47665a9d438f08a80028bae1aa485f4ee13a95e15072", 0),
  ("8839683cf5e306bb2f1e96d312d2ffbcc061dc5bf34b7f4887e81b7dc6268574", 0),
  ("9b7729257e87dde1acf8d2871ea4a219128d536efb7e740d3f50f8c03f0bc6c5", 0),
  ("cce94bde5450a97e3c7cdef07876e69c2568274058df3fce51e89e05a9707f62", 0),
  ("fae621195e254c96c8e458beb1d50786cab878493a8b1038027e7f1479306cc7", 0),
  ("17018647005c37d46943f7f5739d47780d86bf695f086ba3d9ca767655acbf7f", 1),
  ("ba8c294f3ab08a4f2b6fb52219098f063067c1e043eb34bd6c1d94f5b1d00c21", 1),
  ("c01dcd3c526a28e7580daa2510e892074a6a4a58582952f5db2bd48f2f91ba5a", 1),
  ("cd8c5266e26c0e77b32acfdcfc54011ab07aff58bce80a278b28b5dff4b42eb2", 1),
];

/// The text of an engine that the sandbox runs for a segment of a record: the text that the record
/// holds, with the dispatch of [`FORMS`] when [`TEXTS`] lists it, and as it is else.
pub(super) fn ordered(source: String) -> Result<String, Fault> {
  let digest = format!("{:x}", Sha256::digest(source.as_bytes()));
  let Some((_, form)) = TEXTS.iter().find(|(one, _)| *one == digest) else { return Ok(source) };
  let (old, new) = FORMS.get(*form).ok_or_else(|| Fault::refused(format!("no form {form}")))?;
  if !source.contains(old) {
    return Err(Fault::refused(format!(
      "the engine {digest} is listed with a dispatch that it does not hold, so its replay cannot be ordered"
    )));
  }
  Ok(source.replacen(old, new, 1))
}

/// The sha256 of each text of a checkpointer of the major that carries no chain that jumped: its
/// seed makes no chain under the name of a root that a compaction moved on, and the next engine,
/// which makes that root, drifts at its first entry.
const CARRIERS: [&str; 1] = ["95b810d66c359256600ed7da2f884d5d19fdeb6905588731a9f419d5130c2677"];

/// The text of the checkpointer that the sandbox runs for a segment or for the carry of its tip:
/// the checkpointer of this build in place of one that [`CARRIERS`] lists, which carries each chain
/// that jumped under its name, and the text that the record holds else.
pub(super) fn carrier(checkpointer: &str) -> &str {
  let digest = format!("{:x}", Sha256::digest(checkpointer.as_bytes()));
  if CARRIERS.contains(&digest.as_str()) { CHECKPOINTER } else { checkpointer }
}

#[cfg(test)]
#[path = "order.test.rs"]
mod test;
