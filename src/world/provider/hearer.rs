//! The ear of the provider: one method for each event that it hears, and the replies that it took.

use std::{
  collections::{HashMap, HashSet},
  path::PathBuf,
  sync::{Arc, PoisonError, RwLock},
};

use futures::{FutureExt, stream};
use monty::ReplContinuationMode;
use rig_core::completion::CompletionError;
use tokio::sync::{mpsc, watch as holding};

use super::{
  Advertised, Mode, Provider, Refresh, Retries, Search, Settings, Told, Writes,
  answer::{Answering, answered, cut},
  images::Images,
  model::{Ask, Model, requested, standing},
  retry::{Backoff, OVERFLOW, Retry, failure, overflows, recoverable},
  search::free,
  thinking::{Thinking, thinks},
  watch::{self, Watch},
  word::{AFTER, Found, RAN, STOPPED, afterward, ending, judged, kept, unquoted, writing},
};
use crate::fact;
use crate::skill;
use crate::value;
use crate::{
  ear::{Co, say, spawned},
  engine::OPERATOR,
  fact::Fact,
  value::{Fault, Object},
};
use std::fmt;
use std::mem;

/// The ear of the provider as it hears a life: what the provider stands on, which a refresh
/// changes, the replies that it took, and the stands and the searches that it answers.
pub(super) struct Hearer {
  life: String,
  directory: String,
  config: Option<PathBuf>,
  extensions: Option<Vec<Advertised>>,
  models: Arc<RwLock<Vec<Model>>>,
  actor: String,
  thinking: Thinking,
  images: Images,
  writes: Option<Writes>,
  backoff: Backoff,
  retries: Option<Retries>,
  refresh: Option<Refresh>,
  gone: Vec<String>,
  members: Vec<String>,
  mode: Mode,
  /// The system prompt of each request, as [`Provider::system`] sets it.
  system: Arc<str>,
  native: Search,
  replies: Replies,
  /// Whether the ear heard a stand. The opening of the life loaded the settings just now, so the
  /// first stand, which the boot asks, reads them again only after it: a refresh there asks each
  /// program of a model, as the claude command line, a second time.
  stood: bool,
  /// The stand that the ear answers next with the standing, and whether a word made it.
  stands: Option<(String, bool)>,
  /// The chains that the ear told which names left the catalog.
  lacked: HashSet<String>,
}

impl Hearer {
  pub(super) fn new(provider: Provider, life: String) -> Self {
    let native = provider.search();
    let Provider {
      directory,
      config,
      extensions,
      models,
      actor,
      thinking,
      images,
      writes,
      backoff,
      retries,
      refresh,
      gone,
      members,
      mode,
      system,
    } = provider;
    let actor = actor.unwrap_or_else(|| {
      models
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .first()
        .map_or_else(|| OPERATOR.to_owned(), |model| model.at(None))
    });
    Self {
      life,
      directory,
      config,
      extensions,
      models,
      actor,
      thinking,
      images: Images::new(images),
      writes,
      backoff,
      retries,
      refresh,
      gone,
      members,
      mode,
      system,
      native,
      replies: Replies::default(),
      stood: false,
      stands: None,
      lacked: HashSet::new(),
    }
  }

  pub(super) async fn heard(&mut self, co: &mut Co<Replied>, a: &Fact) -> Result<(), Fault> {
    match a.kind() {
      "stand" if a.question() => self.stand(co, a),
      "reply" if a.question() => self.reply(co, a).await?,
      RAN => self.ran(co, a).await,
      "done" => self.done(a),
      "search" if a.question() => self.search(co, a),
      "pause" | "wake" => self.held(co).await,
      _ => {}
    }
    Ok(())
  }

