//! The acts of a life as a window last heard them, which the life keeps as it tells them, so a
//! window asks the life for the meaning that it draws: the desk, the threads of a chain, the acts
//! as they stood at a moment, and the moments.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  collections::{BTreeSet, HashMap},
  mem,
  sync::Arc,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::OPERATOR;

use furb_wire::{
  act::{Act, Kind, Phase},
  desk::Desk,
  ladder::{Rung, Shown},
  state::State,
  threads::{Began, ChainTrack, ThreadNode, Threads},
};

use super::{
  desk::desk,
  ladder::{Index, folded, lives},
  threads::{
    Lookup, Rows, Works, attention, chain_asides, chain_notes, chain_threads, chain_tree,
    nest_threads, order_threads, works,
  },
  time::{as_of, moments_of},
};

/// Each act of a life, as the last event of it told it, in the order the acts were first told. An
/// act is shared, so a host that keeps the event of an act beside it holds the act once.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Held {
  acts: Vec<Arc<Act>>,
  at: HashMap<String, usize>,
  /// The places of the acts of each maker, and of each chain, in the order they were first told.
  made: HashMap<String, Vec<usize>>,
  on: HashMap<String, Vec<usize>>,
  /// The places of the acts that live: not done, or a rung whose word runs.
  live: BTreeSet<usize>,
  /// Whether an act was told that began before the last one held, so the order of the acts is not
  /// the order they began.
  unordered: bool,
  /// The place of each act or chain at which each operator last said that he saw it, by the site
  /// that said it: `operator` alone, or `operator/<login>` for the window of a member. A seen
  /// that a later life says again as the journal names no site, so it stands for each of them.
  seen: HashMap<String, HashMap<String, u64>>,
  /// The place of the last standing that is done, whose roster offers each actor its window.
  #[serde(default)]
  stood: Option<usize>,
}

impl<'a> Lookup<'a> for &'a Held {
  fn act(&self, id: &str) -> Option<&'a Act> {
    self.at.get(id).map(|at| &*self.acts[*at])
  }
}

impl<'a> Works<'a> for &'a Held {
  fn works(&self, chain: &str, at: u64) -> bool {
    works(self.on_chain(chain), at)
  }
}

impl Held {
  /// A fact of the life at its place: what the operator says that he saw, which marks the threads
  /// that he started and that ended after it.
  pub fn noted(&mut self, fact: &Value, at: u64) {
    let text = |n: usize| fact.get(n).and_then(Value::as_str).unwrap_or_default();
    if text(0) == "seen" && (super::operator(text(2)) || text(2) == "journal") {
      self.seen.entry(text(2).to_owned()).or_default().insert(text(1).to_owned(), at);
    }
  }

  /// An act of the life heard as a host tells it: it takes the place of the earlier one of the same
  /// id.
  pub fn heard(&mut self, act: &Act) {
    self.took(act.clone());
  }

  /// An act of the life heard as a host tells it, which the life gives up or shares with it: the
  /// fold of a record tells some hundred thousand of them, each with its words and its value, so it
  /// takes no copy of each. It gives back the act of the same id that it held before.
  ///
  /// A name names one act of a life. A birth on a checkpoint tells each act that the checkpoint
  /// carries again, from its start, with the maker `journal` or the operator in place of the
  /// maker that the record holds, at the place of the birth: the act keeps who made it, where it
  /// began and ended, and a done act takes no state that is not done. [`Held::shared`] gives the
  /// act as it holds it.
  pub fn took(&mut self, act: impl Into<Arc<Act>>) -> Option<Arc<Act>> {
    let mut act = act.into();
    let (at, before) = if let Some(&at) = self.at.get(&act.id) {
      let was = &self.acts[at];
      if was.done() && !act.done() {
        return None;
      }
      let ended = was.ended.or(act.ended);
      if (&act.by, &act.maker, act.began, act.ended) != (&was.by, &was.maker, was.began, ended) {
        let one = Arc::make_mut(&mut act);
        one.by.clone_from(&was.by);
        one.maker.clone_from(&was.maker);
        one.began = was.began;
        one.ended = ended;
      }
      (at, Some(mem::replace(&mut self.acts[at], act)))
    } else {
      let at = self.acts.len();
      // A read of now takes the acts in place as the order they began, which a host and a record
      // tell them in. A window that makes acts of its own may tell one that began before the last,
      // and then the reads sort a copy.
      if self.acts.last().is_some_and(|last| last.began > act.began) {
        self.unordered = true;
      }
      self.at.insert(act.id.clone(), at);
      self.made.entry(act.by.clone()).or_default().push(at);
      self.on.entry(act.chain.clone()).or_default().push(at);
      self.acts.push(act);
      (at, None)
    };
    if lives(&self.acts[at]) {
      self.live.insert(at);
    } else {
      self.live.remove(&at);
    }
    if self.acts[at].kind == Kind::Stand && self.acts[at].done() {
      self.stood = self.stood.max(Some(at));
    }
    before
  }

