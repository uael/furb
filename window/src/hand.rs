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
    Container, Edge, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind, Span,
    Text, TimerId, View, Widget, WrapMode,
  },
};

use crate::{
  Machine, Options, Screen, kept,
  line::Line,
  link::{self, Heard},
  model::{LIST, Model, SIDE, Told},
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
    link::dialed(app, machine, move |app, heard| hear(app, &on, at, heard));
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
  screen.part(Model::home, move |lines| {
    home.set_text(written(lines));
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

/// A key of the list: a chord with alt, or a letter after the leader.
fn key(app: &App, window: &Shared, key: &KeyEvent) {
  if key.kind == KeyEventKind::Release {
    return;
  }
  let led = window.borrow_mut().led.take();
  let led = led.is_some_and(|at| app.now().saturating_duration_since(at) <= LEADER);
  let alt = key.modifiers.contains(Modifiers::ALT);
  let change: fn(&mut Model) = match (key.code, alt, led) {
    (KeyCode::Char('x'), ..) if key.modifiers.contains(Modifiers::CTRL) => {
      window.borrow_mut().led = Some(app.now());
      return;
    }
    (KeyCode::Down, true, _) | (KeyCode::Char('j'), _, true) => |model| model.step(1),
    (KeyCode::Up, true, _) | (KeyCode::Char('k'), _, true) => |model| model.step(-1),
    (KeyCode::Char('a'), true, _) | (KeyCode::Char('a'), _, true) => Model::needed,
    (KeyCode::Char('l'), _, true) => Model::toggle,
    (KeyCode::Escape, ..) => Model::close,
    _ => return,
  };
  window.borrow_mut().touched = Some(app.now());
  tell(app, window, change);
}

/// The pointer: it is on the list or away from it, it clicks a row, and its wheel moves the rows.
fn mouse(app: &App, window: &Shared, event: MouseEvent) {
  let on = tell(app, window, |model| {
    let y = usize::from(event.y);
    let on = model.listed() && event.x >= model.size.0.saturating_sub(LIST);
    model.pointed = on.then_some(y);
    match event.kind {
      MouseEventKind::Down(MouseButton::Left) if on => model.clicked(y),
      MouseEventKind::ScrollUp if on => model.scrolled(-WHEEL),
      MouseEventKind::ScrollDown if on => model.scrolled(WHEEL),
      _ => {}
    }
    on
  });
  if on {
    window.borrow_mut().touched = Some(app.now());
  }
}