  /// A stand that a word makes, as the stand of a setup, reads the standing alone: only a stand
  /// from outside an act reads the settings again, so a setup or a new standing on many chains
  /// asks no command line.
  fn stand(&mut self, co: &mut Co<Replied>, a: &Fact) {
    let about = a.about().to_owned();
    if self.mode == Mode::Held {
      say(co, Fact::says("started", &about, []));
      return;
    }
    let word = fact::named(a.by(), "rung");
    let first = !mem::replace(&mut self.stood, true);
    if let Some(refresh) = self.refresh.as_ref().filter(|_| !first && !word) {
      // Reading the settings asks each program of a model, which can take seconds, so the
      // stand awaits them as a task of the runtime of the ears, and every chain of the life
      // goes on meanwhile.
      say(co, Fact::says("started", &about, []));
      let read = spawned(refresh());
      co.take(
        a.about(),
        stream::once(read.map(move |got| {
          let got = got.map_err(|no| no.to_string()).and_then(|got| got);
          Replied::Refreshed { about, got }
        })),
      );
      return;
    }
    if !self.gone.is_empty() && self.lacked.insert(a.on().to_owned()) {
      told(co, a.on(), &departed(&about, &self.gone));
    }
    self.stands = Some((about, word));
  }

  /// The settings that a refresh read, on which the provider stands from now on, or why it read
  /// none.
  pub(super) fn refreshed(
    &mut self,
    co: &mut Co<Replied>,
    about: String,
    got: Result<Settings, String>,
  ) {
    match got {
      Ok((fresh, default, budget)) => {
        if !fresh.iter().any(|model| model.settings(&self.actor).is_some()) {
          self.actor = default
            .or_else(|| fresh.first().map(|model| model.at(None)))
            .unwrap_or_else(|| OPERATOR.to_owned());
        }
        *self.models.write().unwrap_or_else(PoisonError::into_inner) = fresh;
        self.thinking = budget;
        self.stands = Some((about, false));
      }
      Err(why) => say(co, Fact::says("done", &about, [Fault::refused(why).object()])),
    }
  }

  /// The stand that waits, answered with the standing. A stand reads the settings again, as after
  /// a change of the command line or of its account, or when the operator asks to retry now, so a
  /// reply whose call waits for a retry, which can wait for days at a usage limit, ends its wait
  /// and asks again at once, on the models that stand now.
  pub(super) async fn answer(&mut self, co: &mut Co<Replied>) -> Result<(), Fault> {
    let Some((about, word)) = self.stands.take() else { return Ok(()) };
    let given = standing(
      &self.models.read().unwrap_or_else(PoisonError::into_inner),
      &self.directory,
      &self.actor,
      self.extensions.as_deref(),
      &self.members,
      &self.config.clone().unwrap_or_else(|| skill::Places::here().config),
    );
    say(co, Fact::says("done", &about, [given]));
    if word {
      return Ok(());
    }
    let waiting: Vec<String> = self
      .replies
      .taken
      .iter()
      .filter(|(_, reply)| reply.waits)
      .map(|(id, _)| id.clone())
      .collect();
    for id in waiting {
      let Some(reply) = self.replies.taken.get(&id) else { continue };
      let (chain, actor) = (reply.chain.clone(), reply.actor.clone());
      let asking = Asking { about: &id, chain: &chain, actor: &actor, again: true };
      let call = self.asking(co, asking).await?;
      if let Some(reply) = self.replies.taken.get_mut(&id) {
        reply.waits = false;
        reply.call = call;
      }
    }
    Ok(())
  }

  async fn reply(&mut self, co: &mut Co<Replied>, a: &Fact) -> Result<(), Fault> {
    let about = a.about().to_owned();
    say(co, Fact::says("started", &about, []));
    let chain = a.on().to_owned();
    let actor = a.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
    let asking = Asking { about: &about, chain: &chain, actor: &actor, again: false };
    let call = self.asking(co, asking).await?;
    let again = self.replies.mute.get(&chain) == Some(&actor);
    let rung = a.by().to_owned();
    let reply = Reply { chain, actor, rung, again, waits: false, noted: None, call };
    self.replies.taken.insert(about, reply);
    Ok(())
  }