  /// The window that the roster of the last standing offers an actor, by its model alone or with
  /// one of its efforts, as `offered` of the engine reads it, and none when the roster holds no
  /// such actor.
  pub fn window(&self, actor: &str) -> Option<u64> {
    let roster = self.acts.get(self.stood?)?.value()?.get(0)?.as_array()?;
    roster.iter().find_map(|one| {
      let name = one.get(0)?.as_str()?;
      let efforts = one.get(1)?.as_array()?;
      let leveled =
        |effort: &Value| effort.as_str().is_some_and(|e| actor == format!("{name}/{e}"));
      (actor == name || efforts.iter().any(leveled)).then(|| one.get(2)?.as_u64()).flatten()
    })
  }

  /// The act of an id as the last event of it told it.
  pub fn shared(&self, id: &str) -> Option<&Arc<Act>> {
    self.at.get(id).map(|at| &self.acts[*at])
  }

  /// The ladder of a chain, now or as it stood just after a moment, with the folds that the window
  /// opened, the prompts that the operator owes, and the only acts at its top when `taken` names
  /// them.
  pub fn ladder(
    &self,
    chain: &str,
    at: Option<u64>,
    opened: &[String],
    owed: &[String],
    taken: Option<&[String]>,
    shown: Shown,
  ) -> Vec<Rung> {
    let Some(at) = at else {
      let index =
        Index { acts: &self.acts, at: &self.at, made: &self.made, on: &self.on, live: &self.live };
      return folded(&index, chain, opened, owed, taken, shown);
    };
    self.past(at).ladder(chain, None, opened, owed, taken, shown)
  }

  /// The acts as they stood just after a moment, held in the order they began, so each read of
  /// now reads that moment.
  #[must_use]
  pub fn past(&self, at: u64) -> Self {
    let mut then = Self::default();
    for act in self.as_of(at) {
      then.took(act);
    }
    then
  }

  /// The acts now, or as they stood just after a moment, in the order they began, as `read` takes
  /// them. The acts now are read in place, and the order of a life that was told an act out of
  /// order is a sort of references: a read of the threads, the rail, the desk or the cards takes
  /// no copy of the life, which holds some hundred thousand acts.
  pub(super) fn then<T>(&self, at: Option<u64>, read: impl FnOnce(&[&Act]) -> T) -> T {
    let past;
    let mut acts: Vec<&Act> = match at {
      None => self.acts.iter().map(|act| &**act).collect(),
      Some(at) => {
        past = as_of(self.acts.iter().map(|act| &**act), at);
        past.iter().collect()
      }
    };
    if self.unordered || at.is_some() {
      acts.sort_by_key(|act| act.began);
    }
    read(&acts)
  }

  /// What waits for the operator, with each notice that the window still shows.
  pub fn desk(&self, notices: &[String]) -> Desk {
    let notices: Vec<&str> = notices.iter().map(String::as_str).collect();
    self.then(None, |acts| desk(acts.iter().copied(), &notices))
  }

  /// The threads of a chain, with the workers of its rounds nested under the thread that started
  /// each, now or as they stood just after a moment. `read` is the place up to which the window
  /// read the chain, from which its runs of notes count the notes not yet read.
  pub fn threads(&self, chain: &str, at: Option<u64>, read: u64) -> Threads {
    self.threads_as(OPERATOR, chain, at, read)
  }

