//! The window on an app: its widgets, and what each key, each move of the pointer, each timer and
//! each frame of a machine changes in its model.

use std::{
  cell::RefCell,
  mem,
  path::PathBuf,
  rc::Rc,
  time::{Duration, Instant},
};

use unicode_width::UnicodeWidthStr;

use opentui::{
  App,
  input::KeyEventKind,
  prelude::{
    Container, Edge, EditorAction, ExtmarkOptions, KeyCode, KeyEvent, Modifiers, MouseButton,
    MouseEvent, MouseEventKind, Span, Text, Textarea, TimerId, View, Widget, WrapMode,
  },
};

use furb_host::protocol::Verb;

use furb_client::Asked;

use crate::{
  Machine, Options, Screen,
  input::Armed,
  kept,
  keys::{self, Does},
  line::Line,
  link::{self, Asker, Heard},
  model::{Model, Told},
  pick::Pick,
  question::Keyed,
  tokens::{AWAY, Role, SIDE, Tokens},
};

/// How long the window waits for a machine before it says that it waits.
const PATIENCE: Duration = Duration::from_millis(400);

/// How long the leader key waits for the key that follows it, and the first escape for the second.
const LEADER: Duration = Duration::from_secs(2);

/// How long the window waits after a change of the cards before it writes them, so the changes of
/// one moment are one write.
const KEEPS: Duration = Duration::from_secs(1);

/// The step of the one timer of the loaders, which each step of a loader is a count of.
const MOVES: Duration = Duration::from_millis(40);

/// How often the window reads the clock for the age of each card.
const AGES: Duration = Duration::from_secs(20);

/// How many loaders move at once at most: the line that runs, the agents of the dock, and the
/// row of hints.
const LOADERS: usize = 8;

/// How many rows one step of the wheel moves the list.
const WHEEL: isize = 3;

struct Window {
  screen: Screen<Model>,
  clock: fn() -> u64,
  /// Whether the clocks of the window stand still.
  still: bool,
  kept: PathBuf,
  /// When the window opened, on the clock of the app.
  born: Instant,
  /// When the hand was last on the list.
  touched: Option<Instant>,
  /// When the leader key was pressed, until the key that follows it.
  led: Option<Instant>,
  /// The editor of the text of the input, which has the focus.
  area: Textarea,
  /// Whether a timer waits to settle the list.
  settles: bool,
  /// Whether a timer waits to write the cards.
  keeps: bool,
  /// The timer of the loaders, while a mark moves.
  moves: Option<TimerId>,
  /// What asks each machine a verb, in the order of the machines.
  askers: Vec<Asker>,
}

type Shared = Rc<RefCell<Window>>;

/// The window drawn on an app, with the cards that the window of before kept, and each machine
/// dialed.
pub(crate) fn opened(app: &App, machines: Vec<Machine>, options: Options) {
  let tokens = Tokens::on(app.palette().and_then(|palette| palette.background));
  app.set_theme(tokens.theme());
  let mut before = kept::kept(&options.kept);
  let told = |machine: &Machine| {
    let kept =
      before.iter().position(|one| one.name == machine.name).map(|at| before.swap_remove(at));
    kept.unwrap_or_else(|| Told { name: machine.name.clone(), ..Told::default() })
  };
  let told = machines.iter().map(told).collect();
  let mut model = Model::new(told, app.size(), tokens, &options);
  model.client.now = (options.clock)();
  model.settle();
  let area = edited(app, &tokens);
  let window = Window {
    screen: drawn(app, model, &area),
    clock: options.clock,
    still: options.still,
    kept: options.kept,
    born: app.now(),
    touched: None,
    led: None,
    area: area.clone(),
    settles: false,
    keeps: false,
    moves: None,
    askers: Vec::new(),
  };
  let window = Rc::new(RefCell::new(window));

  let on = Rc::clone(&window);
  app.on_key_capture(move |cx, pressed| {
    if key(cx.app(), &on, pressed) {
      cx.stop_propagation();
    }
  });
  let on = Rc::clone(&window);
  area.on_change(move |cx, text| {
    let text = text.to_owned();
    tell(cx.app(), &on, |model| {
      // A word that he writes leaves what a first press of a key named.
      if !text.is_empty() {
        model.armed = None;
      }
      model.edited(text);
    });
    colored(&on);
  });
  let on = Rc::clone(&window);
  app.on_mouse(move |cx, event| mouse(cx.app(), &on, *event));
  let on = Rc::clone(&window);
  app.on_resize(move |cx, width, height| {
    tell(cx.app(), &on, |model| model.resized((width, height)));
  });
  let on = Rc::clone(&window);
  app.set_timeout(PATIENCE, move |cx| tell(cx.app(), &on, |model| model.slow = true));
  let on = Rc::clone(&window);
  app.set_interval(AGES, move |cx| tick(cx.app(), &on));
  for (at, machine) in machines.into_iter().enumerate() {
    let on = Rc::clone(&window);
    let asker = link::dialed(app, machine, move |app, heard| hear(app, &on, at, heard));
    window.borrow_mut().askers.push(asker);
  }
}