  /// The call of the model of a reply, begun on the turns of its chain and the models that stand
  /// now, as a work of the ear.
  async fn asking(&mut self, co: &mut Co<Replied>, asking: Asking<'_>) -> Result<Call, Fault> {
    let Asking { about, chain, actor, again } = asking;
    let turns = co.call("turns", vec![], vec![("on", Object::string(chain))]).await?;
    let (tells, told) = mpsc::unbounded_channel();
    let wrote = written(about, &tells);
    let key = format!("{}/{chain}", self.life);
    let asked = {
      let held = self.models.read().unwrap_or_else(PoisonError::into_inner);
      let asked =
        Ask { system: &self.system, actor, chain, turns: turns.as_ref(), conversation: &key };
      requested(&held, asked, &mut self.images, wrote).map(|one| thinks(one, self.thinking))
    };
    let streams = asked.as_ref().is_ok_and(|(model, _)| model.streams_its_word());
    let paused = co.call("paused", vec![Object::string(about)], vec![]).await;
    let paused = paused.ok().and_then(|one| one.as_ref().as_bool()).unwrap_or(false);
    let (hold, holds) = holding::channel(paused);
    let id = about.to_owned();
    let backoff = self.backoff;
    co.take(about, answered(Answering { id, asked, backoff, tells, told, hold: holds, again }));
    Ok(Call { watch: streams.then(Watch::default), hold })
  }

  /// What the model of a reply that still stands writes, told to the host under the rung it writes
  /// for, and each new part of its word that the gate finds nothing against, said as a writing of
  /// that rung.
  pub(super) async fn wrote(&mut self, co: &mut Co<Replied>, wrote: Wrote) {
    let Wrote { id, text, thinking } = wrote;
    let Some(reply) = self.replies.taken.get_mut(&id) else { return };
    reply.waits = false;
    if let Some(writes) = &self.writes {
      writes(&reply.rung, &reply.chain, &text, &thinking);
    }
    let (chain, rung) = (reply.chain.clone(), reply.rung.clone());
    let Some(watch) = reply.call.watch.as_mut() else { return };
    let Some(end) = watch.wrote(&text).filter(|_| !text.is_empty()) else { return };
    // The statements before the last line at the left edge, as python: each quote in them bound
    // as a string.
    let read = watch.upto(end).to_owned();
    if read.contains(watch::QUOTE) && watch::open(&read) {
      return;
    }
    let Some(word) = unquoted(co, &read).await else { return };
    let (line, why) = match judged(co, &chain, &word).await {
      Found::Refused(line, why) => (line, why),
      Found::Unsettled => return,
      Found::Nothing => {
        // The word holds no fault so far, so its new statements run now, as the turn holds them:
        // from its first line that is not blank, once monty calls them complete. A statement whose
        // clause starts at the end, as an `else`, waits, and so does a word that monty calls
        // incomplete or invalid, which the gate of a config turned off finds nothing against.
        let (word, end) = match watch.clause(end).then(|| watch::last(&word)).flatten() {
          Some(line) => {
            let held: String = word.split_inclusive('\n').take(line - 1).collect();
            (held, watch.start(line))
          }
          None if watch.clause(end) => return,
          None => (word, end),
        };
        if watch::complete(&word) != ReplContinuationMode::Complete {
          return;
        }
        let given = word.trim_start();
        let Some((part, before)) = watch.part(given) else { return };
        if writing(co, &rung, part, before).await {
          watch.gave(given, end);
        } else if watch.ran().is_none() {
          // A word whose first part a pause held is given whole, so no later part of it is given.
          watch.untaken();
        }
        return;
      }
    };
    // The word as far as the statement that cannot be python, or that the gate refuses for good,
    // is the turn of the reply, so the gate refuses it and the chain tells why; and once a part of
    // the word ran, the turn is the word that ran, which the chain runs as it ran, and the chain
    // is told why before it. The model wrote no more of it that counts, and its call ends with the
    // reply.
    let ran = watch.ran().map(str::to_owned);
    let Ok(turn) = cut(ran.as_deref().unwrap_or_else(|| watch.through(line))) else { return };
    let note = ran.map(|ran| (AFTER, [("findings", afterward(&why)), ("ran", ending(&ran))]));
    self.replies.end(co, &id, Ending { turn, note }).await;
  }

