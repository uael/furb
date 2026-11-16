//! The window on an app: its widgets, and what each key, each move of the pointer, each timer and
//! each frame of a machine changes in its model.

use std::{
  cell::{Cell, RefCell},
  fs, mem,
  ops::Range,
  path::PathBuf,
  rc::Rc,
  sync::Arc,
  time::{Duration, Instant},
};

use crate::cells::Cells;

use opentui::{
  App,
  input::KeyEventKind,
  native::ClipboardOutcome,
  prelude::{
    Container, Diff, DiffView, Edge, EditorAction, ExtmarkOptions, ImageView, KeyCode, KeyEvent,
    Modifiers, MouseButton, MouseEvent, MouseEventKind, ScrollBox, Span, Text, Textarea, TimerId,
    View, Widget, WrapMode,
  },
  style::{Color, Style},
};

use furb_host::protocol::Verb;

use furb::world::images;
use furb_client::{ASIDE, Anchor, Asked, CD, Folded, Pasted, Picture, Where, mentioned};

use crate::{
  Machine, Options, Screen, cells,
  code::syntax,
  input::Armed,
  kept,
  keys::{self, Does},
  line::Line,
  link::{self, Asker, Heard},
  machine::Looking,
  model::{Acts, Model, Told},
  pick::Pick,
  picture,
  question::Keyed,
  reload::{self, Watch},
  review::{MARKS, NEXT, PREVIOUS},
  thread::{Boxed, Spot},
  tokens::{AWAY, Role, Tokens},
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

/// How often the window looks at its program, which a build replaces.
const WATCH: Duration = Duration::from_secs(1);

/// How many loaders move at once at most: the line that runs and its light, the thought, the mark
/// and the word of each agent of the dock, and the row of hints.
const LOADERS: usize = 16;

/// How many images of the feed draw at once at most.
const PICTURES: usize = 4;

/// How many rows one step of the wheel moves the list, the feed or a diff.
const WHEEL: u8 = 3;

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
  /// The box of the diff of the review of the changes, which the keys of a page scroll.
  scroll: ScrollBox,
  /// The diff of the review, whose changes the keys of a change scroll to.
  diff: Diff,
  /// The watch of the program of the window, which a build replaces.
  program: Option<Watch>,
  /// Whether the window ended to give its place to a new build of its program.
  reloads: Rc<Cell<bool>>,
}

type Shared = Rc<RefCell<Window>>;

/// The window drawn on an app, with the cards that the window of before kept, and each machine
/// dialed, where the window before stood when it gave its place to this one. Whether the window
/// ended to give its place to a new build of its program, once the app ends.
pub(crate) fn opened(app: &App, machines: Vec<Machine>, options: Options) -> Rc<Cell<bool>> {
  cells::follow(app);
  let tokens = Tokens::on(app.palette().and_then(|palette| palette.background));
  app.set_theme(tokens.theme());
  let mut before: Vec<Told> = kept::kept(&options.kept);
  let told = |machine: &Machine| {
    let kept =
      before.iter().position(|one| one.name == machine.name).map(|at| before.swap_remove(at));
    kept.unwrap_or_else(|| Told { name: machine.name.clone(), ..Told::default() })
  };
  let told = machines.iter().map(told).collect();
  let mut model = Model::new(told, app.size(), tokens, &options);
  model.client.commands = Model::offered_commands();
  model.client.now = (options.clock)();
  model.settle();
  if options.reloaded {
    let file = reload::file_of(&options.kept);
    model.resume(kept::kept(&file));
    // A place serves once, so a window that a person starts later opens as a new one.
    fs::remove_file(file).ok();
  }
  let area = edited(app, &tokens);
  let (scroll, diff) = (ScrollBox::new(app), Diff::new(app));
  let window = Window {
    screen: drawn(app, model, &area, (&scroll, &diff)),
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
    scroll,
    diff,
    program: options.program.map(Watch::new),
    reloads: Rc::new(Cell::new(false)),
  };
  let reloads = Rc::clone(&window.reloads);
  let window = Rc::new(RefCell::new(window));
  listened(app, &window, &area);
  let (known, reads) = (machines.iter().map(|one| one.name.clone()).collect(), machines[0].reads);
  for machine in machines {
    dialed(app, &window, machine);
  }
  if let Some(tailnet) = options.tailnet {
    let on = Rc::clone(&window);
    let looking = Looking { tailnet, known, reads };
    link::looked(app, looking, Duration::ZERO, move |app, machine| {
      let told = Told { name: machine.name.clone(), ..Told::default() };
      tell(app, &on, |model| model.client.machines.push(told));
      dialed(app, &on, machine);
    });
  }
  reloads
}

/// The daemon of a machine dialed, whose frames the window hears under the place of the machine in
/// its list.
fn dialed(app: &App, window: &Shared, machine: Machine) {
  let (on, at) = (Rc::clone(window), window.borrow().askers.len());
  let asker = link::dialed(app, machine, move |app, heard| hear(app, &on, at, heard));
  window.borrow_mut().askers.push(asker);
}

