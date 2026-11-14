//! The window on an app: its widgets, and what each key, each move of the pointer, each timer and
//! each frame of a machine changes in its model.

use std::{
  cell::RefCell,
  mem,
  path::PathBuf,
  rc::Rc,
  time::{Duration, Instant},
};

use opentui::{
  App,
  input::KeyEventKind,
  prelude::{
    Container, Edge, KeyEvent, MouseButton, MouseEvent, MouseEventKind, Span, Text, TimerId, View,
    Widget, WrapMode,
  },
};

use furb_host::protocol::Verb;

use crate::{
  Machine, Options, Screen, kept,
  keys::{self, Does},
  line::Line,
  link::{self, Asker, Heard},
  model::{Model, SIDE, Told},
  tokens::Tokens,
};

/// How long the hand stays away from the list before a card changes its place.
const AWAY: Duration = Duration::from_millis(1500);

/// How long the window waits for a machine before it says that it waits.
const PATIENCE: Duration = Duration::from_millis(400);

/// How long the leader key waits for the key that follows it.
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
  let window = Window {
    screen: drawn(app, model),
    clock: options.clock,
    kept: options.kept,
    born: app.now(),
    touched: None,
    led: None,
    settles: false,
    keeps: false,
    moves: None,
    askers: Vec::new(),
  };
  let window = Rc::new(RefCell::new(window));

  let on = Rc::clone(&window);
  app.on_key(move |cx, pressed| key(cx.app(), &on, pressed));
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

/// The widgets of the window on a model: the home, and the list at its right, over it. Each is one
/// text, which is written only when a row of it changes.
fn drawn(app: &App, model: Model) -> Screen<Model> {
  let tokens = model.tokens;
  let column = || Text::new(app, "").wrap(WrapMode::None);
  let (home, list) = (column(), column());
  // The list stands on a panel, so a change of ground parts it from the home, with no line.
  let side = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Right, 0);
  let side =
    side.position(Edge::Bottom, 0).width(SIDE).bg(tokens.panel).padding_edge(Edge::Left, 1);
  let side = side.child(&list);
  app.root().child(&home).child(&side);

  let mut screen = Screen::new(model);
  let written = |lines: &Vec<Line>| {
    let rows = lines.iter().enumerate().flat_map(|(n, line)| {
      let ended = (n > 0).then(|| Span::raw("\n"));
      ended.into_iter().chain(line.iter().cloned())
    });
    rows.collect::<Vec<_>>()
  };
  let left = |model: &Model| model.left().map_or_else(|| model.home(), |left| left.rows);
  screen.part(left, move |lines| {
    home.set_text(written(lines));
  });
  // A loader is a cell of its own, so its step draws no other cell.
  let cells: Vec<Text> =
    (0..LOADERS).map(|_| Text::new(app, "").absolute().visible(false)).collect();
  for cell in &cells {
    app.root().child(cell);
  }
  let frames = |model: &Model| {
    let spots = model.left().map(|left| left.loaders).unwrap_or_default();
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
  if tell(app, window, |model| model.hear(at, heard)) {
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

/// A page that the open thread asks of its machine, said, with the id of its request kept until
/// its answer comes.
fn ask(window: &Shared, verb: Option<Verb>) {
  let Some(verb @ Verb::Thread { before, .. }) = verb else { return };
  let mut held = window.borrow_mut();
  let Window { askers, screen, .. } = &mut *held;
  let Some(opened) = screen.model.thread.as_mut() else { return };
  let Some(asker) = askers.get_mut(opened.key.0) else { return };
  opened.asked.insert(asker.ask(verb), before);
}

/// A key of the window, from the one table of the keys.
fn key(app: &App, window: &Shared, key: &KeyEvent) {
  if key.kind == KeyEventKind::Release {
    return;
  }
  let led = window.borrow_mut().led.take();
  let led = led.is_some_and(|at| app.now().saturating_duration_since(at) <= LEADER);
  if keys::leads(key) {
    window.borrow_mut().led = Some(app.now());
    return;
  }
  let change: fn(&mut Model) = match keys::pressed(key, led) {
    Some(Does::Next) => |model| model.step(1),
    Some(Does::Previous) => |model| model.step(-1),
    Some(Does::Needed) => Model::needed,
    Some(Does::List) => Model::toggle,
    Some(Does::Close) => Model::close,
    Some(Does::Jump) => Model::jump,
    Some(Does::Open) => {
      let verb = tell(app, window, Model::open_thread);
      return ask(window, verb);
    }
    Some(page @ (Does::PageUp | Does::PageDown)) => {
      // A page keeps two rows of the screen before it in view.
      let verb = tell(app, window, |model| {
        let rows = isize::try_from(model.feed_room()).unwrap_or_default() - 2;
        model.scroll_feed(if page == Does::PageUp { -rows } else { rows })
      });
      return ask(window, verb);
    }
    // The keys that the window names and that no screen of it does yet.
    Some(Does::Stop | Does::GoTo | Does::Commands | Does::Keys | Does::NewThread) | None => return,
  };
  window.borrow_mut().touched = Some(app.now());
  tell(app, window, change);
}

/// The pointer: it is on the list or on the thread, it clicks a row, and its wheel moves the rows
/// under it. A click on a card opens its thread.
fn mouse(app: &App, window: &Shared, event: MouseEvent) {
  let (on, verb) = tell(app, window, |model| {
    let (x, y) = (usize::from(event.x), usize::from(event.y));
    model.pointer = Some((event.x, event.y));
    let on = model.on_list(event.x);
    let verb = match (event.kind, on) {
      (MouseEventKind::Down(MouseButton::Left), true) => model.clicked(y),
      (MouseEventKind::Down(MouseButton::Left), false) => {
        model.clicked_feed(x, y);
        None
      }
      (MouseEventKind::ScrollUp, true) => {
        model.scrolled(-WHEEL);
        None
      }
      (MouseEventKind::ScrollDown, true) => {
        model.scrolled(WHEEL);
        None
      }
      (MouseEventKind::ScrollUp, false) => model.scroll_feed(-WHEEL),
      (MouseEventKind::ScrollDown, false) => model.scroll_feed(WHEEL),
      _ => None,
    };
    (on, verb)
  });
  ask(window, verb);
  if on {
    window.borrow_mut().touched = Some(app.now());
  }
}