  /// A retry of the call of the model of a reply that still stands, told to the host under its
  /// rung.
  pub(super) async fn retried(&mut self, co: &mut Co<Replied>, retried: Retried) {
    let Retried { id, retry, limit } = retried;
    let Some(reply) = self.replies.taken.get_mut(&id) else { return };
    // A model asked again writes another word, and a part of this one ran, so the reply ends with
    // the word that ran, and the model is not asked again.
    if let Some(ran) = reply.call.watch.as_ref().and_then(Watch::ran).map(str::to_owned)
      && let Ok(turn) = cut(&ran)
    {
      let note = Some((STOPPED, [("why", retry.why.clone()), ("ran", ending(&ran))]));
      return self.replies.end(co, &id, Ending { turn, note }).await;
    }
    reply.waits = true;
    if let Some(watch) = reply.call.watch.as_mut() {
      watch.forget();
    }
    if let Some(retries) = &self.retries {
      retries(&reply.rung, &reply.chain, &retry);
    }
    // A wait for the end of a limit lasts hours, so the chain is told once, before it. A stand
    // asks the call again, and a call that meets the same wait tells nothing more.
    if !limit {
      return;
    }
    let chain = &reply.chain;
    let other = if retry.why.starts_with("usage limit") {
      format!(
        ". To go on with another actor, say rung(\"actor = '<model>'\", on=\"{chain}\") and wake(\"{chain}\")."
      )
    } else {
      String::new()
    };
    let note = format!("#{chain} waits on {}: {}{other}", reply.actor, retry.why);
    if reply.noted.as_deref() != Some(note.as_str()) {
      told(co, &reply.chain, &note);
      reply.noted = Some(note);
    }
  }

  /// What the model of a reply that still stands came to.
  pub(super) async fn came(&mut self, co: &mut Co<Replied>, came: Came) {
    let Came { id, got, dropped } = came;
    let Some(reply) = self.replies.taken.get(&id) else { return };
    let Some(got) = kept(co, reply, got).await else { return };
    // The word that runs is not all the text that the model wrote, so the chain is told what went,
    // before it.
    let wrapped = reply.call.watch.as_ref().and_then(Watch::note);
    if let Some(dropped) = dropped.or(wrapped) {
      told(co, &reply.chain, &format!("#{} {dropped}", reply.rung));
    }
    let recoverable = got.as_ref().is_err_and(recoverable);
    let got = got.map_err(|no| {
      let over = if overflows(&no) { format!("{OVERFLOW}: ") } else { String::new() };
      format!("{} answered nothing: {over}{}", reply.actor, failure(&no))
    });
    if let (Err(why), true) = (&got, reply.again)
      && !recoverable
    {
      let paused =
        co.call("pause", vec![Object::string(&reply.chain), Object::string(why)], vec![]).await;
      if let Err(no) = paused {
        let note = format!("#{} could not pause the chain: {}", reply.rung, no.message());
        told(co, &reply.chain, &note);
      }
    }
    let got = got.map_err(Fault::refused).unwrap_or_else(|fault| fault.object());
    say(co, Fact::says("done", &id, [got]));
  }

  /// A part of a word that raised ends the word where it raised, so its reply ends with the word
  /// that ran, whose run is done with that raise.
  async fn ran(&mut self, co: &mut Co<Replied>, a: &Fact) {
    if a.word(1).is_none_or(|one| one.type_name() == "NoneType") {
      return;
    }
    let mine = self.replies.taken.iter().find(|(_, reply)| reply.rung == a.about());
    let Some((id, reply)) = mine else { return };
    let Some(Ok(turn)) = reply.call.watch.as_ref().and_then(Watch::ran).map(cut) else { return };
    let id = id.clone();
    self.replies.end(co, &id, Ending { turn, note: None }).await;
  }

  /// A done of a reply ends it. A done that the ear said of its own reply ends the row of
  /// refusals on the chain with a turn, or adds to it with a refusal, which is what the ear says
  /// when it says no turn; a done that a control said leaves the row as it is.
  fn done(&mut self, a: &Fact) {
    let Some(Reply { chain, actor, .. }) = self.replies.taken.remove(a.about()) else { return };
    if a.by() == a.about() {
      return;
    }
    let turned = a
      .word(0)
      .and_then(|one| one.items())
      .is_some_and(|turn| turn.len() == 4 && turn[0].as_str() == Some("assistant"));
    if turned {
      self.replies.mute.remove(&chain);
    } else {
      self.replies.mute.insert(chain, actor);
    }
  }

