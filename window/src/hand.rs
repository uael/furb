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

use crate::{
  Machine, Options, Screen,
  input::Asked,
  kept,
  keys::{self, Does},
  line::Line,
  link::{self, Asker, Heard},
  model::{Model, SIDE, Told},
  tokens::{Role, Tokens},
};

/// How long the hand stays away from the list before a card changes its place.
const AWAY: Duration = Duration::from_millis(1500);

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
  kept: PathBuf,
  /// When the window opened, on the clock of the app.
  born: Instant,
  /// When the hand was last on the list.
  touched: Option<Instant>,
  /// When the leader key was pressed, until the key that follows it.
  led: Option<Instant>,
  /// When the first escape was pressed, which a second one within the bound of the leader makes
  /// the stop of the work of the open thread.
  armed: Option<Instant>,
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
  model.now = (options.clock)();
  model.settle();
  let area = edited(app, &tokens);
  let window = Window {
    screen: drawn(app, model, &area),
    clock: options.clock,
    kept: options.kept,
    born: app.now(),
    touched: None,
    led: None,
    armed: None,
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
    tell(cx.app(), &on, |model| model.draft.text = text);
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
  let area = Textarea::new(app).absolute().wrap(WrapMode::Char);
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
  let written = |lines: &Vec<Line>| {
    let rows = lines.iter().enumerate().flat_map(|(n, line)| {
      let ended = (n > 0).then(|| Span::raw("\n"));
      ended.into_iter().chain(line.iter().cloned())
    });
    rows.collect::<Vec<_>>()
  };
  screen.part(
    |model: &Model| model.left().rows,
    move |lines| {
      home.set_text(written(lines));
    },
  );
  let area = area.clone();
  screen.part(Model::area, move |shown| {
    let area = area.position(Edge::Top, shown.y).position(Edge::Left, shown.x);
    area.width(shown.width).height(shown.height).placeholder(shown.placeholder);
  });
  // A loader is a cell of its own, so its step draws no other cell.
  let cells: Vec<Text> =
    (0..LOADERS).map(|_| Text::new(app, "").absolute().visible(false)).collect();
  for cell in &cells {
    app.root().child(cell);
  }
  let frames = |model: &Model| {
    let spots = model.left().loaders;
    spots.into_iter().map(|spot| (spot, spot.loader.frame(model.moved))).collect::<Vec<_>>()
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
  screen
}

/// A change of the model at this moment of the two clocks, which each part then shows. The loaders
/// move on one timer, which runs only while a mark moves.
fn tell<T>(app: &App, window: &Shared, change: impl FnOnce(&mut Model) -> T) -> T {
  let given = told(app, window, change);
  pump(window);
  given
}

/// A change of the model, which each part then shows, and the timer of the loaders.
fn told<T>(app: &App, window: &Shared, change: impl FnOnce(&mut Model) -> T) -> T {
  let mut held = window.borrow_mut();
  let (now, moved) = ((held.clock)(), app.now().saturating_duration_since(held.born));
  let given = held.screen.tell(|model| {
    (model.now, model.moved) = (now, moved);
    change(model)
  });
  match (held.screen.model.moves(), held.moves) {
    (true, None) => {
      let on = Rc::clone(window);
      held.moves = Some(app.set_interval(MOVES, move |cx| tick(cx.app(), &on)));
    }
    (false, Some(timer)) => {
      app.cancel_timer(timer);
      held.moves = None;
    }
    _ => {}
  }
  given
}

/// A moment of the clocks with no other change: the loaders and the ages move.
fn tick(app: &App, window: &Shared) {
  tell(app, window, |_| ());
}

/// What a machine told. A card shows its new state at once, and the list settles when the hand is
/// away.
fn hear(app: &App, window: &Shared, at: usize, heard: Heard) {
  let changed = tell(app, window, |model| model.hear(at, heard));
  let back = window.borrow_mut().screen.model.restore.take();
  if let Some(text) = back {
    window.borrow().area.set_text(text);
    colored(window);
  }
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
      kept::keep(&held.kept, &held.screen.model.machines)
    };
    let fault = written.err().map(|fault| format!("The cards are not kept: {fault}"));
    tell(cx.app(), &on, |model| model.fault = fault);
  });
}

/// Each request that is due asked of its machine: the page of the end of the open thread, and the
/// next request of the queue of the input, each with its id kept until its answer comes.
fn pump(window: &Shared) {
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  let model = &mut screen.model;
  if let Some(verb) = model.page_due()
    && let Some(opened) = model.thread.as_mut()
    && let Some(asker) = askers.get_mut(opened.key.0)
  {
    opened.asked.insert(asker.ask(verb), None);
  }
  if let Some((at, verb, asked)) = model.due()
    && let Some(asker) = askers.get_mut(at)
  {
    model.asked.insert((at, asker.ask(verb)), asked);
  }
}