/// The window hears the terminal: its keys, the edits of its input, the pointer, its focus and its
/// size, and its clocks and the watch of its program.
fn listened(app: &App, window: &Shared, area: &Textarea) {
  let on = Rc::clone(window);
  app.on_key_capture(move |cx, pressed| {
    if key(cx.app(), &on, pressed) {
      cx.stop_propagation();
    }
  });
  heard_edits(window, area);
  let on = Rc::clone(window);
  app.on_mouse(move |cx, event| mouse(cx.app(), &on, *event));
  let on = Rc::clone(window);
  app.on_terminal_focus(move |cx, focused| {
    if focused {
      tell(cx.app(), &on, |model| model.client.focused());
    }
  });
  let on = Rc::clone(window);
  app.on_resize(move |cx, width, height| {
    tell(cx.app(), &on, |model| model.resized((width, height)));
  });
  let on = Rc::clone(window);
  app.set_timeout(PATIENCE, move |cx| tell(cx.app(), &on, |model| model.slow = true));
  let on = Rc::clone(window);
  app.set_interval(AGES, move |cx| tick(cx.app(), &on));
  if window.borrow().program.is_some() {
    let on = Rc::clone(window);
    app.set_interval(WATCH, move |cx| watched(cx.app(), &on));
  }
}

/// A look at the program of the window: once a build replaced it and its size and time hold
/// still between two looks, the window writes where it stands and ends, and the program that ran
/// it starts the new one.
fn watched(app: &App, window: &Shared) {
  if !window.borrow_mut().program.as_mut().is_some_and(Watch::replaced) {
    return;
  }
  let (file, stood) = {
    let held = window.borrow();
    (reload::file_of(&held.kept), held.screen.model.stood())
  };
  if let Err(fault) = kept::keep(&file, &stood) {
    let fault = format!("The window cannot give its place to the new build: {fault}");
    return tell(app, window, |model| model.fault = Some(fault));
  }
  window.borrow().reloads.set(true);
  app.quit();
}

/// The editor of the input heard: a change of its text, which the draft takes with the place of
/// its caret, and a move of its caret to another word, which completes that word.
fn heard_edits(window: &Shared, area: &Textarea) {
  let (on, editor) = (Rc::clone(window), area.clone());
  area.on_change(move |cx, text| {
    let text = text.to_owned();
    let caret = usize::try_from(editor.cursor_offset()).unwrap_or_default();
    tell(cx.app(), &on, |model| {
      // A word that he writes leaves what a first press of a key named.
      if !text.is_empty() {
        model.armed = None;
      }
      model.client.edited(text, caret);
    });
    colored(&on);
  });
  // A paste of the path of an image file puts in that image, as a terminal pastes a file that he
  // drags on it, and a long text folds into its mark.
  let on = Rc::clone(window);
  area.on_paste(move |cx, text| {
    if filed(cx.app(), &on, text) || folded(cx.app(), &on, text) {
      cx.prevent_default();
    }
  });
  // A move of the caret to another word completes that word.
  let (on, editor) = (Rc::clone(window), area.clone());
  area.on_cursor_change(move |cx, _, _| {
    let caret = usize::try_from(editor.cursor_offset()).unwrap_or_default();
    tell(cx.app(), &on, |model| model.client.complete_at(caret));
  });
}

/// The editor of the text of the input, which has the focus. Enter sends, so a line breaks with
/// shift and enter, or with ctrl and j, which a terminal with no keys of Kitty sends for both, and
/// the end goes to the end of the feed, as the table of the keys says.
fn edited(app: &App, tokens: &Tokens) -> Textarea {
  let area = Textarea::new(app).absolute().wrap(WrapMode::Word);
  let area = area.colors(tokens.text, tokens.raised).focused_colors(tokens.text, tokens.raised);
  let area = area.placeholder_color(tokens.muted).cursor_color(tokens.text);
  let area = area.bind("shift+enter", EditorAction::Newline).bind("ctrl+j", EditorAction::Newline);
  area.unbind("end")
}

/// The widgets of the window on a model: the left side, the editor of the input over it, and the
/// list at its right. Each side is one text, which is written only when a row of it changes.
fn drawn(app: &App, model: Model, area: &Textarea, review: (&ScrollBox, &Diff)) -> Screen<Model> {
  let (scroll, diff) = review;
  let tokens = model.tokens;
  let column = || Text::new(app, "").wrap(WrapMode::None);
  let (home, list) = (column(), column());
  let stage = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Bottom, 0);
  // The list stands on a panel, so a change of ground parts it from the home, with no line.
  let side = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Right, 0);
  let side = side.position(Edge::Bottom, 0).bg(tokens.panel).padding_edge(Edge::Left, 1);
  let side = side.child(&list);
  app.root().child(&stage.child(&home).child(area)).child(&side);
  app.focus(area);

  let mut screen = Screen::new(model);
  staged(&mut screen, &stage);
  let mut left = Screen::new(sides(&screen.model));
  left.part(
    |sides: &Sides| sides.rows.clone(),
    move |lines| {
      home.set_text(written(lines));
    },
  );
  placed(app, &stage, &mut screen, area);
  loaded(app, &stage, &mut left);
  imaged(app, &stage, &mut left);
  screen.split(sides, left);
  screen.part(Model::list, move |lines| {
    list.set_text(written(lines));
  });
  screen.part(Model::listed, {
    let side = side.clone();
    move |listed| {
      side.visible(*listed);
    }
  });
  screen.part(Model::panel_width, move |width| {
    side.width(*width);
  });
  completion(app, &stage, &mut screen, &tokens);
  veiled(app, &mut screen, &tokens);
  reviewed(app, &mut screen, &tokens, (scroll, diff));
  screen
}