  /// A search of the web: the native search of the provider of its actor, else the free search,
  /// as a work of the ear, so every chain of the life goes on meanwhile. A life that inspects
  /// searches nothing.
  fn search(&mut self, co: &mut Co<Replied>, a: &Fact) {
    let about = a.about().to_owned();
    let word = |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
    let (query, actor) = (word(1).unwrap_or_default(), word(2).unwrap_or_default());
    if self.mode != Mode::Live {
      let no = "a life that inspects searches nothing";
      say(co, Fact::says("done", &about, [Fault::refused(no).object()]));
      return;
    }
    say(co, Fact::says("started", &about, []));
    let search = (self.native)(&actor).unwrap_or_else(free);
    let id = about.clone();
    let asked = async move {
      let got = search(query.clone()).await;
      Replied::Searched {
        id,
        got: got.map(|answer| value::Text::new(query, answer)).map_err(|no| no.to_string()),
      }
    };
    co.take(&about, stream::once(asked));
  }

  /// A pause or a wake changes what holds a reply, so each reply that stands reads whether a pause
  /// holds it now, and a call that waits to be made again goes on at the wake.
  async fn held(&mut self, co: &mut Co<Replied>) {
    let ids: Vec<String> = self.replies.taken.keys().cloned().collect();
    for id in ids {
      let held = co.call("paused", vec![Object::string(&id)], vec![]).await;
      let held = held.ok().and_then(|one| one.as_ref().as_bool()).unwrap_or(false);
      if let Some(reply) = self.replies.taken.get(&id) {
        reply.call.hold.send_replace(held);
      }
    }
  }
}

/// The replies that the ear took and that still stand, by id, and the actor whose last reply on
/// each chain the ear refused.
#[derive(Default)]
struct Replies {
  taken: HashMap<String, Reply>,
  mute: HashMap<String, String>,
}

impl Replies {
  /// The end of a reply that still stands with the word that ran: its chain is told what its model
  /// wrote that the word lost, then why the reply ends there, and the reply is done with its turn,
  /// which ends the row of refusals on its chain.
  async fn end(&mut self, co: &mut Co<Replied>, id: &str, ending: Ending) {
    let Some(Reply { chain, rung, call, .. }) = self.taken.remove(id) else { return };
    self.mute.remove(&chain);
    if let Some(note) = call.watch.as_ref().and_then(Watch::note) {
      told(co, &chain, &format!("#{rung} {note}"));
    }
    if let Some((what, fields)) = ending.note {
      noted(co, &chain, &rung, what, &fields).await;
    }
    say(co, Fact::says("done", id, [ending.turn]));
  }
}

/// How a reply ends with the word that ran: the turn that it is done with, and the note that tells
/// its chain why, after its rung, as what the note says and its fields.
struct Ending {
  turn: Object,
  note: Option<(&'static str, [(&'static str, String); 2])>,
}

/// A reply the ear took: its chain, its actor, the rung it answers, whether a prior refusal of it
/// pauses its chain, whether its call waits for a retry, and the call of its model.
pub(super) struct Reply {
  pub(super) chain: String,
  actor: String,
  pub(super) rung: String,
  again: bool,
  waits: bool,
  /// The last note that told the chain of a wait, so the same wait asked again after a stand is
  /// told once.
  noted: Option<String>,
  pub(super) call: Call,
}

/// A call of the model of a reply that the ear makes: the id of the reply, its chain, its actor,
/// and whether it asks again a call that waits for a retry, which a pause holds until the wake.
struct Asking<'a> {
  about: &'a str,
  chain: &'a str,
  actor: &'a str,
  again: bool,
}

/// The call of the model of a reply that works: the watch over its word while its model writes
/// it, when the model streams its word, and what tells the call whether a pause holds its reply.
pub(super) struct Call {
  pub(super) watch: Option<Watch>,
  /// Whether a pause holds the reply, which holds the next call of its model: a call that runs
  /// returns, and a call that waits to be made again waits for the wake.
  hold: holding::Sender<bool>,
}