  /// The acts that an act or a person made, in the order they were first told.
  pub(super) fn made_by<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a Act> + use<'a> {
    self.made.get(id).into_iter().flatten().map(|at| &*self.acts[*at])
  }

  /// The text to write back at each path that a list of writes wrote, to revert them: the text
  /// that the life knew there before the first of them, in their order. A path that the life did
  /// not know before has none.
  pub fn reverted(&self, writes: &[String]) -> Vec<(String, String)> {
    let mut texts: Vec<(String, String)> = Vec::new();
    for id in writes {
      let Some((path, _)) = (&self).act(id).and_then(|act| self.filed(act)) else { continue };
      if texts.iter().any(|one| one.0 == path) {
        continue;
      }
      if let Some(before) = self.text_before(path, id) {
        texts.push((path.to_owned(), before.to_owned()));
      }
    }
    texts
  }

  /// The text that the life knew at the path of a file before an act: what the last read or write
  /// of that path that was told before the act came to. A read gives the whole file, whatever
  /// lines it tells, so its text is what stood there.
  pub(super) fn text_before(&self, path: &str, act: &str) -> Option<&str> {
    let place = *self.at.get(act)?;
    let mut texts = self.acts[..place].iter().rev().filter_map(|one| self.filed(one));
    texts.find(|(at, _)| *at == path).map(|(_, content)| content)
  }

  /// The path and the content of the file that a read or a write came to, once it is done: a text
  /// of a path that is no door of an act, whose first part is the name of that act.
  pub(super) fn filed<'a>(&self, act: &'a Act) -> Option<(&'a str, &'a str)> {
    let (Kind::Other(kind), Phase::Done(value)) = (&act.kind, &act.phase) else { return None };
    let text = (kind == "read" || kind == "write").then_some(value)?;
    let field = |name: &str| text.get(name).and_then(Value::as_str);
    let door = |path: &str| path.split('/').next().is_some_and(|first| self.at.contains_key(first));
    let path = field("path").filter(|path| !door(path))?;
    (field("is") == Some("Text")).then_some((path, field("content")?))
  }

  /// The acts of a chain, in the order they were first told.
  pub(super) fn on_chain<'a>(&'a self, chain: &str) -> impl Iterator<Item = &'a Act> + use<'a> {
    self.on.get(chain).into_iter().flatten().map(|at| &*self.acts[*at])
  }

  /// Each site that said what it saw, and the operator with no login, who saw what the journal
  /// says again.
  pub(super) fn sites(&self) -> impl Iterator<Item = &str> {
    let said =
      self.seen.keys().map(String::as_str).filter(|site| !matches!(*site, "journal" | OPERATOR));
    [OPERATOR].into_iter().chain(said)
  }

  /// What one site saw: its own seen and those of the journal, the later for each act, so a seen
  /// of another site clears nothing of its marks.
  pub(super) fn seen_by(&self, site: &str) -> HashMap<String, u64> {
    let mut seen = self.seen.get("journal").cloned().unwrap_or_default();
    for (id, at) in self.seen.get(site).into_iter().flatten() {
      let place = seen.entry(id.clone()).or_default();
      *place = (*place).max(*at);
    }
    seen
  }

  /// The threads of a chain as the window of a site reads them: the mark of what it has not seen
  /// counts the seen of that site alone.
  pub fn threads_as(&self, site: &str, chain: &str, at: Option<u64>, read: u64) -> Threads {
    self.then(at, |acts| {
      let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), *act)).collect();
      let on: Vec<&Act> =
        acts.iter().copied().filter(|act| act.chain == chain && act.id != chain).collect();
      let threads = chain_tree(chain, acts, &rows);
      let state =
        rows.get(chain).filter(|act| act.kind == Kind::Chain).map_or(State::Idle, |act| act.state);
      let (asides, notes) = (chain_asides(&on, &rows), chain_notes(&on, &rows, read));
      Threads {
        attention: attention(&threads, &rows, &self.seen_by(site)),
        asides,
        notes,
        threads,
        state,
      }
    })
  }

  /// The threads of every chain of the life but those that `hidden` names, as one tree in the
  /// order of the list of the threads: each thread under the thread whose ladder made its prompt,
  /// the closed ones first and the open ones last, now or as they stood just after a moment.
  pub fn rail(&self, hidden: &[String], at: Option<u64>) -> Vec<ThreadNode> {
    self.then(at, |acts| {
      let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), *act)).collect();
      let mut chains: Vec<&str> = Vec::new();
      let mut on: HashMap<&str, Vec<&Act>> = HashMap::new();
      for act in acts.iter().copied().filter(|act| act.id != act.chain) {
        let chain = act.chain.as_str();
        if hidden.iter().any(|one| one == chain) {
          continue;
        }
        on.entry(chain).or_insert_with(|| {
          chains.push(chain);
          Vec::new()
        });
        on.entry(chain).or_default().push(act);
      }
      let mut list: Vec<_> =
        chains.iter().flat_map(|chain| chain_threads(&on[chain], &rows)).collect();
      list.sort_by_key(|one| rows.get(one.prompt.as_str()).map_or(0, |act| act.began));
      order_threads(nest_threads(list, &rows), &rows)
    })
  }

  /// The open work of the life: each act that is not done and is no chain, that no other open work
  /// made, in the order the acts were first told. A cancel of each ends what each made too.
  pub fn open_work(&self) -> Vec<String> {
    let open = |act: &Act| !act.done() && act.kind != Kind::Chain;
    let work = |at: &usize| Some(&*self.acts[*at]).filter(|act| open(act));
    let made_by_work = |act: &Act| self.act(&act.by).is_some_and(open);
    self
      .live
      .iter()
      .filter_map(work)
      .filter(|act| !made_by_work(act))
      .map(|act| act.id.clone())
      .collect()
  }

  /// Each run of a word that is not done, in the order the acts were first told.
  pub fn running(&self) -> impl Iterator<Item = &Arc<Act>> {
    self.live.iter().map(|at| &self.acts[*at]).filter(|act| act.kind == Kind::Run && !act.done())
  }

  /// The acts as they stood just after a moment, in the order they began.
  pub fn as_of(&self, at: u64) -> Vec<Act> {
    let mut acts = as_of(self.acts.iter().map(|act| &**act), at);
    acts.sort_by_key(|act| act.began);
    acts
  }

  /// Each chain of the life in time, in the order the chains began: its label, the chain whose word
  /// made it, the state of its work now, the moments of its acts, and the moment each of its
  /// threads began.
  pub fn tracks(&self) -> Vec<ChainTrack> {
    self.then(None, |acts| {
      let chains = acts.iter().filter(|act| act.kind == Kind::Chain);
      chains
        .map(|chain| {
          let on: Vec<&Act> = self.on_chain(&chain.id).filter(|act| act.id != chain.id).collect();
          // A prompt on the chain itself starts a thread; one on a prompt is a message of it.
          let starts = on.iter().filter(|act| act.kind == Kind::Prompt && act.on == chain.id);
          // A prompt that a word of another chain asked is a link between their tracks, and one
          // that the operator said has no act as its maker.
          let asker = |act: &Act| {
            let maker = (&self).act(&act.by).map(|maker| maker.chain.clone());
            maker.filter(|one| *one != chain.id)
          };
          let says = |act: &Act| {
            let text = act.words.get(1).and_then(Value::as_str).unwrap_or_default();
            text.lines().find(|line| !line.trim().is_empty()).unwrap_or_default().to_owned()
          };
          let mut threads: Vec<Began> = starts
            .map(|act| Began {
              prompt: act.id.clone(),
              at: act.began,
              by: asker(act),
              ended: act.ended,
              says: says(act),
            })
            .collect();
          threads.sort_by_key(|one| one.at);
          ChainTrack {
            chain: chain.id.clone(),
            label: chain
              .words
              .first()
              .and_then(Value::as_str)
              .unwrap_or_default()
              .trim()
              .to_owned(),
            parent: (&self).act(&chain.by).map(|maker| maker.chain.clone()),
            state: chain.state,
            moments: moments_of(on.iter().copied()),
            threads,
          }
        })
        .collect()
    })
  }

  /// Each moment at which an act began or ended, of the life, of the acts of one chain, or of the
  /// acts that the page of one thread shows.
  pub fn moments(&self, on: Option<&str>) -> Vec<u64> {
    match on.and_then(|id| self.act(id)) {
      Some(thread) if thread.kind == Kind::Prompt => self.thread_moments(&thread.id),
      _ => moments_of(
        self.acts.iter().map(|act| &**act).filter(|act| on.is_none_or(|chain| act.chain == chain)),
      ),
    }
  }
}