/// What the left side shows, read from the model in one build of its rows for each change: the
/// rows, the frame of each loader, and each image that the client holds.
struct Sides {
  rows: Vec<Line>,
  loaders: Vec<(Spot, Vec<(&'static str, Color)>)>,
  pictures: Vec<Drawn>,
}

fn sides(model: &Model) -> Sides {
  let left = model.left();
  let frame = |spot: Spot| {
    let color = spot.style.fg.unwrap_or(model.tokens.accent);
    (spot, spot.loader.cells(model.client.moved, color, &model.tokens))
  };
  let key = model.client.thread.as_ref().map(|one| &one.thread.key);
  let held = |one: Boxed| {
    let (at, life, _) = key?;
    match model.client.pictures.get(&(*at, life.clone(), one.source.clone())) {
      Some(Picture::Held(held)) => Some(Drawn(one, Arc::clone(&held.bytes))),
      _ => None,
    }
  };
  Sides {
    loaders: left.loaders.into_iter().map(frame).collect(),
    pictures: left.pictures.into_iter().filter_map(held).take(PICTURES).collect(),
    rows: left.rows,
  }
}

/// The loaders of the left side on its stage: a loader is a cell of its own, so its step draws no
/// other cell.
fn loaded(app: &App, stage: &View, screen: &mut Screen<Sides>) {
  let cells: Vec<Text> =
    (0..LOADERS).map(|_| Text::new(app, "").absolute().visible(false)).collect();
  for cell in &cells {
    stage.child(cell);
  }
  let frames = |sides: &Sides| sides.loaders.clone();
  screen.part(frames, move |shown| {
    for (at, cell) in cells.iter().enumerate() {
      match shown.get(at) {
        Some((spot, frame)) => {
          let cell = cell.position(Edge::Top, spot.y).position(Edge::Left, spot.x);
          let spans = frame.iter().map(|&(text, color)| Span::styled(text, spot.style.fg(color)));
          cell.set_text(spans.collect::<Vec<_>>());
          cell.visible(true);
        }
        None => {
          cell.visible(false);
        }
      }
    }
  });
}

/// An image of the left side in its box, with its bytes, which stay the same bytes while the
/// client holds them, so a change of the feed that keeps them decodes nothing.
#[derive(Clone)]
struct Drawn(Boxed, Arc<[u8]>);

impl PartialEq for Drawn {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 && Arc::ptr_eq(&self.1, &other.1)
  }
}

/// The images of the left side on its stage, each over the rows of its box. An image that does
/// not decode leaves its box blank, over the row that names it.
fn imaged(app: &App, stage: &View, screen: &mut Screen<Sides>) {
  let views = (0..PICTURES)
    .map(|_| (ImageView::new(app).absolute().visible(false), RefCell::new(None)))
    .collect::<Vec<(ImageView, RefCell<Option<Arc<[u8]>>>)>>();
  for (view, _) in &views {
    stage.child(view);
  }
  let shown = |sides: &Sides| sides.pictures.clone();
  screen.part(shown, move |shown| {
    for (at, (view, loaded)) in views.iter().enumerate() {
      let Some(Drawn(boxed, bytes)) = shown.get(at) else {
        view.visible(false);
        continue;
      };
      let fresh = !loaded.borrow().as_ref().is_some_and(|one| Arc::ptr_eq(one, bytes));
      let decoded = !fresh || view.load(bytes).is_ok();
      *loaded.borrow_mut() = decoded.then(|| Arc::clone(bytes));
      let view = view.position(Edge::Top, boxed.y).position(Edge::Left, boxed.x);
      view.size(boxed.wide, boxed.high).visible(decoded);
    }
  });
}

/// The stage of each widget of the left side at the left edge and at the width that the list
/// leaves it.
fn staged(screen: &mut Screen<Model>, stage: &View) {
  let stage = stage.position(Edge::Left, 0);
  screen.part(Model::wide, move |wide| {
    stage.width(u16::try_from(*wide).unwrap_or(u16::MAX));
  });
}

/// The review of the changes of a project over the whole window, while one stands, in the frame of
/// a thread: the head, the diff of the file that he reads in a box that scrolls, and the foot at
/// the left, and the changed files on the panel of the list at the right.
fn reviewed(app: &App, screen: &mut Screen<Model>, tokens: &Tokens, shown: (&ScrollBox, &Diff)) {
  let (scroll, diff) = shown;
  let text = || Text::new(app, "").wrap(WrapMode::None);
  let (head, side, foot) = (text(), text(), text());
  let diff = diff.line_numbers(true).colors(tokens.diff()).syntax_theme(syntax());
  // The marks of the sub threads stand in a column beside the diff, so they scroll with it.
  let marks = Text::new(app, "").wrap(WrapMode::None).width(u32::from(MARKS)).shrink(0.0);
  let rows = View::new(app).row().child(&marks).child(&diff.grow(1.0));
  let scroll = scroll.child(&rows);
  let page = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Left, 0);
  let page = page.position(Edge::Right, 0).position(Edge::Bottom, 0).bg(tokens.base);
  let panel = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Right, 0);
  let panel = panel.position(Edge::Bottom, 0).bg(tokens.panel).padding_edge(Edge::Left, 1);
  let panel = panel.child(&side);
  let head = head.absolute().position(Edge::Top, 0).position(Edge::Left, 0);
  let scroll = scroll.absolute().position(Edge::Top, 2).position(Edge::Left, 1);
  let scroll = scroll.position(Edge::Bottom, 2);
  let foot = foot.absolute().position(Edge::Bottom, 0).position(Edge::Left, 0);
  let page = page.child(&head).child(&scroll).child(&foot).child(&panel);
  app.root().child(&page.z_index(3).visible(false));
  let reviewing = |model: &Model| (model.reviewing(), model.panel_width());
  screen.part(reviewing, move |(shown, width)| {
    page.visible(shown.is_some());
    panel.width(*width);
    if let Some(shown) = shown {
      head.set_text(shown.head.clone());
      panel.visible(shown.side.is_some());
      side.set_text(written(shown.side.as_deref().unwrap_or_default()));
      scroll.position(Edge::Right, if shown.side.is_some() { width + 2 } else { 2 });
      foot.set_text(shown.foot.clone());
      let view = if shown.split { DiffView::Split } else { DiffView::Unified };
      diff.set_diff(shown.diff.clone()).filetype(shown.kind).view(view);
      marks.set_text(written(&shown.marks));
    }
  });
}