/// What a search came to: its text, or why it gives none.
pub(super) fn searched(co: &mut Co<Replied>, id: &str, got: Result<value::Text, String>) {
  let got = got.map_or_else(|no| Fault::refused(no).object(), |one| one.object());
  say(co, Fact::says("done", id, [got]));
}

/// The wait of a call of a model to be made again while a pause holds its reply, which the wake
/// ends. A reply that went has no pause to wait for, so its call ends.
pub(super) async fn released(hold: &mut holding::Receiver<bool>) -> Result<(), CompletionError> {
  hold.wait_for(|paused| !*paused).await.map(drop).map_err(reply_gone)
}

/// The failure of a call of a model whose reply went: no one reads what it would give.
pub(super) fn reply_gone(why: impl fmt::Display) -> CompletionError {
  CompletionError::ProviderError(format!(
    "the reply went before its call of the model ended: {why}"
  ))
}

/// What the work of a reply gives: a part that its model wrote, a retry of the call of its model,
/// or what the reply came to, and what the chain is told when its turn lost marks of its text: the
/// marks of a fence, of a tool call or of a break of HTML, and the name that a binding of no name
/// lacked, which [`watch::bare`] drops and gives, or the line of three backticks at the end of its
/// word, which [`unclosed`](super::answer::unclosed) drops.
pub(super) enum Replied {
  Wrote(Wrote),
  Retried(Retried),
  Came(Came),
  Refreshed { about: String, got: Result<Settings, String> },
  Searched { id: String, got: Result<value::Text, String> },
}

/// A part that the model of a reply wrote: the id of the reply, and what the part added to its
/// text and to its thought.
pub(super) struct Wrote {
  id: String,
  text: String,
  thinking: String,
}

/// A retry of the call of the model of a reply: the id of the reply, the retry, and whether the
/// chain of the reply is told of its wait.
pub(super) struct Retried {
  pub(super) id: String,
  pub(super) retry: Retry,
  pub(super) limit: bool,
}

/// What the call of the model of a reply came to: the id of the reply, its turn or why it gives
/// none, and the marks that its turn lost.
pub(super) struct Came {
  pub(super) id: String,
  pub(super) got: Result<Object, CompletionError>,
  pub(super) dropped: Option<String>,
}

/// A note told to a chain.
pub(super) fn told(co: &mut Co<Replied>, chain: &str, note: &str) {
  say(co, Fact::says("tell", chain, [Object::list([Object::string(note)])]));
}

/// The note that tells a chain which names of the roster and of the actor left the catalog, as a
/// stand names it.
fn departed(about: &str, names: &[String]) -> String {
  let quote = |name: &String| format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"));
  let list = names.iter().map(quote).collect::<Vec<_>>().join(", ");
  format!("#{about} names left the catalog\n{about}_left = [{list}]")
}

/// A note of the chain about the turn of a rung: its header, then each text that the gate or the
/// call gave, which the engine binds under the rung as it binds a value of an act, so the turn that
/// holds the note stays python. A text after the header as a line of its own is no python, and the
/// run of the turn would raise a `SyntaxError`.
pub(super) async fn noted(
  co: &mut Co<Replied>,
  chain: &str,
  rung: &str,
  what: &str,
  fields: &[(&str, String)],
) {
  let words = vec![Object::string(rung), Object::string(what)];
  let fields = fields.iter().map(|(word, text)| (*word, Object::string(text))).collect();
  let got = co.call("headed", words, fields).await;
  let note = got.ok().and_then(|one| one.as_ref().as_str().map(str::to_owned));
  told(co, chain, &note.unwrap_or_else(|| format!("#{rung} {what}")));
}

/// What a turn tells as it streams, sent to the work of its reply, which tells the host and reads
/// the word.
pub(super) fn written(id: &str, tells: &mpsc::UnboundedSender<Replied>) -> Told {
  let (id, tells) = (id.to_owned(), tells.clone());
  Arc::new(move |text, thinking| {
    let (text, thinking) = (text.to_owned(), thinking.to_owned());
    // A reply that went has no reader for what its model still writes.
    drop(tells.send(Replied::Wrote(Wrote { id: id.clone(), text, thinking })));
  })
}