/// The editor of the text of the input, which has the focus. Enter sends, so a line breaks with
/// shift and enter, and the end goes to the end of the feed, as the table of the keys says.
fn edited(app: &App, tokens: &Tokens) -> Textarea {
  let area = Textarea::new(app).absolute().wrap(WrapMode::Word);
  let area = area.colors(tokens.text, tokens.raised).focused_colors(tokens.text, tokens.raised);
  let area = area.placeholder_color(tokens.muted).cursor_color(tokens.text);
  area.bind("shift+enter", EditorAction::Newline).unbind("end")
}

/// The widgets of the window on a model: the left side, the editor of the input over it, and the
/// list at its right. Each side is one text, which is written only when a row of it changes.
fn drawn(app: &App, model: Model, area: &Textarea) -> Screen<Model> {
  let tokens = model.tokens;
  let column = || Text::new(app, "").wrap(WrapMode::None);
  let (home, list) = (column(), column());
  // The list stands on a panel, so a change of ground parts it from the home, with no line.
  let side = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Right, 0);
  let side =
    side.position(Edge::Bottom, 0).width(SIDE).bg(tokens.panel).padding_edge(Edge::Left, 1);
  let side = side.child(&list);
  app.root().child(&home).child(area).child(&side);
  app.focus(area);

  let mut screen = Screen::new(model);
  screen.part(
    |model: &Model| model.left().rows,
    move |lines| {
      home.set_text(written(lines));
    },
  );
  placed(app, &mut screen, area);
  // A loader is a cell of its own, so its step draws no other cell.
  let cells: Vec<Text> =
    (0..LOADERS).map(|_| Text::new(app, "").absolute().visible(false)).collect();
  for cell in &cells {
    app.root().child(cell);
  }
  let frames = |model: &Model| {
    let spots = model.left().loaders;
    spots.into_iter().map(|spot| (spot, spot.loader.frame(model.client.moved))).collect::<Vec<_>>()
  };
  screen.part(frames, move |shown| {
    for (at, cell) in cells.iter().enumerate() {
      match shown.get(at) {
        Some((spot, frame)) => {
          let cell = cell.position(Edge::Top, spot.y).position(Edge::Left, spot.x);
          cell.set_text(vec![Span::styled(*frame, spot.style)]);
          cell.visible(true);
        }
        None => {
          cell.visible(false);
        }
      }
    }
  });
  screen.part(Model::list, move |lines| {
    list.set_text(written(lines));
  });
  screen.part(Model::listed, move |listed| {
    side.visible(*listed);
  });
  veiled(app, &mut screen, &tokens);
  screen
}

/// The list part over each other widget, on a veil that dims the whole window, while one is open.
fn veiled(app: &App, screen: &mut Screen<Model>, tokens: &Tokens) {
  let veil = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Left, 0);
  let veil = veil.position(Edge::Right, 0).position(Edge::Bottom, 0).bg(tokens.veil);
  let rows = Text::new(app, "").wrap(WrapMode::None);
  let part = View::new(app).absolute().bg(tokens.raised).child(&rows);
  app.root().child(&veil.z_index(1).visible(false)).child(&part.z_index(2).visible(false));
  screen.part(Model::drawn_over, move |over| {
    veil.visible(over.is_some());
    part.visible(over.is_some());
    if let Some((x, y, width, lines)) = over {
      let height = u16::try_from(lines.len()).unwrap_or_default();
      part.position(Edge::Left, *x).position(Edge::Top, *y).width(*width).height(height);
      rows.set_text(written(lines));
    }
  });
}