/// A page of older exchanges that the open thread asks of its machine, said, with the id of its
/// request kept until its answer comes.
fn older(window: &Shared, verb: Option<Verb>) {
  let Some(verb @ Verb::Thread { before, .. }) = verb else { return };
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  let Some(opened) = screen.model.thread.as_mut() else { return };
  let Some(asker) = askers.get_mut(opened.key.0) else { return };
  opened.asked.insert(asker.ask(verb), before);
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
  match keys::pressed(key, led) {
    Some(Does::Leave) if !window.borrow().area.text().is_empty() => false,
    Some(does) => {
      act(app, window, does);
      true
    }
    None => led,
  }
}

/// A key while the picker of the sessions is open: it filters the list, moves in it, picks an
/// entry or closes it. Whether the picker took the key.
fn picking(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  let text = key.text().filter(|_| key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
  tell(app, window, |model| {
    let count = model.sessions().len();
    let Some(picker) = model.picker.as_mut() else { return false };
    match (key.code, text) {
      (KeyCode::Escape, _) => model.picker = None,
      (KeyCode::Enter, _) => {
        let at = picker.at;
        model.picked(at);
      }
      (KeyCode::Up, _) => picker.at = picker.at.saturating_sub(1),
      (KeyCode::Down, _) => picker.at = (picker.at + 1).min(count.saturating_sub(1)),
      (KeyCode::Backspace, _) => {
        picker.filter.pop();
        picker.at = 0;
      }
      (_, Some(one)) => {
        picker.filter.push(one);
        picker.at = 0;
      }
      _ => return false,
    }
    true
  })
}

/// What a key does, which a click on its text does too.
fn act(app: &App, window: &Shared, does: Does) {
  let change: fn(&mut Model) = match does {
    Does::Next => |model| model.step(1),
    Does::Previous => |model| model.step(-1),
    Does::Needed => Model::needed,
    Does::List => Model::toggle,
    Does::Jump => Model::jump,
    Does::NewThread => Model::turn,
    Does::Sessions => Model::pick_session,
    Does::Python => |model| model.draft.python = !model.draft.python,
    Does::Close => return escaped(app, window),
    Does::Stop => return stopped(app, window),
    Does::Send => return sent(app, window),
    Does::Empty => {
      window.borrow().area.set_text("");
      return tell(app, window, |model| model.draft.text.clear());
    }
    Does::Leave => return app.quit(),
    Does::PageUp | Does::PageDown => {
      // A page keeps two rows of the screen before it in view.
      let verb = tell(app, window, |model| {
        let rows = isize::try_from(model.feed_room()).unwrap_or_default() - 2;
        model.scroll_feed(if does == Does::PageUp { -rows } else { rows })
      });
      return older(window, verb);
    }
    // The keys that the window names and that no screen of it does yet.
    Does::GoTo | Does::Commands | Does::Keys => return,
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

/// The text of the input sent to its target, and the input empty.
fn sent(app: &App, window: &Shared) {
  let text = window.borrow().area.text();
  if text.trim().is_empty() {
    return;
  }
  window.borrow().area.set_text("");
  tell(app, window, |model| {
    model.draft.text.clear();
    model.send(text);
  });
}

/// An escape: it closes the list that stands over the home, or arms the stop of the work of the
/// open thread, which a second escape within the bound makes.
fn escaped(app: &App, window: &Shared) {
  let armed = window.borrow_mut().armed.take();
  if armed.is_some_and(|at| app.now().saturating_duration_since(at) <= LEADER) {
    return stopped(app, window);
  }
  let closes = {
    let model = &window.borrow().screen.model;
    model.toggled && !model.listed()
  };
  if closes {
    tell(app, window, Model::close);
  } else {
    window.borrow_mut().armed = Some(app.now());
  }
}

/// The work of the open thread stopped: the cancel of its prompt, asked of its machine.
fn stopped(app: &App, window: &Shared) {
  let stop = window.borrow().screen.model.stop();
  let Some((at, verb)) = stop else { return };
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  if let Some(asker) = askers.get_mut(at) {
    screen.model.asked.insert((at, asker.ask(verb)), Asked::Stop);
  }
  drop(held);
  tell(app, window, |_| ());
}

/// The text of the input in the colors of python while it is a word, and plain otherwise.
fn colored(window: &Shared) {
  let held = window.borrow();
  let (area, model) = (&held.area, &held.screen.model);
  area.clear_extmarks();
  if !model.draft.python {
    return;
  }
  let plain = model.tokens.style(Role::Prose);
  let mut offset = 0;
  for (row, line) in model.colors.lines(&model.draft.text, plain).iter().enumerate() {
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
  let (on, does, verb) = tell(app, window, |model| {
    let (x, y) = (usize::from(event.x), usize::from(event.y));
    model.pointer = Some((event.x, event.y));
    let on = model.on_list(event.x);
    let (does, verb) = match (event.kind, on) {
      (MouseEventKind::Down(MouseButton::Left), true) => {
        model.clicked(y);
        (None, None)
      }
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
  if let Some(does) = does {
    act(app, window, does);
  }
}