/// The panel of the completion of the input, over the feed on the top of the box of the input,
/// while one shows.
fn completion(app: &App, stage: &View, screen: &mut Screen<Model>, tokens: &Tokens) {
  let rows = Text::new(app, "").wrap(WrapMode::None);
  let part = View::new(app).absolute().bg(tokens.raised).child(&rows);
  stage.child(&part.z_index(1).visible(false));
  screen.part(Model::drawn_completion, move |shown| {
    part.visible(shown.is_some());
    if let Some((x, y, width, lines)) = shown {
      let height = u16::try_from(lines.len()).unwrap_or_default();
      part.position(Edge::Left, *x).position(Edge::Top, *y).width(*width).height(height);
      rows.set_text(written(lines));
    }
  });
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
fn placed(app: &App, stage: &View, screen: &mut Screen<Model>, area: &Textarea) {
  let tokens = screen.model.tokens;
  let ghost = Text::new(app, "").absolute().wrap(WrapMode::None).z_index(1).visible(false);
  stage.child(&ghost);
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
  // What the operator saw marks the card of his thread read at once, which the parts then show:
  // a card of a login that only reads, which records no Seen, would else show the unread mark that
  // the last cards of its machine gave it.
  if pump(window) {
    told(app, window, |_| ());
  }
  let back = window.borrow_mut().screen.model.client.restore.take();
  if let Some(text) = back {
    // The editor tells no change that the window makes, so the draft takes the text too.
    window.borrow_mut().screen.model.client.draft.text.clone_from(&text);
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
  // The terminal tells what it speaks some time after the window opens, so each change reads it.
  let speaks = app.session().capabilities().map(|one| one.flags).unwrap_or_default();
  cells::follow(app);
  let given = held.screen.tell(|model| {
    (model.client.now, model.client.moved, model.speaks) = (now, moved, speaks);
    let given = change(model);
    model.client.played();
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
  // A leave that chose what each session that works does waits until each choice went.
  let leaves = {
    let model = &window.borrow().screen.model;
    model.acts == Some(Acts::Leaving) && !model.client.controlling()
  };
  if leaves {
    return app.quit();
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
      kept::keep(&held.kept, &held.screen.model.client.machines)
    };
    // A write that works leaves a fault that the window shows, until he reads it.
    if let Err(fault) = written {
      tell(cx.app(), &on, |model| model.fault = Some(format!("The cards are not kept: {fault}")));
    }
  });
}

/// The draft emptied into the stash, or put aside or brought back by the key of the stash, and
/// the stash written to the file of the window at once, so a draft outlives a window that ends.
fn stashed(app: &App, window: &Shared, does: Does) {
  if does == Does::Empty {
    window.borrow().area.set_text("");
    tell(app, window, |model| model.client.stash_draft());
  } else if tell(app, window, |model| model.client.toggle_stash()) {
    window.borrow().area.set_text("");
    tell(app, window, |model| {
      let key = model.key(Does::Stash);
      model.client.notice = Some(format!("The input is put aside. {key} brings it back."));
    });
  }
  let written = {
    let held = window.borrow();
    kept::keep(&kept::stash_of(&held.kept), &held.screen.model.client.stash)
  };
  if let Err(fault) = written {
    tell(app, window, |model| {
      model.fault = Some(format!("The emptied drafts are not kept: {fault}"));
    });
  }
}

/// What the open thread said last, copied to the clipboard of the system or of the terminal, with
/// a notice that says whether it went.
fn copied(app: &App, window: &Shared) {
  let said = window
    .borrow()
    .screen
    .model
    .client
    .thread
    .as_ref()
    .and_then(|one| one.thread.last_said().map(str::to_owned));
  let notice = match said {
    None => "Nothing to copy yet.",
    Some(text) if app.copy_to_clipboard(&text) => "Copied what the thread said last.",
    Some(_) => "This terminal takes no copy.",
  };
  tell(app, window, |model| model.client.notice = Some(notice.to_owned()));
}

/// The image of the clipboard put in the input, else its text, as a paste puts it. A clipboard
/// with neither says so.
fn clipboard(app: &App, window: &Shared) {
  let on = Rc::clone(window);
  let kinds = ["image/png", "image/jpeg", "image/gif", "image/webp", "text/plain"];
  app.read_clipboard_as(&kinds, move |cx, outcome| match outcome {
    ClipboardOutcome::Read { mime_type, data } if mime_type.starts_with("image/") => {
      pictured(cx.app(), &on, &data);
    }
    ClipboardOutcome::Read { data, .. } => {
      let text = String::from_utf8_lossy(&data);
      if !filed(cx.app(), &on, &text) {
        let area = on.borrow().area.clone();
        area.insert(text);
      }
    }
    _ => tell(cx.app(), &on, |model| {
      model.client.notice = Some("The clipboard holds no image and no text.".to_owned());
    }),
  });
}

/// Whether a text that he pastes is the path of an image file, which then goes in as an image.
fn filed(app: &App, window: &Shared, text: &str) -> bool {
  let folder = PathBuf::from(&window.borrow().screen.model.client.folder);
  let Some(bytes) = picture::file(text, &folder) else { return false };
  pictured(app, window, &bytes);
  true
}

/// A long text that he pastes put in the input at its caret as its mark. The same text pasted
/// again while its mark stands shows it whole in the place of its mark.
fn folded(app: &App, window: &Shared, text: &str) -> bool {
  let area = window.borrow().area.clone();
  let draft = area.text();
  // A terminal ends each line of a paste with a return, as a key of the return.
  let text = text.replace("\r\n", "\n").replace('\r', "\n");
  let again = window
    .borrow()
    .screen
    .model
    .client
    .draft
    .folds
    .iter()
    .position(|one| one.text == text && draft.contains(&one.mark()));
  if let Some(at) = again {
    // The editor tells no change that the window makes, so the draft takes the text too.
    let whole = tell(app, window, |model| {
      let one = model.client.draft.folds.remove(at);
      model.client.draft.text = draft.replacen(&one.mark(), &one.text, 1);
      model.client.draft.text.clone()
    });
    area.set_text(whole);
  } else {
    let Some(mark) = tell(app, window, |model| model.client.fold(&text)) else { return false };
    area.insert(format!("{mark} "));
  }
  colored(window);
  true
}

/// An image put in the input at its caret as its mark, or why it does not go.
fn pictured(app: &App, window: &Shared, bytes: &[u8]) {
  let put = match images::kind(bytes) {
    Ok((_, suffix)) => tell(app, window, |model| model.client.paste_image(bytes, suffix)),
    Err(why) => Err(why.to_string()),
  };
  match put {
    Ok(mark) => {
      let area = window.borrow().area.clone();
      area.insert(format!("{mark} "));
    }
    Err(why) => tell(app, window, |model| model.client.notice = Some(why)),
  }
  colored(window);
}

/// Each request that is due asked of its machine: the open of each life that the window holds,
/// the models of each machine, the page of the end of the open thread, the next message of the
/// queue of the input, what the operator saw of the open thread, each image that it shows, and the
/// end of the word that the window ran, each with its id kept until its answer comes. Whether the
/// operator saw the open thread, which marks its card read.
fn pump(window: &Shared) -> bool {
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
  said(askers, model);
  let pictures = model.feed().map(|feed| feed.pictures()).unwrap_or_default();
  model.client.show_pictures(pictures);
  let listing = model.picker.as_ref().is_some_and(|one| one.of == Pick::GoTo);
  for (at, verb, asked) in model.client.asks_due(listing) {
    if let Some(id) = ask(askers, model, at, verb) {
      model.client.asked.insert((at, id), asked);
    }
  }
  if let Some((at, verb, asked)) = model.client.completion_due()
    && let Some(id) = ask(askers, model, at, verb)
  {
    model.client.asked.insert((at, id), asked);
  }
  if let Some((at, verb, target)) = model.client.history_due()
    && let Some(id) = ask(askers, model, at, verb)
  {
    model.client.asked.insert((at, id), Asked::History(target));
  }
  let seen = model.client.seen_due();
  let saw = seen.is_some();
  if let Some((at, verb)) = seen
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
    model.client.asked.insert((at, id), Asked::Ended(word.life.clone(), word.rung.clone()));
  }
  saw
}

/// The first message of the queue of the input, then the wake of its thread when a pause holds
/// that thread, on the same connection, so the daemon takes the message first.
fn said(askers: &mut [Asker], model: &mut Model) {
  let Some((at, verb)) = model.client.due() else { return };
  let Some(id) = ask(askers, model, at, verb) else { return };
  model.client.asked.insert((at, id), Asked::Say);
  if let Some(id) = model.client.wake_due().and_then(|wake| ask(askers, model, at, wake)) {
    model.client.asked.insert((at, id), Asked::Control);
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
  // A key after the leader is a key of the window, which no text that finds a chain takes.
  if completing(app, window, key) || looked(app, window, key) || !led && timed(app, window, key) {
    return true;
  }
  let does = keys::pressed(key, led);
  // A letter after the leader is a key of the window, and no letter of a text.
  let typed =
    !led && key.text().is_some() && (key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
  if answered(app, window, does, typed) || reached(app, window, does) {
    return true;
  }
  match does {
    Some(Does::Up | Does::Down) if walked(app, window, does == Some(Does::Up)) => true,
    // The grey rest, which the editor hears as a move when none shows.
    Some(Does::Take | Does::TakeWord) => took(window, does == Some(Does::TakeWord)),
    // The keys of a form, which the editor hears when no form takes them.
    Some(Does::Up | Does::Down | Does::Lean | Does::Tick | Does::NextQuestion) => false,
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
      (KeyCode::Escape, _) => model.picker = None,
      (KeyCode::Enter, _) => return Some(model.took(at)),
      (KeyCode::Up, _) => model.chose(-1),
      (KeyCode::Down, _) => model.chose(1),
      (KeyCode::Left | KeyCode::Right, _) => {
        if !model.leaned(if key.code == KeyCode::Left { -1 } else { 1 }) {
          return None;
        }
      }
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

/// A key while the completion of the input shows: the arrows move its chosen row, enter and tab
/// take it, and escape hides it. Whether the completion took the key.
fn completing(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  let shows = {
    let model = &window.borrow().screen.model;
    model.client.completion.as_ref().is_some_and(|one| !one.hidden) && model.picker.is_none()
  };
  if !shows || !key.modifiers.is_empty() {
    return false;
  }
  match key.code {
    KeyCode::Up => tell(app, window, |model| model.client.choose(-1)),
    KeyCode::Down => tell(app, window, |model| model.client.choose(1)),
    KeyCode::Escape => tell(app, window, |model| model.client.hide()),
    KeyCode::Enter | KeyCode::Tab => return completed(app, window, None),
    _ => return false,
  }
  true
}

/// A key while the review of the changes stands: the arrows read another file, the keys of a page
/// scroll its diff, escape and the key of the review close it, and it takes each other key, so no
/// letter goes to the input under it. Whether the review took the key.
fn looked(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  if window.borrow().screen.model.client.review.is_none() {
    return false;
  }
  match key.code {
    KeyCode::Up => tell(app, window, |model| model.client.review_by(-1)),
    KeyCode::Down => tell(app, window, |model| model.client.review_by(1)),
    KeyCode::Escape => tell(app, window, |model| model.client.review = None),
    KeyCode::PageUp | KeyCode::PageDown => {
      let rows = f64::from(window.borrow().screen.model.size.1.saturating_sub(6));
      let by = if key.code == KeyCode::PageUp { -rows } else { rows };
      window.borrow().scroll.scroll_by(0.0, by);
    }
    KeyCode::Char(sign) if [PREVIOUS, NEXT].contains(&sign.to_string().as_str()) => {
      let shown = window.borrow();
      let top = shown.scroll.scroll_position().1;
      let hunks = shown.diff.hunk_lines().into_iter().map(f64::from);
      let to = if sign.to_string() == NEXT {
        hunks.filter(|one| *one > top).reduce(f64::min)
      } else {
        hunks.filter(|one| *one < top).reduce(f64::max)
      };
      if let Some(to) = to {
        shown.scroll.scroll_to(0.0, to);
      }
    }
    _ if keys::pressed(key, false) == Some(Does::Changes) => {
      tell(app, window, |model| model.client.review = None);
    }
    _ => {}
  }
  true
}

/// A key while the bar of the time of the open thread shows: escape closes it back to now, and on
/// an empty input the arrows step one moment back or on, and space plays or stops. Whether the
/// bar took the key.
fn timed(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  let shows = {
    let model = &window.borrow().screen.model;
    model.client.thread.as_ref().is_some_and(|one| one.thread.time.is_some())
  };
  let finds = window
    .borrow()
    .screen
    .model
    .client
    .thread
    .as_ref()
    .is_some_and(|one| one.thread.time.as_ref().is_some_and(|time| time.chains.is_some()));
  if finds {
    return found(app, window, key);
  }
  let empty = window.borrow().area.text().is_empty();
  if !shows || !key.modifiers.is_empty() {
    return false;
  }
  match key.code {
    KeyCode::Escape => tell(app, window, |model| model.client.time()),
    KeyCode::Left if empty => tell(app, window, |model| model.client.step(-1)),
    KeyCode::Right if empty => tell(app, window, |model| model.client.step(1)),
    KeyCode::Char(' ') if empty => tell(app, window, |model| model.client.play()),
    _ => return false,
  }
  true
}

/// A key while the chains show under the bar of the time: a letter finds a chain by its label,
/// enter opens the first chain that shows at the moment that the thread shows, and escape hides
/// the chains. Whether the chains took the key.
fn found(app: &App, window: &Shared, key: &KeyEvent) -> bool {
  let text = key.text().filter(|_| key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
  match (key.code, text) {
    (KeyCode::Escape, _) => tell(app, window, |model| model.client.chains()),
    (KeyCode::Backspace, _) => tell(app, window, |model| {
      model.client.find(|find| {
        find.pop();
      });
    }),
    (KeyCode::Enter, _) => tell(app, window, |model| {
      let at = model.client.thread.as_ref().and_then(|one| one.thread.time.as_ref());
      let at = at.map(furb_client::Time::place).unwrap_or_default();
      model.open_chain(0, at);
    }),
    (_, Some(one)) => tell(app, window, |model| model.client.find(|find| find.push(one))),
    _ => return false,
  }
  true
}

/// The pointer while the review of the changes stands: a click on a file reads it, the name of the
/// file or the folder under it goes bright, a click on a folder or on a line of the diff opens the
/// sub thread about it, or turns the input to a message about it, and the wheel scrolls the diff.
fn looked_at(app: &App, window: &Shared, event: MouseEvent) {
  let file = window.borrow().screen.model.review_at(event.x, event.y);
  // The diff stands under the two rows of the head, from the row where its box scrolled.
  let scrolled = window.borrow().scroll.scroll_position().1;
  let row = event.y.checked_sub(2).map(|y| whole(f64::from(y) + scrolled));
  tell(app, window, |model| {
    model.pointer = Some((event.x, event.y));
    let down = matches!(event.kind, MouseEventKind::Down(MouseButton::Left));
    if let (Some(file), true) = (file, down)
      && let Some(review) = model.client.review.as_mut()
    {
      review.chosen = file;
    } else if down && let Some(folder) = model.folder_at(event.x, event.y) {
      // A folder opens the sub thread about it, or turns the input to a message about it.
      model.client.review = None;
      match model.sub_of(&folder).map(|one| one.card.prompt.clone()) {
        Some(sub) => model.open_agent(sub),
        None => model.client.draft.anchor = Some(Anchor { path: folder, at: Where::Folder }),
      }
    } else if down
      && !model.on_side(event.x)
      && let Some(sub) = row.and_then(|row| model.sub_on(row))
    {
      // A line that a sub thread is about opens that thread.
      model.client.review = None;
      model.open_agent(sub);
    } else if down
      && !model.on_side(event.x)
      && let Some(anchor) = row.and_then(|row| model.anchor_at(event.x, row))
    {
      (model.client.draft.anchor, model.client.review) = (Some(anchor), None);
    }
  });
  let rows = f64::from(WHEEL);
  let by = match event.kind {
    MouseEventKind::ScrollUp => -rows,
    MouseEventKind::ScrollDown => rows,
    _ => return,
  };
  window.borrow().scroll.scroll_by(0.0, by);
}

/// A place of a box that scrolls, in whole rows.
#[expect(
  clippy::cast_possible_truncation,
  clippy::cast_sign_loss,
  reason = "a box scrolls by whole rows, from 0"
)]
fn whole(rows: f64) -> usize {
  rows.max(0.0).round() as usize
}

/// A row of the completion taken, at a place or the chosen one: the editor takes the text with
/// its name and the caret after it, and a command runs. Whether a row was taken.
fn completed(app: &App, window: &Shared, place: Option<usize>) -> bool {
  let Some(taken) = tell(app, window, |model| model.client.take(place)) else { return false };
  {
    let area = window.borrow().area.clone();
    area.set_text(&taken.text);
    area.set_cursor_offset(u32::try_from(taken.caret).unwrap_or(u32::MAX));
  }
  tell(app, window, |model| model.client.complete_at(taken.caret));
  // The editor drops its marks with the text that it took, so the new text takes them again.
  colored(window);
  if let Some(does) = taken.command.as_deref().and_then(Model::command) {
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

/// A key while the keys reach a control of the views: the arrows move among the controls, Enter
/// uses the one that they reach, and Escape gives the keys back to the input. Whether it took
/// the key.
fn reached(app: &App, window: &Shared, does: Option<Does>) -> bool {
  if window.borrow().screen.model.client.aimed.is_none() {
    return false;
  }
  let change: fn(&mut Model) = match does {
    Some(Does::Up) => |model| model.client.step_control(-1),
    Some(Does::Down) => |model| model.client.step_control(1),
    Some(Does::Send) => |model| model.client.use_control(),
    Some(Does::Close) => |model| model.client.aimed = None,
    _ => return false,
  };
  tell(app, window, change);
  true
}

/// The feed a page up or down. A page keeps two rows of the screen before it in view.
fn paged(app: &App, window: &Shared, up: bool) {
  let verb = tell(app, window, |model| {
    let rows = isize::try_from(model.feed_room()).unwrap_or_default() - 2;
    model.scroll_feed(if up { -rows } else { rows })
  });
  older(window, verb);
}

/// What a key does, which a click on its text does too. Any other key leaves what a first press of
/// a key named, and what the key before did not do.
fn act(app: &App, window: &Shared, does: Does) {
  let forgot = window.borrow_mut().screen.model.pressed(does);
  asked(window, does);
  if let Some(pick) = Pick::of(does) {
    return changed(app, window, does, |model| model.pick(pick));
  }
  let change: fn(&mut Model) = match does {
    Does::Next => |model| model.step(1),
    Does::Previous => |model| model.step(-1),
    Does::Needed => Model::needed,
    Does::List => Model::toggle,
    Does::Jump => Model::jump,
    Does::NewThread => |model| model.client.turn(),
    Does::Effort => Model::effort,
    Does::Changes => |model| model.client.review(),
    Does::Views => Model::reach_views,
    Does::Context => Model::context_step,
    Does::Rewind => |model| model.client.time(),
    Does::Chains => |model| model.client.chains(),
    Does::Aside => |model| model.client.drafted(ASIDE.trim_end()),
    Does::Cd => |model| model.client.drafted(CD),
    Does::Rename => |model| model.client.rename_draft(),
    Does::Running => Model::act_on_open,
    Does::Python => |model| model.client.draft.python = !model.client.draft.python,
    Does::Agent => Model::next_agent,
    Does::Forward => Model::forward,
    Does::Close => return escaped(app, window, forgot),
    Does::Stop | Does::Resolve | Does::Revert | Does::Pause => {
      return controlled(app, window, |model| Some((model.control(does)?, Asked::Control)));
    }
    Does::Send => return sent(app, window),
    Does::Queue => return queued(app, window),
    Does::Empty if !window.borrow().area.text().is_empty() => return stashed(app, window, does),
    Does::Stash => return stashed(app, window, does),
    Does::Picture => return clipboard(app, window),
    Does::Copy => return copied(app, window),
    Does::Empty => {
      return controlled(app, window, |model| {
        model.take_back().map(|(asked, text)| (asked, Asked::Back(text)))
      });
    }
    Does::Leave => return left(app, window),
    Does::PageUp | Does::PageDown => return paged(app, window, does == Does::PageUp),
    // The keys of a form, of a list part and of the input, which the key that heard them did, and
    // the keys that open a list part, which the start of this function took.
    Does::Up
    | Does::Down
    | Does::Lean
    | Does::Tick
    | Does::NextQuestion
    | Does::Take
    | Does::TakeWord
    | Does::Sessions
    | Does::GoTo
    | Does::Machines
    | Does::Projects
    | Does::History
    | Does::Model
    | Does::Settings
    | Does::Skills
    | Does::Keys => return,
  };
  changed(app, window, does, change);
}

/// The change that a key makes to the window, then the thread that it opens.
fn changed(app: &App, window: &Shared, does: Does, change: impl FnOnce(&mut Model)) {
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

/// A leave: a first press arms it, and a second leaves, or asks first what each session that the
/// window holds and that works does.
fn left(app: &App, window: &Shared) {
  let leaves = tell(app, window, |model| {
    model.armed.replace(Armed::Leave) == Some(Armed::Leave) && model.leaves_now()
  });
  if leaves {
    app.quit();
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

/// The text of the input queued on the open thread, which its life sends once the thread rests.
fn queued(app: &App, window: &Shared) {
  let text = window.borrow().area.text();
  if text.trim().is_empty() || !tell(app, window, |model| model.queue(&text)) {
    return;
  }
  window.borrow().area.set_text("");
  tell(app, window, |model| model.client.draft.text.clear());
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

/// The text of the input in the colors of python while it is a word, and otherwise plain with
/// each mention of a file marked, so a file that the picker put in the text stands out.
fn colored(window: &Shared) {
  let held = window.borrow();
  let (area, model) = (&held.area, &held.screen.model);
  area.clear_extmarks();
  if !model.client.draft.python {
    let text = &model.client.draft.text;
    let t = &model.tokens;
    let mark = ExtmarkOptions::default().style(Style::new().fg(t.accent).bg(t.lifted));
    // The editor counts the cells of the text, and one for each end of a line.
    let cells = |bytes: &str| {
      bytes.split('\n').map(Cells::width).sum::<usize>() + bytes.matches('\n').count()
    };
    let place = |span: Range<usize>| {
      let start = u32::try_from(cells(&text[..span.start])).unwrap_or_default();
      start..start + u32::try_from(cells(&text[span])).unwrap_or_default()
    };
    for (span, _) in mentioned(text) {
      area.add_extmark(place(span), mark);
    }
    // An image and a folded text stand as one unit, which a key of the caret goes over and a
    // delete takes whole.
    let images = model.client.draft.images.iter().map(Pasted::mark);
    for shown in images.chain(model.client.draft.folds.iter().map(Folded::mark)) {
      for (at, _) in text.match_indices(&shown) {
        area.add_extmark(place(at..at + shown.len()), mark.atomic());
      }
    }
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
      model.pointed_over(event)
    });
    if let Some(does) = does {
      act(app, window, does);
    }
    return;
  }
  if window.borrow().screen.model.client.review.is_some() {
    looked_at(app, window, event);
    return;
  }
  let place = window.borrow().screen.model.completion_at(event.x, event.y);
  if let Some(place) = place {
    tell(app, window, |model| model.pointer = Some((event.x, event.y)));
    if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
      completed(app, window, Some(place));
    }
    return;
  }
  let (on, does, verb) = tell(app, window, |model| {
    let y = usize::from(event.y);
    model.pointer = Some((event.x, event.y));
    model.pointed_name();
    let on = model.on_list(event.x);
    let (does, verb) = match (event.kind, on) {
      (MouseEventKind::Down(MouseButton::Left), true) => (model.clicked(y), None),
      (MouseEventKind::Down(MouseButton::Left), false) => (model.clicked_left(event.x, y), None),
      (MouseEventKind::Drag(MouseButton::Left), false) => {
        model.dragged_left(event.x, y);
        (None, None)
      }
      (MouseEventKind::ScrollUp, true) => {
        model.scrolled(-isize::from(WHEEL));
        (None, None)
      }
      (MouseEventKind::ScrollDown, true) => {
        model.scrolled(isize::from(WHEEL));
        (None, None)
      }
      (MouseEventKind::ScrollUp, false) => (None, model.scroll_feed(-isize::from(WHEEL))),
      (MouseEventKind::ScrollDown, false) => (None, model.scroll_feed(isize::from(WHEEL))),
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