/// The lines of a text widget, one row each.
fn written(lines: &[Line]) -> Vec<Span<'static>> {
  let rows = lines.iter().enumerate().flat_map(|(n, line)| {
    let ended = (n > 0).then(|| Span::raw("\n"));
    ended.into_iter().chain(line.iter().cloned())
  });
  rows.collect()
}

/// The editor of the input on its box. A hidden editor loses the focus, which it takes back when it
/// shows again.
fn placed(app: &App, screen: &mut Screen<Model>, area: &Textarea) {
  let tokens = screen.model.tokens;
  let ghost = Text::new(app, "").absolute().wrap(WrapMode::None).z_index(1).visible(false);
  app.root().child(&ghost);
  screen.part(Model::ghost, move |shown| match shown {
    Some((x, y, rest)) => {
      let ghost = ghost.position(Edge::Left, *x).position(Edge::Top, *y);
      ghost.set_text(vec![Span::styled(rest.clone(), tokens.style(Role::Muted).bg(tokens.raised))]);
      ghost.visible(true);
    }
    None => {
      ghost.visible(false);
    }
  });
  let (area, focus) = (area.clone(), app.clone());
  screen.part(Model::area, move |shown| {
    let area = area.position(Edge::Top, shown.y).position(Edge::Left, shown.x);
    let area = area.width(shown.width).height(shown.height).placeholder(&shown.placeholder);
    if area.visible(shown.shown).is_visible() {
      focus.focus(&area);
    }
  });
}

/// A change of the model at this moment of the two clocks, which each part then shows. The loaders
/// move on one timer, which runs only while a mark moves.
fn tell<T>(app: &App, window: &Shared, change: impl FnOnce(&mut Model) -> T) -> T {
  let given = told(app, window, change);
  pump(window);
  let back = window.borrow_mut().screen.model.client.restore.take();
  if let Some(text) = back {
    window.borrow().area.set_text(text);
    colored(window);
  }
  given
}

/// A change of the model, which each part then shows, and the timer of the loaders.
fn told<T>(app: &App, window: &Shared, change: impl FnOnce(&mut Model) -> T) -> T {
  let mut held = window.borrow_mut();
  let moved =
    if held.still { Duration::ZERO } else { app.now().saturating_duration_since(held.born) };
  let now = (held.clock)();
  let given = held.screen.tell(|model| {
    (model.client.now, model.client.moved) = (now, moved);
    let given = change(model);
    model.questioned();
    given
  });
  match (held.screen.model.moves() && !held.still, held.moves) {
    (true, None) => {
      // The next step falls on the grid of the steps from the open of the window, so a late step
      // never shifts the frames after it and each loader keeps its beat.
      let late = u64::try_from(moved.as_millis() % MOVES.as_millis()).unwrap_or_default();
      let on = Rc::clone(window);
      let next = MOVES.saturating_sub(Duration::from_millis(late));
      held.moves = Some(app.set_timeout(next, move |cx| tick(cx.app(), &on)));
    }
    (false, Some(timer)) => {
      app.cancel_timer(timer);
      held.moves = None;
    }
    _ => {}
  }
  given
}

/// A moment of the clocks with no other change: the loaders and the ages move, and the next step
/// is set.
fn tick(app: &App, window: &Shared) {
  window.borrow_mut().moves = None;
  tell(app, window, |_| ());
}

/// What a machine told. A card shows its new state at once, and the list settles when the hand is
/// away.
fn hear(app: &App, window: &Shared, at: usize, heard: Heard) {
  let changed = tell(app, window, |model| model.hear(at, heard));
  if changed {
    rest(app, window);
    keep(app, window);
  }
}

/// The list settled: now when the hand is away from it, else once the hand was away for long
/// enough.
fn rest(app: &App, window: &Shared) {
  let touched = window.borrow().touched;
  let away = touched.map_or(AWAY, |at| app.now().saturating_duration_since(at));
  if away >= AWAY {
    tell(app, window, Model::settle);
  } else if !mem::replace(&mut window.borrow_mut().settles, true) {
    let on = Rc::clone(window);
    app.set_timeout(AWAY.saturating_sub(away), move |cx| {
      on.borrow_mut().settles = false;
      rest(cx.app(), &on);
    });
  }
}

/// The cards written to the file of the window, a moment after they changed.
fn keep(app: &App, window: &Shared) {
  if mem::replace(&mut window.borrow_mut().keeps, true) {
    return;
  }
  let on = Rc::clone(window);
  app.set_timeout(KEEPS, move |cx| {
    let written = {
      let mut held = on.borrow_mut();
      held.keeps = false;
      kept::keep(&held.kept, &held.screen.model.client.machines)
    };
    let fault = written.err().map(|fault| format!("The cards are not kept: {fault}"));
    tell(cx.app(), &on, |model| model.fault = fault);
  });
}

/// Each request that is due asked of its machine: the open of each life that the window holds,
/// the models of each machine, the page of the end of the open thread, the next message of the
/// queue of the input, what the operator saw of the open thread, and the end of the word that the
/// window ran, each with its id kept until its answer comes.
fn pump(window: &Shared) {
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  let model = &mut screen.model;
  for (life, verb) in model.client.opens_due() {
    let id = ask(askers, model, life.0, verb);
    model.client.asked_open(&life, id);
    model.client.asked.extend(id.map(|id| ((life.0, id), Asked::Open)));
  }
  for (at, verb) in model.client.models_due() {
    if let Some(id) = ask(askers, model, at, verb) {
      model.client.asked.insert((at, id), Asked::Models);
    }
  }
  if let Some(verb) = model.client.page_due()
    && let Some(at) = model.client.thread.as_ref().map(|one| one.thread.key.0)
    && let Some(id) = ask(askers, model, at, verb)
    && let Some(opened) = model.client.thread.as_mut()
  {
    opened.thread.asked.insert(id, None);
    opened.thread.again = false;
  }
  if let Some((at, verb)) = model.client.due()
    && let Some(id) = ask(askers, model, at, verb)
  {
    model.client.asked.insert((at, id), Asked::Say);
  }
  if let Some((at, verb, target)) = model.client.history_due()
    && let Some(id) = ask(askers, model, at, verb)
  {
    model.client.asked.insert((at, id), Asked::History(target));
  }
  if let Some((at, verb)) = model.client.seen_due()
    && let Some(id) = ask(askers, model, at, verb)
  {
    model.client.asked.insert((at, id), Asked::Seen);
  }
  if let Some((at, verb)) = model.client.chosen_due()
    && let Some(id) = ask(askers, model, at, verb)
  {
    model.client.asked.insert((at, id), Asked::Control);
  }
  if let Some((at, verb)) = model.ended_due()
    && let Some(id) = ask(askers, model, at, verb)
    && let Some(word) = model.word.as_mut()
  {
    word.asked = true;
    model.client.asked.insert((at, id), Asked::Ended);
  }
}

/// A verb asked of a machine through its connection, and the id of its request: none while the
/// machine does not list its cards, as the client core says.
fn ask(askers: &mut [Asker], model: &mut Model, at: usize, verb: Verb) -> Option<u64> {
  let asker = askers.get_mut(at)?;
  let request = model.client.request(at, verb)?;
  let id = request.id;
  asker.send(request);
  Some(id)
}

/// A page of older exchanges that the open thread asks of its machine, said, with the id of its
/// request kept until its answer comes.
fn older(window: &Shared, verb: Option<Verb>) {
  let Some(verb @ Verb::Thread { before, .. }) = verb else { return };
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  let Some(at) = screen.model.client.thread.as_ref().map(|one| one.thread.key.0) else { return };
  let Some(id) = ask(askers, &mut screen.model, at, verb) else { return };
  let Some(opened) = screen.model.client.thread.as_mut() else { return };
  opened.thread.asked.insert(id, before);
}

/// A key of the window, from the one table of the keys, before the editor of the input hears it:
/// whether the window took it. A key that follows the leader never reaches the editor.
fn key(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  if key.kind == KeyEventKind::Release {
    return false;
  }
  let led = window.borrow_mut().led.take();
  let led = led.is_some_and(|at| app.now().saturating_duration_since(at) <= LEADER);
  if keys::leads(key) {
    window.borrow_mut().led = Some(app.now());
    return true;
  }
  if window.borrow().screen.model.picker.is_some() && picking(app, window, key) {
    return true;
  }
  let does = keys::pressed(key, led);
  // A letter after the leader is a key of the window, and no letter of a text.
  let typed =
    !led && key.text().is_some() && (key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
  if answered(app, window, does, typed) {
    return true;
  }
  match does {
    Some(Does::Up | Does::Down) if walked(app, window, does == Some(Does::Up)) => true,
    // The grey rest, which the editor hears as a move when none shows.
    Some(Does::Take | Does::TakeWord) => took(window, does == Some(Does::TakeWord)),
    // The keys of a form, which the editor hears when no form takes them.
    Some(Does::Up | Does::Down | Does::Tick | Does::NextQuestion) => false,
    Some(Does::Leave) if !window.borrow().area.text().is_empty() => false,
    Some(does) => {
      act(app, window, does);
      true
    }
    None => led,
  }
}

/// An arrow up on the first row of the input, or down on its last row, which walks the history of
/// the input: whether the walk took it. An arrow on another row moves in the input.
fn walked(app: &App, window: &Shared, up: bool) -> bool {
  let (row, rows) = {
    let area = &window.borrow().area;
    (area.cursor().0, area.text().split('\n').count())
  };
  let edge = if up { row == 0 } else { usize::try_from(row).is_ok_and(|row| row + 1 >= rows) };
  edge && tell(app, window, |model| model.client.walked(up))
}

/// The grey rest of the input taken whole, or its first word, at the end of the text: whether a
/// rest showed there. The editor tells the change, as a text that he wrote.
fn took(window: &Shared, word: bool) -> bool {
  let held = window.borrow();
  let area = &held.area;
  let end = usize::try_from(area.cursor_offset()).is_ok_and(|at| at == area.text().chars().count());
  let Some(rest) = held.screen.model.client.suggested().filter(|_| end) else { return false };
  // A word is the blanks before it and its letters, as the editor moves by words.
  let start = rest.len() - rest.trim_start().len();
  let word_end = rest[start..].find(char::is_whitespace).map_or(rest.len(), |at| start + at);
  let rest = if word { &rest[..word_end] } else { rest };
  let (rest, area) = (rest.to_owned(), area.clone());
  // The editor tells the change at once, and the window then hears it.
  drop(held);
  area.insert(rest);
  true
}

/// A key while a list part is open: it filters the part, moves its chosen row, takes it or closes
/// it. Whether the part took the key.
fn picking(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  let text = key.text().filter(|_| key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
  let taken = tell(app, window, |model| {
    let at = model.picker.as_ref().map(|one| one.at)?;
    match (key.code, text) {
      (KeyCode::Escape, _) => model.unpicked(),
      // A backspace on a list that a slash opened, with no text that filters it, takes the slash.
      (KeyCode::Backspace, _)
        if model.picker.as_ref().is_some_and(|one| one.slashed && one.filter.is_empty()) =>
      {
        model.picker = None;
      }
      (KeyCode::Enter, _) => return Some(model.took(at)),
      (KeyCode::Up, _) => model.chose(-1),
      (KeyCode::Down, _) => model.chose(1),
      (KeyCode::Backspace, _) => model.filtered(|filter| {
        filter.pop();
      }),
      (_, Some(one)) => model.filtered(|filter| filter.push(one)),
      _ => return None,
    }
    Some(None)
  });
  let Some(does) = taken else { return false };
  if let Some(does) = does {
    act(app, window, does);
  }
  true
}

/// A key or a click while a form of a question shows: whether the form took it. An answer goes to
/// its machine, and waits there for its answer.
fn answered(app: &App, window: &Shared, does: Option<Does>, typed: bool) -> bool {
  // A key with no form tells the model nothing, so it draws no part again.
  if window.borrow().screen.model.form().is_none() {
    return false;
  }
  match tell(app, window, |model| model.keyed(does, typed)) {
    Keyed::Passed => false,
    Keyed::Taken => true,
    Keyed::Empties => {
      window.borrow().area.set_text("");
      tell(app, window, |model| model.client.draft.text.clear());
      true
    }
    Keyed::Says(at, verb) => {
      let mut held = window.borrow_mut();
      let Window { askers, screen, .. } = &mut *held;
      match ask(askers, &mut screen.model, at, verb) {
        Some(id) => drop(screen.model.client.asked.insert((at, id), Asked::Answer)),
        None => screen.model.unanswered(screen.model.client.away(at).unwrap_or_default()),
      }
      drop(held);
      tell(app, window, |_| ());
      true
    }
  }
}

/// What a key does, which a click on its text does too. Any other key leaves what a first press of
/// a key named, and what the key before did not do.
fn act(app: &App, window: &Shared, does: Does) {
  let forgot = window.borrow_mut().screen.model.pressed(does);
  asked(window, does);
  let change: fn(&mut Model) = match does {
    Does::Next => |model| model.step(1),
    Does::Previous => |model| model.step(-1),
    Does::Needed => Model::needed,
    Does::List => Model::toggle,
    Does::Jump => Model::jump,
    Does::NewThread => |model| model.client.turn(),
    Does::Sessions => |model| model.pick(Pick::Sessions),
    Does::GoTo => |model| model.pick(Pick::GoTo),
    Does::Commands => |model| model.pick(Pick::Commands),
    Does::Machines => |model| model.pick(Pick::Machines),
    Does::Projects => |model| model.pick(Pick::Projects),
    Does::History => |model| model.pick(Pick::History),
    Does::Model => |model| model.pick(Pick::Models),
    Does::Effort => Model::effort,
    Does::Keys => |model| model.pick(Pick::Keys),
    Does::Python => |model| model.client.draft.python = !model.client.draft.python,
    Does::Agent => Model::next_agent,
    Does::Close => return escaped(app, window, forgot),
    Does::Stop => return controlled(app, window, |model| Some((model.stop()?, Asked::Control))),
    Does::Resolve => {
      return controlled(app, window, |model| Some((model.resolve()?, Asked::Control)));
    }
    Does::Send => return sent(app, window),
    Does::Empty if !window.borrow().area.text().is_empty() => {
      window.borrow().area.set_text("");
      return tell(app, window, |model| model.client.draft.text.clear());
    }
    Does::Empty => {
      return controlled(app, window, |model| {
        model.take_back().map(|(asked, text)| (asked, Asked::Back(text)))
      });
    }
    Does::Leave => {
      if tell(app, window, |model| model.armed.replace(Armed::Leave) == Some(Armed::Leave)) {
        app.quit();
      }
      return;
    }
    Does::PageUp | Does::PageDown => {
      // A page keeps two rows of the screen before it in view.
      let verb = tell(app, window, |model| {
        let rows = isize::try_from(model.feed_room()).unwrap_or_default() - 2;
        model.scroll_feed(if does == Does::PageUp { -rows } else { rows })
      });
      return older(window, verb);
    }
    // The keys of a form, of a list part and of the input, which the key that heard them did.
    Does::Up | Does::Down | Does::Tick | Does::NextQuestion | Does::Take | Does::TakeWord => return,
  };
  if matches!(does, Does::Next | Does::Previous | Does::Needed) {
    window.borrow_mut().touched = Some(app.now());
  }
  tell(app, window, |model| {
    change(model);
    model.open_thread();
  });
  colored(window);
}

/// What a list part shows asked of each machine as it opens: the records of each machine for Go
/// to, and the projects of each machine, or of the machine of the input, for their pickers.
fn asked(window: &Shared, does: Does) {
  match does {
    Does::GoTo => listed(window),
    Does::Machines => projected(window, None),
    Does::Projects => {
      let at = window.borrow().screen.model.client.target().machine();
      projected(window, Some(at));
    }
    _ => {}
  }
}

/// The records of each machine that lists its cards asked, for Go to.
fn listed(window: &Shared) {
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  for at in 0..askers.len() {
    if let Some(id) = ask(askers, &mut screen.model, at, Verb::List { config: None }) {
      screen.model.client.asked.insert((at, id), Asked::List);
    }
  }
}

/// The projects of a machine asked, or of each machine that lists its cards, for the parts that
/// choose where a new session runs.
fn projected(window: &Shared, of: Option<usize>) {
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  for at in (0..askers.len()).filter(|at| of.is_none_or(|of| of == *at)) {
    if let Some(id) = ask(askers, &mut screen.model, at, Verb::Projects { config: None }) {
      screen.model.client.asked.insert((at, id), Asked::Projects);
    }
  }
}

/// The text of the input sent to its target, and the input empty.
fn sent(app: &App, window: &Shared) {
  let text = window.borrow().area.text();
  if text.trim().is_empty() || !tell(app, window, |model| model.send(text)) {
    return;
  }
  window.borrow().area.set_text("");
  tell(app, window, |model| model.client.draft.text.clear());
  // A thread that the message starts shows in the list at once.
  rest(app, window);
}

/// An escape: it forgets what a first press of another key named, goes back from a thread that
/// opens or that failed to open, closes the list that stands over the home, or arms the stop of the
/// work of the open thread, which the next escape makes.
fn escaped(app: &App, window: &Shared, forgot: bool) {
  if forgot {
    return tell(app, window, |_| ());
  }
  if tell(app, window, Model::back) {
    return;
  }
  if window.borrow().screen.model.armed == Some(Armed::Stop) {
    return act(app, window, Does::Stop);
  }
  let closes = {
    let model = &window.borrow().screen.model;
    model.toggled && !model.listed()
  };
  if closes {
    tell(app, window, Model::close);
  } else {
    tell(app, window, Model::arm_stop);
  }
}

/// A control of the open thread that a key asks, as a stop, a resolve or a take back, asked of its
/// machine with what the window waits for. A machine that the window dials again takes nothing.
fn controlled(
  app: &App,
  window: &Shared,
  change: impl FnOnce(&mut Model) -> Option<((usize, Verb), Asked)>,
) {
  let Some(((at, verb), waits)) = tell(app, window, change) else { return colored(window) };
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  match ask(askers, &mut screen.model, at, verb) {
    Some(id) => drop(screen.model.client.asked.insert((at, id), waits)),
    None => screen.model.fault = screen.model.client.away(at),
  }
  drop(held);
  tell(app, window, |_| ());
}

/// The text of the input in the colors of python while it is a word, and plain otherwise.
fn colored(window: &Shared) {
  let held = window.borrow();
  let (area, model) = (&held.area, &held.screen.model);
  area.clear_extmarks();
  if !model.client.draft.python {
    return;
  }
  let plain = model.tokens.style(Role::Prose);
  let mut offset = 0;
  for (row, line) in model.colors.lines(&model.client.draft.text, plain).iter().enumerate() {
    offset += u32::from(row > 0);
    for one in line {
      let wide = u32::try_from(one.content.width()).unwrap_or_default();
      if one.style != plain {
        area.add_extmark(offset..offset + wide, ExtmarkOptions::default().style(one.style));
      }
      offset += wide;
    }
  }
}

/// The pointer: it is on the list or on the left side, it clicks a row, and its wheel moves the
/// rows under it. A click on a card opens its thread, and a click on a thing of the left side does
/// what its key does.
fn mouse(app: &App, window: &Shared, event: MouseEvent) {
  if window.borrow().screen.model.picker.is_some() {
    let does = tell(app, window, |model| {
      model.pointer = Some((event.x, event.y));
      let click = matches!(event.kind, MouseEventKind::Down(MouseButton::Left));
      click.then(|| model.clicked_over(event.x, event.y)).flatten()
    });
    if let Some(does) = does {
      act(app, window, does);
    }
    return;
  }
  let (on, does, verb) = tell(app, window, |model| {
    let (x, y) = (usize::from(event.x), usize::from(event.y));
    model.pointer = Some((event.x, event.y));
    let on = model.on_list(event.x);
    let (does, verb) = match (event.kind, on) {
      (MouseEventKind::Down(MouseButton::Left), true) => (model.clicked(y), None),
      (MouseEventKind::Down(MouseButton::Left), false) => (model.clicked_left(x, y), None),
      (MouseEventKind::ScrollUp, true) => {
        model.scrolled(-WHEEL);
        (None, None)
      }
      (MouseEventKind::ScrollDown, true) => {
        model.scrolled(WHEEL);
        (None, None)
      }
      (MouseEventKind::ScrollUp, false) => (None, model.scroll_feed(-WHEEL)),
      (MouseEventKind::ScrollDown, false) => (None, model.scroll_feed(WHEEL)),
      _ => (None, None),
    };
    (on, does, verb)
  });
  older(window, verb);
  if on {
    window.borrow_mut().touched = Some(app.now());
  }
  if let Some(does) = does
    && !answered(app, window, Some(does), false)
  {
    act(app, window, does);
  }
}
