//! The window on an app: its widgets, and what each key, each move of the pointer, each timer and
//! each frame of a machine changes in its model.

use std::{
  env, fs, io, mem,
  ops::Range,
  path::{Path, PathBuf},
  process,
  sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
  },
  time::{Duration, Instant},
};

use crate::cells::Cells;

use opentui::{
  App,
  input::{Event, KeyEventKind, Keymap},
  native::ClipboardOutcome,
  prelude::{
    AppProxy, Container, Diff, DiffView, Edge, EditorAction, ExtmarkOptions, ImageView, KeyCode,
    KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind, ScrollBox, Span, Text, Textarea,
    TimerId, View, Widget, WrapMode,
  },
  style::{Color, Style},
};

use furb_host::protocol::{BOUND, Verb};
use futures::StreamExt;
use tokio::process::ChildStdout;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};

use furb::world::{
  config::{KEY_VOICE, Layers},
  images,
};
use furb_client::{ASIDE, Anchor, Asked, CD, Folded, Pasted, Picture, Where, mentioned};

use crate::{
  Machine, Options, Screen, cells,
  code::theme,
  input::{Armed, LEVELS},
  kept,
  keys::{self, Does},
  line::Line,
  link::{self, Asker, Heard},
  machine::Looking,
  model::{Acts, Model, Told},
  pick::Pick,
  picture,
  question::Keyed,
  reload::{self, Seen, Watch},
  review::{MARKS, NEXT, PREVIOUS},
  thread::{Boxed, Click, Rows, Spot},
  tokens::{AWAY, Role, Tokens},
  voice::{self, RELEASED, Record, Run, STOPPED, Said, Space, Stage, Voice},
};

/// How long the window waits for a machine before it says that it waits.
const PATIENCE: Duration = Duration::from_millis(400);

/// What the window says when the terminal took no copy.
const TAKES_NO_COPY: &str = "This terminal takes no copy.";

/// What the window says once a text that he selected went to the clipboard.
const COPIED_SELECTION: &str = "Copied the selection.";

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

/// How each notice of the program of the speech starts, so its next notice takes its place.
const VOICE: &str = "The voice ";

/// How long the notice that the speech is ready stays.
const READY: Duration = Duration::from_secs(5);

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
  /// The keys of the editor, which the window gives it for each key that it does not take.
  keymap: Keymap<EditorAction>,
  /// The caret of the last edit, until the event of that edit ends: the editor tells the move of
  /// the caret of an edit after the edit, which completes no word there.
  edited_at: Option<usize>,
  /// The timer that settles the list, while one waits.
  settles: Option<TimerId>,
  /// The timer that writes the cards, while one waits.
  keeps: Option<TimerId>,
  /// The timer of the loaders, while a mark moves.
  moves: Option<TimerId>,
  /// The rows that the steps of the wheel over the feed moved since the last change, and where the
  /// pointer stood at the last of them, which a timer tells once the input of the terminal ends.
  wheel: Option<(isize, (u16, u16))>,
  /// What asks each machine a verb, in the order of the machines.
  askers: Vec<Asker>,
  /// The box of the diff of the review of the changes, which the keys of a page scroll.
  scroll: ScrollBox,
  /// The diff of the review, whose changes the keys of a change scroll to.
  diff: Diff,
  /// The box of the transcript of the chain of the open thread, which the keys of a page scroll.
  pages: ScrollBox,
  /// The watch of the program of the window, which a build replaces.
  program: Option<Watch>,
  /// Where the window says that it ended to give its place to a new build of its program.
  reloads: Sender<()>,
  /// The voice of the operator, while he holds the space bar.
  voice: Voice,
  /// The program of the speech that the window was given, else the one that the build carries.
  listen: Option<PathBuf>,
  /// Whether the last press of the pointer fell on the bar of the time, whose drag moves in time
  /// and selects no text.
  on_bar: bool,
  /// What each callback of the app holds of the window.
  hand: Hand,
  /// What the callbacks sent, which the window hears in the order sent.
  inputs: Receiver<Input>,
}

/// What reaches the window. A callback of the app only sends one and returns, and the window hears
/// each in the order sent once the event that sent it ends: so no hearing runs inside another, as
/// when a change of the text of the editor tells the editor again.
enum Input {
  /// A key of the terminal, which the window takes or gives to its editor.
  Key(KeyEvent),
  Mouse(MouseEvent),
  /// A text that he selected with the pointer, which the window copies.
  Selected(String),
  /// The terminal took the focus again.
  Focused,
  Resized(u16, u16),
  /// The new text of the editor.
  Edited(String),
  /// A move of the caret of the editor.
  Moved,
  /// A text that he pasted in the editor.
  Pasted(String),
  /// A frame of the machine at a place of the list.
  Framed(usize, Heard),
  /// A machine that the tailnet found.
  Found(Machine),
  /// The machines kept the window waiting for a moment.
  Slow,
  /// A moment of the clocks: a step of the loaders, or the ages of the cards.
  Tick,
  /// A look at the program of the window, which a build replaces.
  Watch,
  /// The hand was away from the list for long enough.
  Settle,
  /// The cards changed a moment ago.
  Keep,
  /// The steps of the wheel that came together ended.
  Turned,
  /// What the clipboard held, which goes in the input.
  Clipboard(ClipboardOutcome),
  /// The text that the editor of the operator saved, or why it gave none.
  Wrote(io::Result<String>),
  /// A line of the program of the speech, and the rest of its output.
  Spoke(Option<Result<String, LinesCodecError>>, FramedRead<ChildStdout, LinesCodec>),
  /// The notice that the speech is ready leaves, unless another took its place.
  Unready(String),
  /// A look at the record of his voice after a pause of the repeats of the space bar.
  Hold,
  /// The time for the final text of the record that stopped at a moment ended.
  Lost(Instant),
}

/// What each callback of the app holds of the window: the way of its inputs, and the queue of the
/// app that wakes the window to hear them.
#[derive(Clone)]
struct Hand {
  inputs: Sender<Input>,
  proxy: AppProxy,
}

/// The call that wakes the window to hear its inputs.
struct Due;

impl Hand {
  /// An input sent to the window, which hears it once the event that runs the callback ends,
  /// before the next event of the terminal. A window that ended hears nothing, and a queue of the
  /// app that is full already holds a call that hears this input.
  fn send(&self, app: &App, input: Input) {
    if self.inputs.send(input).is_ok() && self.proxy.send(Due).is_ok() {
      app.defer(|cx| cx.app().dispatch(Event::Wake));
    }
  }
}

/// The window drawn on an app, with the cards that the window of before kept, and each machine
/// dialed, where the window before stood when it gave its place to this one. What says, once the
/// app ends, that the window ended to give its place to a new build of its program.
pub(crate) fn opened(app: &App, machines: Vec<Machine>, mut options: Options) -> Receiver<()> {
  let tailnet = options.tailnet.take();
  let (reloads, reloaded) = mpsc::channel();
  let mut window = windowed(app, &machines, options, reloads);
  let known = machines.iter().map(|one| one.name.clone()).collect();
  let reads = machines.first().is_some_and(|one| one.reads);
  for machine in machines {
    window.dialed(app, machine);
  }
  if let Some(tailnet) = tailnet {
    let hand = window.hand.clone();
    let looking = Looking { tailnet, known, reads };
    link::looked(app, looking, Duration::ZERO, move |app, machine| {
      hand.send(app, Input::Found(machine));
    });
  }
  // The app owns the window from now on, and gives it each input that its callbacks sent.
  app.on_message(move |cx, _: &Due| window.drain(cx.app()));
  reloaded
}

/// The model of the window on the theme of the terminal, with the cards that the window of before
/// kept for each machine.
fn modeled(app: &App, machines: &[Machine], options: &Options) -> Model {
  let tokens = Tokens::on(app.palette().and_then(|palette| palette.background));
  app.set_theme(tokens.theme());
  let mut before: Vec<Told> = kept::kept(&options.kept);
  let told = |machine: &Machine| {
    let kept =
      before.iter().position(|one| one.name == machine.name).map(|at| before.swap_remove(at));
    kept.unwrap_or_else(|| Told { name: machine.name.clone(), ..Told::default() })
  };
  let told = machines.iter().map(told).collect();
  let mut model = Model::new(told, app.size(), tokens, options);
  model.client.commands = Model::offered_commands();
  model.client.now = (options.clock)();
  model.settle();
  model
}

/// The window drawn on an app, with the cards that the window of before kept for each machine, and
/// where it stood when it gave its place to this one, before it dials a machine.
fn windowed(app: &App, machines: &[Machine], options: Options, reloads: Sender<()>) -> Window {
  cells::follow(app);
  let mut model = modeled(app, machines, &options);
  let tokens = model.tokens;
  let place = options.reloaded.then(|| reload::file_of(&options.kept));
  if let Some(file) = &place {
    model.resume(kept::kept(file));
    let built = options.program.as_deref().and_then(reload::built_at);
    model.client.notice = built.map(|at| format!("This window runs the build of {at}."));
  }
  let widgets = Widgets {
    area: edited(app, &tokens),
    scroll: ScrollBox::new(app),
    diff: Diff::new(app),
    pages: ScrollBox::new(app),
  };
  let screen = drawn(app, model, &widgets);
  let Widgets { area, scroll, diff, pages } = widgets;
  let (inputs, heard) = mpsc::channel();
  let mut window = Window {
    screen,
    clock: options.clock,
    still: options.still,
    kept: options.kept,
    born: app.now(),
    touched: None,
    led: None,
    keymap: area.keymap(),
    area,
    edited_at: None,
    settles: None,
    keeps: None,
    moves: None,
    wheel: None,
    askers: Vec::new(),
    scroll,
    diff,
    pages,
    program: options.program.map(Watch::new),
    reloads,
    voice: Voice::default(),
    listen: options.listen,
    on_bar: false,
    hand: Hand { inputs, proxy: app.proxy() },
    inputs: heard,
  };
  window.restored();
  // A place serves once, so a window that a person starts later opens as a new one. It goes only
  // once its draft stands in the input again.
  if let Some(file) = place {
    fs::remove_file(file).ok();
  }
  window.listened(app);
  // A window with no speech says why at the first hold, which tries to start it again.
  window.readied(app).ok();
  window
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

/// The widgets that the window keeps beside its parts: the editor of the input, the box and the
/// diff of the review of the changes, and the box of the transcript.
struct Widgets {
  area: Textarea,
  scroll: ScrollBox,
  diff: Diff,
  pages: ScrollBox,
}

/// The widgets of the window on a model: the left side, the editor of the input over it, and the
/// list at its right. Each side is one text, which is written only when a row of it changes.
fn drawn(app: &App, model: Model, widgets: &Widgets) -> Screen<Model> {
  let Widgets { area, scroll, diff, pages } = widgets;
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
  let mut left = Screen::new(sides(&mut screen.model));
  left.held(
    |sides: &Sides| -> &Rows { &sides.rows },
    move |rows| {
      home.set_text(written(rows.iter().map(|row| &row.line)));
    },
  );
  placed(app, &stage, &mut screen, area);
  loaded(app, &stage, &mut left);
  imaged(app, &stage, &mut left);
  screen.split(sides, left);
  screen.part(Model::list, move |lines| {
    list.set_text(written(lines.iter()));
  });
  screen.part(|model| model.listed(), {
    let side = side.clone();
    move |listed| {
      side.visible(*listed);
    }
  });
  screen.part(
    |model| model.panel_width(),
    move |width| {
      side.width(*width);
    },
  );
  completion(app, &stage, &mut screen, &tokens);
  veiled(app, &mut screen, &tokens);
  reviewed(app, &mut screen, &tokens, (scroll, diff));
  transcribed(app, &mut screen, &tokens, pages);
  screen
}

/// The transcript of the chain of the open thread over the whole window, while one stands, in the
/// frame of the review: its head, its rows in a box that scrolls, and its foot.
fn transcribed(app: &App, screen: &mut Screen<Model>, tokens: &Tokens, pages: &ScrollBox) {
  let text = || Text::new(app, "").wrap(WrapMode::None);
  let (head, body, foot) = (text(), text(), text());
  let page = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Left, 0);
  let page = page.position(Edge::Right, 0).position(Edge::Bottom, 0).bg(tokens.base);
  let head = head.absolute().position(Edge::Top, 0).position(Edge::Left, 0);
  let pages = pages.child(&body).absolute().position(Edge::Top, 2).position(Edge::Left, 0);
  let pages = pages.position(Edge::Right, 0).position(Edge::Bottom, 2);
  let foot = foot.absolute().position(Edge::Bottom, 0).position(Edge::Left, 0);
  app.root().child(&page.child(&head).child(&pages).child(&foot).z_index(4).visible(false));
  screen.part(Model::transcribing, move |shown| {
    page.visible(shown.is_some());
    if let Some(shown) = shown {
      head.set_text(shown.head.clone());
      body.set_text(written(shown.body.iter()));
      foot.set_text(shown.foot.clone());
    }
  });
}

/// What the left side shows, read from the model in one build of its rows for each change: the
/// rows, the frame of each loader, and each image that the client holds.
struct Sides {
  rows: Rows,
  loaders: Vec<(Spot, Vec<(&'static str, Color)>)>,
  pictures: Vec<Drawn>,
}

fn sides(model: &mut Model) -> Sides {
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
  screen.held(
    |sides: &Sides| &sides.loaders,
    move |shown| {
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
    },
  );
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
  let mut views = (0..PICTURES)
    .map(|_| (ImageView::new(app).absolute().visible(false), None))
    .collect::<Vec<(ImageView, Option<Arc<[u8]>>)>>();
  for (view, _) in &views {
    stage.child(view);
  }
  screen.held(
    |sides: &Sides| &sides.pictures,
    move |shown| {
      for (at, (view, loaded)) in views.iter_mut().enumerate() {
        let Some(Drawn(boxed, bytes)) = shown.get(at) else {
          view.visible(false);
          continue;
        };
        let fresh = !loaded.as_ref().is_some_and(|one| Arc::ptr_eq(one, bytes));
        let decoded = !fresh || view.load(bytes).is_ok();
        *loaded = decoded.then(|| Arc::clone(bytes));
        let view = view.position(Edge::Top, boxed.y).position(Edge::Left, boxed.x);
        view.size(boxed.wide, boxed.high).visible(decoded);
      }
    },
  );
}

/// The stage of each widget of the left side at the left edge and at the width that the list
/// leaves it.
fn staged(screen: &mut Screen<Model>, stage: &View) {
  let stage = stage.position(Edge::Left, 0);
  screen.part(
    |model| model.wide(),
    move |wide| {
      stage.width(u16::try_from(*wide).unwrap_or(u16::MAX));
    },
  );
}

/// The review of the changes of a project over the whole window, while one stands, in the frame of
/// a thread: the head, the diff of the file that he reads in a box that scrolls, and the foot at
/// the left, and the changed files on the panel of the list at the right.
fn reviewed(app: &App, screen: &mut Screen<Model>, tokens: &Tokens, shown: (&ScrollBox, &Diff)) {
  let (scroll, diff) = shown;
  let text = || Text::new(app, "").wrap(WrapMode::None);
  let (head, side, foot) = (text(), text(), text());
  let diff = diff.line_numbers(true).colors(tokens.diff()).syntax_theme(theme().clone());
  // The marks of the sub threads stand in a column beside the diff, so they scroll with it.
  let marks = Text::new(app, "").wrap(WrapMode::None).width(u32::from(MARKS)).shrink(0.0);
  let rows = View::new(app).row().child(&marks).child(&diff.grow(1.0));
  // The comment that he writes on a line stands under that line, over the lines after it.
  let note = Text::new(app, "").absolute().wrap(WrapMode::None).z_index(1).visible(false);
  let note = note.position(Edge::Left, MARKS).position(Edge::Right, 0);
  let scroll = scroll.child(&View::new(app).child(&rows).child(&note));
  screen.part(
    |model| model.commenting(),
    move |shown| {
      note.visible(shown.is_some());
      if let Some((row, lines)) = shown {
        note.position(Edge::Top, *row);
        note.set_text(written(lines.iter()));
      }
    },
  );
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
  let reviewing = |model: &mut Model| (model.reviewing(), model.panel_width());
  screen.part(reviewing, move |(shown, width)| {
    page.visible(shown.is_some());
    panel.width(*width);
    if let Some(shown) = shown {
      head.set_text(shown.head.clone());
      panel.visible(shown.side.is_some());
      side.set_text(written(shown.side.iter().flatten()));
      scroll.position(Edge::Right, if shown.side.is_some() { width + 2 } else { 2 });
      foot.set_text(shown.foot.clone());
      let view = if shown.split { DiffView::Split } else { DiffView::Unified };
      diff.set_diff(shown.diff.clone()).filetype(shown.kind).view(view);
      marks.set_text(written(shown.marks.iter()));
    }
  });
}

/// The panel of the completion of the input, over the feed on the top of the box of the input,
/// while one shows.
fn completion(app: &App, stage: &View, screen: &mut Screen<Model>, tokens: &Tokens) {
  let rows = Text::new(app, "").wrap(WrapMode::None);
  let part = View::new(app).absolute().bg(tokens.raised).child(&rows);
  stage.child(&part.z_index(1).visible(false));
  screen.part(
    |model| model.drawn_completion(),
    move |shown| {
      part.visible(shown.is_some());
      if let Some((x, y, width, lines)) = shown {
        let height = u16::try_from(lines.len()).unwrap_or_default();
        part.position(Edge::Left, *x).position(Edge::Top, *y).width(*width).height(height);
        rows.set_text(written(lines.iter()));
      }
    },
  );
}

/// The list part over each other widget, on a veil that dims the whole window, while one is open.
fn veiled(app: &App, screen: &mut Screen<Model>, tokens: &Tokens) {
  let veil = View::new(app).absolute().position(Edge::Top, 0).position(Edge::Left, 0);
  let veil = veil.position(Edge::Right, 0).position(Edge::Bottom, 0).bg(tokens.veil);
  let rows = Text::new(app, "").wrap(WrapMode::None);
  let part = View::new(app).absolute().bg(tokens.raised).child(&rows);
  app.root().child(&veil.z_index(1).visible(false)).child(&part.z_index(2).visible(false));
  screen.part(
    |model| model.drawn_over(),
    move |over| {
      veil.visible(over.is_some());
      part.visible(over.is_some());
      if let Some((x, y, width, lines)) = over {
        let height = u16::try_from(lines.len()).unwrap_or_default();
        part.position(Edge::Left, *x).position(Edge::Top, *y).width(*width).height(height);
        rows.set_text(written(lines.iter()));
      }
    },
  );
}

/// The lines of a text widget, one row each.
fn written<'a>(lines: impl IntoIterator<Item = &'a Line>) -> Vec<Span<'static>> {
  let rows = lines.into_iter().enumerate().flat_map(|(n, line)| {
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
  screen.part(
    |model| model.ghost(),
    move |shown| match shown {
      Some((x, y, rest)) => {
        let ghost = ghost.position(Edge::Left, *x).position(Edge::Top, *y);
        ghost
          .set_text(vec![Span::styled(rest.clone(), tokens.style(Role::Muted).bg(tokens.raised))]);
        ghost.visible(true);
      }
      None => {
        ghost.visible(false);
      }
    },
  );
  let (area, focus) = (area.clone(), app.clone());
  screen.part(
    |model| model.area(),
    move |shown| {
      let area = area.position(Edge::Top, shown.y).position(Edge::Left, shown.x);
      let area = area.width(shown.width).height(shown.height).placeholder(&shown.placeholder);
      if area.visible(shown.shown).is_visible() {
        focus.focus(&area);
      }
    },
  );
}

/// Each request that is due asked of its machine: the open of each life that the window holds,
/// the models of each machine, the page of the end of the open thread, the next message of the
/// queue of the input, what the operator saw of the open thread, each image that it shows, and the
/// end of the word that the window ran, each with its id kept until its answer comes. Whether the
/// operator saw the open thread, which marks its card read.
fn pump(askers: &mut [Asker], model: &mut Model) -> bool {
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
  // The images and the whole outputs of the open thread read each of its exchanges, so they are
  // read again only when what the feed shows changed, and not at each scroll.
  if model.pictured != model.shows {
    model.pictured = model.shows;
    shown(model);
  }
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

/// The images and the whole outputs of commands that the open thread shows, which the client asks
/// of their machine.
fn shown(model: &mut Model) {
  let shown = model.laying(|model, kept| {
    model.feed(kept).map(|feed| (feed.pictures(), feed.wholes())).unwrap_or_default()
  });
  let (pictures, wholes) = shown;
  model.client.show_pictures(pictures);
  model.client.show_wholes(&wholes);
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

/// A place of a box that scrolls, in whole rows.
#[expect(
  clippy::cast_possible_truncation,
  clippy::cast_sign_loss,
  reason = "a box scrolls by whole rows, from 0"
)]
fn whole(rows: f64) -> usize {
  rows.max(0.0).round() as usize
}

/// The change that a key makes to the model alone, after which the thread that it names opens.
fn change_of(does: Does) -> Option<fn(&mut Model)> {
  Some(match does {
    Does::Next => |model| model.step(1),
    Does::Previous => |model| model.step(-1),
    Does::Needed => Model::needed,
    Does::List => Model::toggle,
    Does::Jump => Model::jump,
    Does::NewThread => |model| model.client.turn(),
    Does::Changes => |model| model.client.review(),
    Does::Transcript => |model| model.client.transcript(None),
    Does::Views => Model::reach_views,
    Does::Context => Model::context_step,
    Does::Rewind => |model| model.client.time(),
    Does::Chains => |model| model.client.chains(),
    Does::Aside => |model| model.client.drafted(ASIDE.trim_end()),
    Does::Cd => |model| model.client.drafted(CD),
    Does::Rename => |model| model.client.rename_draft(),
    Does::Login => Model::login,
    Does::Status => Model::status,
    Does::Running | Does::Delete => Model::act_on_open,
    Does::Python => |model| model.client.draft.python = !model.client.draft.python,
    Does::Agent => Model::next_agent,
    Does::Back => |model| model.walk(true),
    Does::Forward => |model| model.walk(false),
    Does::Output => Model::whole_output,
    _ => return None,
  })
}

/// The program told to stop the record. A program that is gone does not hear it, and its output,
/// which ended, already said why.
fn stop(voice: &Voice) {
  if let Some(listener) = &voice.listener {
    listener.say("stop");
  }
}

/// The pointer at a place of the window, and what it does there: whether it stands on the list,
/// and what a click does.
fn pointed(model: &mut Model, event: MouseEvent) -> (bool, Option<Does>) {
  let y = usize::from(event.y);
  model.pointer = Some((event.x, event.y));
  model.pointed_name();
  let on = model.on_list(event.x);
  let does = match (event.kind, on) {
    (MouseEventKind::Down(MouseButton::Left), true) => model.clicked(y),
    (MouseEventKind::Down(MouseButton::Left), false) => model.clicked_left(event.x, y),
    (MouseEventKind::Drag(MouseButton::Left), false) => {
      model.dragged_left(event.x, y);
      None
    }
    (MouseEventKind::ScrollUp | MouseEventKind::ScrollDown, false)
      if !event.modifiers.is_empty() =>
    {
      model.zoomed_left(event.x, y, event.kind == MouseEventKind::ScrollUp);
      None
    }
    (MouseEventKind::ScrollUp, true) => {
      model.scrolled(-isize::from(WHEEL));
      None
    }
    (MouseEventKind::ScrollDown, true) => {
      model.scrolled(isize::from(WHEEL));
      None
    }
    _ => None,
  };
  (on, does)
}

impl Window {
  /// Each input that the callbacks sent, heard in the order sent: an input that a hearing sends is
  /// heard after it, in the same drain.
  fn drain(&mut self, app: &App) {
    while let Ok(input) = self.inputs.try_recv() {
      self.heard(app, input);
    }
    self.edited_at = None;
  }

  /// One input heard.
  fn heard(&mut self, app: &App, input: Input) {
    match input {
      Input::Key(key) => {
        if !self.key(app, &key) {
          self.typed(app, &key);
        }
      }
      Input::Mouse(event) => self.mouse(app, event),
      Input::Selected(text) => self.selected(app, &text),
      Input::Focused => self.tell(app, |model| model.client.focused()),
      Input::Resized(width, height) => self.tell(app, |model| model.resized((width, height))),
      Input::Edited(text) => self.edited(app, text),
      Input::Moved => {
        let caret = self.caret();
        if self.edited_at != Some(caret) {
          self.tell(app, |model| {
            model.same_parts = true;
            model.client.complete_at(caret);
          });
        }
      }
      Input::Pasted(text) => {
        if !self.filed(app, &text) && !self.folded(app, &text) {
          self.area.insert(text);
        }
      }
      Input::Framed(at, heard) => self.framed(app, at, heard),
      Input::Found(machine) => {
        let told = Told { name: machine.name.clone(), ..Told::default() };
        self.tell(app, |model| model.client.machines.push(told));
        self.dialed(app, machine);
      }
      Input::Slow => self.tell(app, |model| model.slow = true),
      Input::Tick => self.tick(app),
      Input::Watch => self.watched(app),
      Input::Settle => {
        self.settles = None;
        self.rest(app);
      }
      Input::Keep => self.kept(app),
      Input::Turned => self.turned(app),
      Input::Clipboard(outcome) => self.clipped(app, outcome),
      Input::Wrote(written) => self.wrote(app, written),
      Input::Spoke(line, lines) => self.spoke(app, line, lines),
      Input::Unready(ready) => {
        self.tell(app, |model| drop(model.client.notice.take_if(|one| *one == ready)));
      }
      Input::Hold => self.holds(app),
      Input::Lost(begun) => self.lost(app, begun),
    }
  }

  /// A key that the window did not take, which its editor hears as the app gives it a key: the
  /// action of the key, else the text that it types. A hidden editor, as in a window that only
  /// reads, hears none, so no text that he cannot see keeps a leave from leaving. Ctrl+Z, which
  /// neither takes, gives the terminal back to the shell, where the system can.
  fn typed(&self, app: &App, key: &KeyEvent) {
    let shown = self.area.is_visible();
    if let Some(action) = self.keymap.get(key).filter(|_| shown) {
      self.area.perform(action);
    } else if let Some(text) = key.text().filter(|_| shown) {
      self.area.insert(text.encode_utf8(&mut [0; 4]));
    } else if app.options().suspend_on_ctrl_z && key.matches("ctrl+z") {
      app.suspend();
    }
  }

  /// The place of the caret of the editor, in characters.
  fn caret(&self) -> usize {
    usize::try_from(self.area.cursor_offset()).unwrap_or_default()
  }

  /// A change of the text of the editor, which the draft takes with the place of its caret.
  fn edited(&mut self, app: &App, text: String) {
    let caret = self.caret();
    self.edited_at = Some(caret);
    self.tell(app, |model| {
      // A word that he writes leaves what a first press of a key named.
      if !text.is_empty() {
        model.armed = None;
      }
      model.same_parts = true;
      model.client.edited(text, caret);
    });
    self.colored();
  }

  /// A text that he selected, copied to the clipboard, with a notice that says whether the copy
  /// took. Each row of the window is as wide as its part, so a line of the selection ends in the
  /// blanks that pad it, which no copy keeps. A drag on the bar of the time moves in time, and a
  /// click there goes back to now: neither selects what it passes.
  fn selected(&mut self, app: &App, text: &str) {
    if self.on_bar {
      app.clear_selection();
      return;
    }
    let text = text.lines().map(str::trim_end).collect::<Vec<_>>().join("\n");
    let notice = if app.copy_to_clipboard(&text) { COPIED_SELECTION } else { TAKES_NO_COPY };
    self.tell(app, |model| model.client.notice = Some(notice.to_owned()));
  }

  /// The daemon of a machine dialed, whose frames the window hears under the place of the machine
  /// in its list.
  fn dialed(&mut self, app: &App, machine: Machine) {
    let (hand, at) = (self.hand.clone(), self.askers.len());
    let asker =
      link::dialed(app, machine, move |app, heard| hand.send(app, Input::Framed(at, heard)));
    self.askers.push(asker);
  }

  /// The window hears the terminal: its keys, the edits of its input, the pointer, its focus and
  /// its size, and its clocks and the watch of its program. The window takes each key and each
  /// paste, so no callback asks the window whether it takes them.
  fn listened(&self, app: &App) {
    let hand = self.hand.clone();
    app.on_key_capture(move |cx, key| {
      cx.stop_propagation();
      hand.send(cx.app(), Input::Key(*key));
    });
    let hand = self.hand.clone();
    app.on_mouse(move |cx, event| hand.send(cx.app(), Input::Mouse(*event)));
    // The window copies a selection itself, so it can say whether the copy took.
    let hand = self.hand.clone();
    app.on_selection(move |cx, text| {
      cx.prevent_default();
      hand.send(cx.app(), Input::Selected(text.to_owned()));
    });
    let hand = self.hand.clone();
    app.on_terminal_focus(move |cx, focused| {
      if focused {
        hand.send(cx.app(), Input::Focused);
      }
    });
    let hand = self.hand.clone();
    app.on_resize(move |cx, width, height| hand.send(cx.app(), Input::Resized(width, height)));
    let hand = self.hand.clone();
    self.area.on_change(move |cx, text| hand.send(cx.app(), Input::Edited(text.to_owned())));
    // A paste of the path of an image file puts in that image, as a terminal pastes a file that he
    // drags on it, and a long text folds into its mark.
    let hand = self.hand.clone();
    self.area.on_paste(move |cx, text| {
      cx.prevent_default();
      hand.send(cx.app(), Input::Pasted(text.to_owned()));
    });
    // A move of the caret to another word completes that word.
    let hand = self.hand.clone();
    self.area.on_cursor_change(move |cx, _, _| hand.send(cx.app(), Input::Moved));
    self.after(app, PATIENCE, Input::Slow);
    let hand = self.hand.clone();
    app.set_interval(AGES, move |cx| hand.send(cx.app(), Input::Tick));
    if self.program.is_some() {
      let hand = self.hand.clone();
      app.set_interval(WATCH, move |cx| hand.send(cx.app(), Input::Watch));
    }
  }

  /// An input that the window hears once a time passed.
  fn after(&self, app: &App, time: Duration, input: Input) -> TimerId {
    let hand = self.hand.clone();
    app.set_timeout(time, move |cx| hand.send(cx.app(), input))
  }

  /// A look at the program of the window: once a build replaced it and its size and time hold
  /// still between two looks, the window writes where it stands and ends, and the program that ran
  /// it starts the new one.
  fn watched(&mut self, app: &App) {
    if !self.program.as_mut().is_some_and(Watch::replaced) {
      return;
    }
    let (file, stood) = (reload::file_of(&self.kept), self.screen.model.stood());
    if let Err(fault) = kept::keep(&file, &stood) {
      let fault = format!("The window cannot give its place to the new build: {fault}");
      return self.tell(app, |model| model.fault = Some(fault));
    }
    // A run that no longer waits for the window takes no new build: the window ends all the same.
    self.reloads.send(()).ok();
    app.quit();
  }

  /// A key while the transcript stands: the keys of a page scroll it, its own key and escape close
  /// it, and it takes each other key, so no letter goes to the input under it. Whether it took the
  /// key.
  fn read_through(&mut self, app: &App, key: &KeyEvent) -> bool {
    if self.screen.model.client.transcript.is_none() {
      return false;
    }
    match key.code {
      KeyCode::Escape => self.tell(app, |model| model.client.transcript = None),
      KeyCode::PageUp | KeyCode::PageDown => {
        let rows = f64::from(self.screen.model.size.1.saturating_sub(6));
        let by = if key.code == KeyCode::PageUp { -rows } else { rows };
        self.pages.scroll_by(0.0, by);
      }
      _ if keys::pressed(key, false) == Some(Does::Transcript) => {
        self.tell(app, |model| model.client.transcript(None));
      }
      _ => {}
    }
    true
  }

  /// The pointer while the transcript stands: the wheel scrolls it, and a click on the row under a
  /// long piece folds it or opens it whole.
  fn read_at(&mut self, app: &App, event: MouseEvent) {
    let scrolled = self.pages.scroll_position().1;
    let rows = f64::from(WHEEL);
    match event.kind {
      MouseEventKind::ScrollUp => {
        self.pages.scroll_by(0.0, -rows);
      }
      MouseEventKind::ScrollDown => {
        self.pages.scroll_by(0.0, rows);
      }
      MouseEventKind::Down(MouseButton::Left) => {
        if let Some(row) = event.y.checked_sub(2).map(|y| whole(f64::from(y) + scrolled)) {
          self.tell(app, |model| model.transcript_clicked(row));
        }
      }
      _ => {}
    }
  }

  /// A change of the model at this moment of the two clocks, which each part then shows. The
  /// loaders move on one timer, which runs only while a mark moves.
  fn tell<T>(&mut self, app: &App, change: impl FnOnce(&mut Model) -> T) -> T {
    self.told_as(app, false, change)
  }

  /// A change that moves only the clocks or the pointer, told as [`Self::tell`] tells each change:
  /// the rows of the left side change only through the clock, so the window lays them out again
  /// only for it.
  fn steadily<T>(&mut self, app: &App, change: impl FnOnce(&mut Model) -> T) -> T {
    self.told_as(app, true, change)
  }

  /// A change told as [`Self::tell`] says, steady or not.
  fn told_as<T>(&mut self, app: &App, steady: bool, change: impl FnOnce(&mut Model) -> T) -> T {
    let given = self.told(app, steady, change);
    self.restored();
    given
  }

  /// The text that the model gives back to the input put in the editor. The editor tells no
  /// change that the window makes, so the draft takes the text too.
  fn restored(&mut self) {
    if let Some(text) = self.screen.model.client.restore.take() {
      self.screen.model.client.draft.text.clone_from(&text);
      self.area.set_text(text);
      self.colored();
    }
  }

  /// A change of the model, which each part then shows, and the timer of the loaders.
  fn told<T>(&mut self, app: &App, steady: bool, change: impl FnOnce(&mut Model) -> T) -> T {
    let moved =
      if self.still { Duration::ZERO } else { app.now().saturating_duration_since(self.born) };
    let now = (self.clock)();
    // The terminal tells what it speaks some time after the window opens, so each change reads it.
    let speaks = app.session().capabilities().map(|one| one.flags).unwrap_or_default();
    cells::follow(app);
    let Self { screen, askers, .. } = self;
    let given = screen.tell(|model| {
      (model.client.now, model.client.moved, model.speaks) = (now, moved, speaks);
      let given = change(model);
      model.client.played();
      // A form that comes or goes takes its question out of the feed or gives it back.
      let formed = model.questioned();
      // What the operator saw marks the card of his thread read at once, in the same layout: a
      // card of a login that only reads, which records no Seen, would else show the unread mark
      // that the last cards of its machine gave it.
      let same = mem::take(&mut model.same_parts) && !formed;
      model.shows += u64::from(!steady && !same);
      let saw = pump(askers, model);
      model.shows += u64::from(saw);
      model.changes += u64::from(!steady || saw);
      given
    });
    match (self.screen.model.moves() && !self.still, self.moves) {
      (true, None) => {
        // The next step falls on the grid of the steps from the open of the window, so a late
        // step never shifts the frames after it and each loader keeps its beat.
        let late = u64::try_from(moved.as_millis() % MOVES.as_millis()).unwrap_or_default();
        let next = MOVES.saturating_sub(Duration::from_millis(late));
        self.moves = Some(self.after(app, next, Input::Tick));
      }
      (false, Some(timer)) => {
        app.cancel_timer(timer);
        self.moves = None;
      }
      _ => {}
    }
    given
  }

  /// A moment of the clocks with no other change: the loaders and the ages move, and the next step
  /// is set.
  fn tick(&mut self, app: &App) {
    self.moves = None;
    self.steadily(app, |_| ());
  }

  /// What a machine told. A card shows its new state at once, and the list settles when the hand
  /// is away.
  fn framed(&mut self, app: &App, at: usize, heard: Heard) {
    let changed = self.tell(app, |model| model.hear(at, heard));
    // A leave that chose what each session that works does waits until each choice went.
    let model = &self.screen.model;
    if model.acts == Some(Acts::Leaving) && !model.client.controlling() {
      return app.quit();
    }
    if changed {
      self.rest(app);
      self.keep(app);
    }
  }

  /// The list settled: now when the hand is away from it, else once the hand was away for long
  /// enough.
  fn rest(&mut self, app: &App) {
    let away = self.touched.map_or(AWAY, |at| app.now().saturating_duration_since(at));
    if away >= AWAY {
      self.tell(app, Model::settle);
    } else if self.settles.is_none() {
      self.settles = Some(self.after(app, AWAY.saturating_sub(away), Input::Settle));
    }
  }

  /// The cards written to the file of the window, a moment after they changed.
  fn keep(&mut self, app: &App) {
    if self.keeps.is_none() {
      self.keeps = Some(self.after(app, KEEPS, Input::Keep));
    }
  }

  /// The cards written, a moment after they changed. A write that fails leaves a fault that the
  /// window shows, until he reads it.
  fn kept(&mut self, app: &App) {
    self.keeps = None;
    if let Err(fault) = kept::keep(&self.kept, &self.screen.model.client.machines) {
      self.tell(app, |model| model.fault = Some(format!("The cards are not kept: {fault}")));
    }
  }

  /// The draft emptied into the stash, or put aside or brought back by the key of the stash, and
  /// the stash written to the file of the window at once, so a draft outlives a window that ends.
  fn stashed(&mut self, app: &App, does: Does) {
    if does == Does::Empty {
      self.area.set_text("");
      self.tell(app, |model| model.client.stash_draft());
    } else if self.tell(app, |model| model.client.toggle_stash()) {
      self.area.set_text("");
      self.tell(app, |model| {
        let key = model.key(Does::Stash);
        model.client.notice = Some(format!("The input is put aside. {key} brings it back."));
      });
    }
    let written = { kept::keep(&kept::stash_of(&self.kept), &self.screen.model.client.stash) };
    if let Err(fault) = written {
      self.tell(app, |model| {
        model.fault = Some(format!("The emptied drafts are not kept: {fault}"));
      });
    }
  }

  /// What the open thread said last, copied to the clipboard of the system or of the terminal, with
  /// a notice that says whether it went.
  fn copied(&mut self, app: &App) {
    let said = self
      .screen
      .model
      .client
      .thread
      .as_ref()
      .and_then(|one| one.thread.last_said().map(str::to_owned));
    let notice = match said {
      None => "Nothing to copy yet.",
      Some(text) if app.copy_to_clipboard(&text) => "Copied what the thread said last.",
      Some(_) => TAKES_NO_COPY,
    };
    self.tell(app, |model| model.client.notice = Some(notice.to_owned()));
  }

  /// The messages of the open thread that the window holds, as Markdown on the clipboard, and a
  /// notice that says whether they went.
  fn exported(&mut self, app: &App) {
    let thread = self.screen.model.exported();
    let notice = match thread {
      None => "No thread is open to copy.".to_owned(),
      Some((text, count)) if app.copy_to_clipboard(&text) => {
        format!("Copied the {count} messages of the thread as Markdown.")
      }
      Some(_) => TAKES_NO_COPY.to_owned(),
    };
    self.tell(app, |model| model.client.notice = Some(notice));
  }

  /// The input written in the editor of the operator, which takes the terminal until it ends, then
  /// gives the input the text that it saved.
  fn written_aside(&mut self, app: &App) {
    let file = env::temp_dir().join(format!("furb-input-{}.md", process::id()));
    if let Err(no) = fs::write(&file, &self.screen.model.client.draft.text) {
      let why = format!("The input cannot go to your editor: {no}");
      return self.tell(app, |model| model.client.notice = Some(why));
    }
    let (edit, hand) = (self.screen.model.edit, self.hand.clone());
    app.release_terminal(move |cx| {
      let written = edit(&file).and_then(|()| fs::read_to_string(&file));
      fs::remove_file(&file).ok();
      hand.send(cx.app(), Input::Wrote(written));
    });
  }

  /// The text that the editor of the operator saved, in the input. An editor that fails leaves the
  /// input as it was.
  fn wrote(&mut self, app: &App, written: io::Result<String>) {
    self.tell(app, |model| match written {
      Ok(mut text) => {
        text.truncate(text.trim_end_matches('\n').len());
        model.client.restore = Some(text.clone());
        model.client.draft.text = text;
      }
      Err(no) => model.client.notice = Some(format!("Your editor gave no text: {no}")),
    });
  }

  /// The image of the clipboard put in the input, else its text, as a paste puts it.
  fn clipboard(&self, app: &App) {
    let hand = self.hand.clone();
    let kinds = ["image/png", "image/jpeg", "image/gif", "image/webp", "text/plain"];
    app.read_clipboard_as(&kinds, move |cx, outcome| {
      hand.send(cx.app(), Input::Clipboard(outcome));
    });
  }

  /// What the clipboard held put in the input. A clipboard with neither an image nor a text says
  /// so.
  fn clipped(&mut self, app: &App, outcome: ClipboardOutcome) {
    match outcome {
      ClipboardOutcome::Read { mime_type, data } if mime_type.starts_with("image/") => {
        self.pictured(app, &data);
      }
      ClipboardOutcome::Read { data, .. } => {
        let text = String::from_utf8_lossy(&data);
        if !self.filed(app, &text) {
          self.area.insert(text);
        }
      }
      _ => self.tell(app, |model| {
        model.client.notice = Some("The clipboard holds no image and no text.".to_owned());
      }),
    }
  }

  /// Whether a text that he pastes is the path of an image file, which then goes in as an image.
  fn filed(&mut self, app: &App, text: &str) -> bool {
    let folder = PathBuf::from(&self.screen.model.client.folder);
    let Some(bytes) = picture::file(text, &folder) else { return false };
    self.pictured(app, &bytes);
    true
  }

  /// A long text that he pastes put in the input at its caret as its mark. The same text pasted
  /// again while its mark stands shows it whole in the place of its mark.
  fn folded(&mut self, app: &App, text: &str) -> bool {
    let draft = self.area.text();
    // A terminal ends each line of a paste with a return, as a key of the return.
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let again = self
      .screen
      .model
      .client
      .draft
      .folds
      .iter()
      .position(|one| one.text == text && draft.contains(&one.mark()));
    if let Some(at) = again {
      // The editor tells no change that the window makes, so the draft takes the text too.
      let whole = self.tell(app, |model| {
        let one = model.client.draft.folds.remove(at);
        model.client.draft.text = draft.replacen(&one.mark(), &one.text, 1);
        model.client.draft.text.clone()
      });
      self.area.set_text(whole);
    } else {
      let Some(mark) = self.tell(app, |model| model.client.fold(&text)) else { return false };
      self.area.insert(format!("{mark} "));
    }
    self.colored();
    true
  }

  /// An image put in the input at its caret as its mark, or why it does not go.
  fn pictured(&mut self, app: &App, bytes: &[u8]) {
    let put = match images::kind(bytes) {
      Ok((_, suffix)) => self.tell(app, |model| model.client.paste_image(bytes, suffix)),
      Err(why) => Err(why.to_string()),
    };
    match put {
      Ok(mark) => {
        self.area.insert(format!("{mark} "));
      }
      Err(why) => self.tell(app, |model| model.client.notice = Some(why)),
    }
    self.colored();
  }

  /// A page of older exchanges that the open thread asks of its machine, said, with the id of its
  /// request kept until its answer comes.
  fn older(&mut self, verb: Option<Verb>) {
    let Some(verb @ Verb::Thread { before, .. }) = verb else { return };
    let Self { askers, screen, .. } = self;
    let Some(at) = screen.model.client.thread.as_ref().map(|one| one.thread.key.0) else { return };
    let Some(id) = ask(askers, &mut screen.model, at, verb) else { return };
    let Some(opened) = screen.model.client.thread.as_mut() else { return };
    opened.thread.asked.insert(id, before);
  }

  /// A key of the window, from the one table of the keys, before the editor of the input hears it:
  /// whether the window took it. A key that follows the leader never reaches the editor.
  fn key(&mut self, app: &App, key: &KeyEvent) -> bool {
    if key.kind == KeyEventKind::Release {
      self.released(app, key);
      return false;
    }
    self.voiced(app, key).unwrap_or_else(|| self.keyed(app, key))
  }

  /// A release of a key, which says that the terminal tells each one, and which plays or stops the
  /// bar of the time for a tap of the space that waited for it. A space released ends its run, so
  /// two quick taps never read as a hold.
  fn released(&mut self, app: &App, key: &KeyEvent) {
    self.voice.releases = true;
    let space = key.code == KeyCode::Char(' ');
    if space {
      self.voice.other();
    }
    if let Some(tap) = self.voice.waits.take_if(|_| space) {
      self.keyed(app, &tap);
    }
  }

  /// Whether a space plays or stops the bar of the time: the bar shows with no chains, no leader
  /// waits for its key, and the input is empty.
  fn plays(&self) -> bool {
    let thread = self.screen.model.client.thread.as_ref();
    let time = thread.and_then(|one| one.thread.time.as_ref());
    time.is_some_and(|time| time.engaged() && time.chains.is_none())
      && self.led.is_none()
      && self.area.text().is_empty()
  }

  /// A key that the voice left: the leader, a list part, the completion, the review, the bar of the
  /// time, a form, the controls of the views, then the table of the keys. Whether the window took
  /// it; the editor of the input hears each other key.
  fn keyed(&mut self, app: &App, key: &KeyEvent) -> bool {
    let led = self.led.take();
    let led = led.is_some_and(|at| app.now().saturating_duration_since(at) <= LEADER);
    if keys::leads(key) {
      self.led = Some(app.now());
      return true;
    }
    if self.screen.model.picker.is_some() && self.picking(app, key) {
      return true;
    }
    // A key after the leader is a key of the window, which no text that finds a chain takes.
    let over = self.read_through(app, key) || self.looked(app, key);
    if self.completing(app, key) || over || !led && self.timed(app, key) {
      return true;
    }
    let does = keys::pressed(key, led);
    // A letter after the leader is a key of the window, and no letter of a text.
    let typed = !led
      && key.text().is_some()
      && (key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
    let digit = key.text().and_then(|one| one.to_digit(10)).filter(|_| typed);
    let digit = digit.and_then(|one| usize::try_from(one).ok());
    if self.answered(app, (does, typed), digit) || self.reached(app, does, typed) {
      return true;
    }
    match does {
      Some(Does::Up | Does::Down) if self.walked(app, does == Some(Does::Up)) => true,
      // The grey rest, which the editor hears as a move when none shows.
      Some(Does::Take | Does::TakeWord) => self.took(does == Some(Does::TakeWord)),
      // The keys of a form, which the editor hears when no form takes them.
      Some(
        Does::Up
        | Does::Down
        | Does::Lean
        | Does::Tick
        | Does::NextQuestion
        | Does::PreviousQuestion,
      ) => false,
      Some(Does::Leave) if !self.area.text().is_empty() => false,
      Some(does) => {
        self.act(app, does);
        true
      }
      None => led,
    }
  }

  /// An arrow up on the first row of the input, or down on its last row, which walks the history of
  /// the input: whether the walk took it. An arrow on another row moves in the input.
  fn walked(&mut self, app: &App, up: bool) -> bool {
    let (row, rows) = (self.area.cursor().0, self.area.text().split('\n').count());
    let edge = if up { row == 0 } else { usize::try_from(row).is_ok_and(|row| row + 1 >= rows) };
    edge && self.tell(app, |model| model.client.walked(up))
  }

  /// The grey rest of the input taken whole, or its first word, at the end of the text: whether a
  /// rest showed there. The editor tells the change, as a text that he wrote.
  fn took(&self, word: bool) -> bool {
    let area = &self.area;
    let end =
      usize::try_from(area.cursor_offset()).is_ok_and(|at| at == area.text().chars().count());
    let Some(rest) = self.screen.model.client.suggested().filter(|_| end) else { return false };
    // A word is the blanks before it and its letters, as the editor moves by words.
    let start = rest.len() - rest.trim_start().len();
    let word_end = rest
      .get(start..)
      .and_then(|one| one.find(char::is_whitespace))
      .map_or(rest.len(), |at| start + at);
    let rest = if word { rest.get(..word_end).unwrap_or(rest) } else { rest };
    area.insert(rest);
    true
  }

  /// A key while a list part is open: it filters the part, moves its chosen row, takes it or closes
  /// it. Whether the part took the key.
  fn picking(&mut self, app: &App, key: &KeyEvent) -> bool {
    let text = key.text().filter(|_| key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
    let taken = self.tell(app, |model| {
      let at = model.picker.as_ref().map(|one| one.at)?;
      match (key.code, text) {
        (KeyCode::Escape, _) => model.closed_part(),
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
      self.act(app, does);
    }
    true
  }

  /// A key while the completion of the input shows: the arrows move its chosen row, enter and tab
  /// take it, and escape hides it. Whether the completion took the key.
  fn completing(&mut self, app: &App, key: &KeyEvent) -> bool {
    let shows = {
      let model = &self.screen.model;
      model.client.completion.as_ref().is_some_and(|one| !one.hidden) && model.picker.is_none()
    };
    if !shows || !key.modifiers.is_empty() {
      return false;
    }
    match key.code {
      KeyCode::Up => self.tell(app, |model| model.client.choose(-1)),
      KeyCode::Down => self.tell(app, |model| model.client.choose(1)),
      KeyCode::Escape => self.tell(app, |model| model.client.hide()),
      KeyCode::Enter | KeyCode::Tab => return self.completed(app, None),
      _ => return false,
    }
    true
  }

  /// A key while the review of the changes stands: the arrows read another file, the keys of a page
  /// scroll its diff, escape and the key of the review close it, and it takes each other key, so no
  /// letter goes to the input under it. While a comment on a line is open, each key but escape goes
  /// to its draft, and escape drops the comment. Whether the review took the key.
  fn looked(&mut self, app: &App, key: &KeyEvent) -> bool {
    if self.screen.model.client.review.is_none() {
      return false;
    }
    if self.screen.model.commenting().is_some() {
      if key.code == KeyCode::Escape {
        self.tell(app, |model| model.client.draft.anchor = None);
        return true;
      }
      return false;
    }
    match key.code {
      KeyCode::Up => self.tell(app, |model| model.client.review_by(-1)),
      KeyCode::Down => self.tell(app, |model| model.client.review_by(1)),
      KeyCode::Escape => self.tell(app, |model| model.client.review = None),
      KeyCode::PageUp | KeyCode::PageDown => {
        let rows = f64::from(self.screen.model.size.1.saturating_sub(6));
        let by = if key.code == KeyCode::PageUp { -rows } else { rows };
        self.scroll.scroll_by(0.0, by);
      }
      KeyCode::Char(sign) if [PREVIOUS, NEXT].contains(&sign.to_string().as_str()) => {
        let shown = &*self;
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
        self.tell(app, |model| model.client.review = None);
      }
      _ => {}
    }
    true
  }

  /// A key while the bar of the time of the open thread shows: escape closes it back to now, and on
  /// an empty input the arrows step one moment back or on, and space plays or stops. Whether the
  /// bar took the key.
  fn timed(&mut self, app: &App, key: &KeyEvent) -> bool {
    let shows = {
      let model = &self.screen.model;
      let time = model.client.thread.as_ref().and_then(|one| one.thread.time.as_ref());
      time.is_some_and(furb_client::Time::engaged)
    };
    let finds = self
      .screen
      .model
      .client
      .thread
      .as_ref()
      .is_some_and(|one| one.thread.time.as_ref().is_some_and(|time| time.chains.is_some()));
    if finds {
      return self.found(app, key);
    }
    let empty = self.area.text().is_empty();
    if !shows || !key.modifiers.is_empty() {
      return false;
    }
    match key.code {
      KeyCode::Escape => self.tell(app, |model| model.client.time()),
      KeyCode::Left if empty => self.tell(app, |model| model.client.step(-1)),
      KeyCode::Right if empty => self.tell(app, |model| model.client.step(1)),
      KeyCode::Char(' ') if empty => self.tell(app, |model| model.client.play()),
      KeyCode::Char('+' | '=') if empty => self.tell(app, |model| model.client.zoom(true, None)),
      KeyCode::Char('-') if empty => self.tell(app, |model| model.client.zoom(false, None)),
      _ => return false,
    }
    true
  }

  /// A key while the chains show under the bar of the time: a letter finds a chain by its label,
  /// enter opens the first chain that shows at the moment that the thread shows, and escape hides
  /// the chains. Whether the chains took the key.
  fn found(&mut self, app: &App, key: &KeyEvent) -> bool {
    let text = key.text().filter(|_| key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT);
    match (key.code, text) {
      (KeyCode::Escape, _) => self.tell(app, |model| model.client.chains()),
      (KeyCode::Backspace, _) => self.tell(app, |model| {
        model.client.find(|find| {
          find.pop();
        });
      }),
      (KeyCode::Enter, _) => self.tell(app, |model| {
        let at = model.client.thread.as_ref().and_then(|one| one.thread.time.as_ref());
        let at = at.map(furb_client::Time::place).unwrap_or_default();
        model.open_chain(0, at);
      }),
      (_, Some(one)) => self.tell(app, |model| model.client.find(|find| find.push(one))),
      _ => return false,
    }
    true
  }

  /// The pointer while the review of the changes stands: a click on a file reads it, the name of
  /// the file or the folder under it goes bright, a click on a folder opens the sub thread about
  /// it, or turns the input to a message about it, a click on the numbers of a line opens the sub
  /// thread about it, or starts a comment on it under the line, a click on its code does nothing,
  /// and the wheel scrolls the diff.
  fn looked_at(&mut self, app: &App, event: MouseEvent) {
    let file = self.screen.model.review_at(event.x, event.y);
    // The diff stands under the two rows of the head, from the row where its box scrolled.
    let scrolled = self.scroll.scroll_position().1;
    let row = event.y.checked_sub(2).map(|y| whole(f64::from(y) + scrolled));
    self.tell(app, |model| {
      model.pointer = Some((event.x, event.y));
      model.review_top = whole(scrolled);
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
        && model.gutter_at(event.x)
        && let Some(row) = row
      {
        // Only the numbers of a line take a click, as on GitHub, so a click that reads the code
        // anchors nothing and leaves the diff where it stands.
        match model.sub_on(row) {
          Some(sub) => {
            model.client.review = None;
            model.open_agent(sub);
          }
          None => model.client.draft.anchor = model.anchor_at(event.x.max(1 + MARKS), row),
        }
      }
    });
    let rows = f64::from(WHEEL);
    let by = match event.kind {
      MouseEventKind::ScrollUp => -rows,
      MouseEventKind::ScrollDown => rows,
      _ => return,
    };
    self.scroll.scroll_by(0.0, by);
    let top = whole(self.scroll.scroll_position().1);
    self.tell(app, |model| model.review_top = top);
  }

  /// A row of the completion taken, at a place or the chosen one: the editor takes the text with
  /// its name and the caret after it, and a command runs. Whether a row was taken.
  fn completed(&mut self, app: &App, place: Option<usize>) -> bool {
    let Some(taken) = self.tell(app, |model| model.client.take(place)) else { return false };
    self.area.set_text(&taken.text);
    self.area.set_cursor_offset(u32::try_from(taken.caret).unwrap_or(u32::MAX));
    self.tell(app, |model| model.client.complete_at(taken.caret));
    // The editor drops its marks with the text that it took, so the new text takes them again.
    self.colored();
    if let Some(does) = taken.command.as_deref().and_then(Model::command) {
      self.act(app, does);
    }
    true
  }

  /// A key or a click while a form of a question shows: whether the form took it. An answer goes to
  /// its machine, and waits there for its answer.
  fn answered(
    &mut self,
    app: &App,
    (does, typed): (Option<Does>, bool),
    digit: Option<usize>,
  ) -> bool {
    // A key with no form tells the model nothing, so it draws no part again.
    if self.screen.model.form().is_none() {
      return false;
    }
    match self.tell(app, |model| model.keyed(does, typed, digit)) {
      Keyed::Passed => false,
      Keyed::Taken => true,
      Keyed::Empties => {
        self.area.set_text("");
        self.tell(app, |model| model.client.draft.text.clear());
        true
      }
      Keyed::Says(at, verb) => {
        let Self { askers, screen, .. } = self;
        match ask(askers, &mut screen.model, at, verb) {
          Some(id) => drop(screen.model.client.asked.insert((at, id), Asked::Answer)),
          None => screen.model.unanswered(screen.model.client.away(at).unwrap_or_default()),
        }
        self.tell(app, |_| ());
        true
      }
    }
  }

  /// A key while the keys reach a control of the views: the arrows move among the controls, Enter
  /// on an empty input uses the one that they reach, and Escape gives the keys back to the input.
  /// A letter that he types, or Enter on his text, gives the keys back to the input too, which then
  /// takes it: a view never takes his words or his Enter. Whether it took the key.
  fn reached(&mut self, app: &App, does: Option<Does>, typed: bool) -> bool {
    let (aims, empty) =
      { (self.screen.model.client.aimed.is_some(), self.area.text().trim().is_empty()) };
    if !aims {
      return false;
    }
    let change: fn(&mut Model) = match does {
      Some(Does::Up) => |model| model.client.step_control(-1),
      Some(Does::Down) => |model| model.client.step_control(1),
      Some(Does::Send) if empty => |model| model.client.use_control(),
      Some(Does::Close) => |model| model.client.aimed = None,
      _ if typed || does == Some(Does::Send) => {
        self.tell(app, |model| model.client.aimed = None);
        return false;
      }
      _ => return false,
    };
    self.tell(app, change);
    true
  }

  /// The feed a page up or down. A page keeps two rows of the screen before it in view.
  fn paged(&mut self, app: &App, up: bool) {
    let verb = self.tell(app, |model| {
      let room =
        model.laying(|model, kept| Some(model.feed_room(model.feed(kept)?.dock().len())))?;
      let rows = isize::try_from(room).unwrap_or_default() - 2;
      model.scroll_feed(if up { -rows } else { rows })
    });
    self.older(verb);
  }

  /// What a key does, which a click on its text does too. Any other key leaves what a first press
  /// of a key named, and what the key before did not do.
  fn act(&mut self, app: &App, does: Does) {
    let forgot = self.screen.model.pressed(does);
    self.asked(does);
    if let Some(pick) = Pick::of(does) {
      return self.changed(app, does, |model| model.pick(pick));
    }
    if let Some(change) = change_of(does) {
      return self.changed(app, does, change);
    }
    match does {
      Does::Close => self.escaped(app, forgot),
      Does::Stop | Does::Resolve | Does::Revert | Does::Pause => {
        self.controlled(app, |model| Some((model.control(does)?, Asked::Control)));
      }
      // The window takes its own newest build too, at once and not at the next look.
      Does::Reload => {
        self.controlled(app, |model| Some((model.control(does)?, Asked::Control)));
        self.watched(app);
      }
      Does::Send => self.sent(app),
      Does::Queue => self.queued(app),
      Does::Empty if !self.area.text().is_empty() => self.stashed(app, does),
      Does::Stash => self.stashed(app, does),
      Does::Picture => self.clipboard(app),
      Does::Copy => self.copied(app),
      Does::Export => self.exported(app),
      Does::Editor => self.written_aside(app),
      Does::Empty => self
        .controlled(app, |model| model.take_back().map(|(asked, text)| (asked, Asked::Back(text)))),
      Does::Leave => self.left(app),
      Does::PageUp | Does::PageDown => self.paged(app, does == Does::PageUp),
      Does::MessageBefore | Does::MessageAfter => {
        let verb = self.tell(app, |model| model.step_message(does == Does::MessageBefore));
        self.older(verb);
      }
      // The keys of a form, of a list part and of the input, which the key that heard them did, and
      // the keys that open a list part, which the start of this function took.
      _ => {}
    }
  }

  /// The change that a key makes to the window, then the thread that it opens.
  fn changed(&mut self, app: &App, does: Does, change: impl FnOnce(&mut Model)) {
    if matches!(does, Does::Next | Does::Previous | Does::Needed) {
      self.touched = Some(app.now());
    }
    self.tell(app, |model| {
      change(model);
      model.open_thread();
    });
    self.colored();
  }

  /// What a list part shows asked of each machine as it opens: the records of each machine for Go
  /// to, and the projects of each machine, or of the machine of the input, for their pickers.
  fn asked(&mut self, does: Does) {
    match does {
      Does::GoTo => self.listed(),
      Does::Machines => self.projected(None),
      Does::Projects => {
        let at = self.screen.model.client.target().machine();
        self.projected(Some(at));
      }
      _ => {}
    }
  }

  /// The records of each machine that lists its cards asked, for Go to.
  fn listed(&mut self) {
    let Self { askers, screen, .. } = self;
    for at in 0..askers.len() {
      if let Some(id) = ask(askers, &mut screen.model, at, Verb::List { config: None }) {
        screen.model.client.asked.insert((at, id), Asked::List);
      }
    }
  }

  /// The projects of a machine asked, or of each machine that lists its cards, for the parts that
  /// choose where a new session runs.
  fn projected(&mut self, of: Option<usize>) {
    let Self { askers, screen, .. } = self;
    for at in (0..askers.len()).filter(|at| of.is_none_or(|of| of == *at)) {
      if let Some(id) = ask(askers, &mut screen.model, at, Verb::Projects { config: None }) {
        screen.model.client.asked.insert((at, id), Asked::Projects);
      }
    }
  }

  /// A leave: a first press arms it, and a second leaves, or asks first what each session that the
  /// window holds and that works does.
  fn left(&mut self, app: &App) {
    let leaves = self.tell(app, |model| {
      model.armed.replace(Armed::Leave) == Some(Armed::Leave) && model.leaves_now()
    });
    if leaves {
      app.quit();
    }
  }

  /// The text of the input sent to its target, and the input empty. A backslash just before the
  /// caret breaks the line in its place and sends nothing, as a terminal with no keys of Kitty
  /// tells no Shift+Enter.
  fn sent(&mut self, app: &App) {
    let text = self.area.text();
    let caret = self.caret();
    let chars: Vec<char> = text.chars().collect();
    if caret > 0 && chars.get(caret - 1) == Some(&'\\') {
      let broken: String =
        chars.iter().take(caret - 1).chain(['\n'].iter()).chain(chars.iter().skip(caret)).collect();
      self.area.set_text(&broken);
      self.area.set_cursor_offset(u32::try_from(caret).unwrap_or(u32::MAX));
      self.tell(app, |model| model.client.edited(broken, caret));
      return;
    }
    if !text.trim().is_empty() {
      self.seen();
    }
    if text.trim().is_empty() || !self.tell(app, |model| model.send(text)) {
      return;
    }
    self.area.set_text("");
    self.tell(app, |model| model.client.draft.text.clear());
    // A thread that the message starts shows in the list at once.
    self.rest(app);
  }

  /// What the operator saw as he sent a message, written beside the cards of the window, so a chain
  /// that he asks about his window draws it again as he saw it.
  fn seen(&mut self) {
    let model = &self.screen.model;
    let seen = Seen { cols: model.size.0, rows: model.size.1, stood: model.stood() };
    // A file that cannot be written keeps what he saw at the send before, which no send needs.
    kept::keep(&reload::seen_of(&self.kept), &seen).ok();
  }

  /// The text of the input queued on the open thread, which its life sends once the thread rests.
  fn queued(&mut self, app: &App) {
    let text = self.area.text();
    if text.trim().is_empty() || !self.tell(app, |model| model.queue(&text)) {
      return;
    }
    self.area.set_text("");
    self.tell(app, |model| model.client.draft.text.clear());
  }

  /// An escape: it forgets what a first press of another key named, goes back from a thread that
  /// opens or that failed to open, closes the list that stands over the home, or arms the stop of
  /// the work of the open thread, which the next escape makes.
  fn escaped(&mut self, app: &App, forgot: bool) {
    if forgot {
      return self.tell(app, |_| ());
    }
    if self.tell(app, Model::back) {
      return;
    }
    if self.screen.model.armed == Some(Armed::Stop) {
      return self.act(app, Does::Stop);
    }
    let closes = {
      let model = &self.screen.model;
      model.toggled && !model.listed()
    };
    if closes {
      self.tell(app, Model::close);
    } else {
      self.tell(app, Model::arm_stop);
    }
  }

  /// A control of the open thread that a key asks, as a stop, a resolve or a take back, asked of
  /// its machine with what the window waits for. A machine that the window dials again takes
  /// nothing.
  fn controlled(
    &mut self,
    app: &App,
    change: impl FnOnce(&mut Model) -> Option<((usize, Verb), Asked)>,
  ) {
    let Some(((at, verb), waits)) = self.tell(app, change) else { return self.colored() };
    let Self { askers, screen, .. } = self;
    match ask(askers, &mut screen.model, at, verb) {
      Some(id) => drop(screen.model.client.asked.insert((at, id), waits)),
      None => screen.model.fault = screen.model.client.away(at),
    }
    self.tell(app, |_| ());
  }

  /// The text of the input in the colors of python while it is a word, and otherwise plain with
  /// each mention of a file marked, so a file that the picker put in the text stands out.
  fn colored(&mut self) {
    let (area, model) = (&self.area, &self.screen.model);
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
        let start =
          u32::try_from(cells(text.get(..span.start).unwrap_or_default())).unwrap_or_default();
        start..start + u32::try_from(cells(text.get(span).unwrap_or_default())).unwrap_or_default()
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
    let model = &mut self.screen.model;
    let lines = model.kept.colors.lines(&model.client.draft.text, plain);
    for (row, line) in lines.iter().enumerate() {
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
  /// rows under it. A click on a card opens its thread, and a click on a thing of the left side
  /// does what its key does.
  fn mouse(&mut self, app: &App, event: MouseEvent) {
    // The side keys of a mouse, which a terminal tells as the buttons 8 and 9, step back and
    // forward as in a browser, wherever the pointer stands.
    if let MouseEventKind::Down(MouseButton::Other(side @ (8 | 9))) = event.kind {
      return self.act(app, if side == 8 { Does::Back } else { Does::Forward });
    }
    if self.screen.model.picker.is_some() {
      let does = self.tell(app, |model| {
        model.pointer = Some((event.x, event.y));
        model.pointed_over(event)
      });
      if let Some(does) = does {
        self.act(app, does);
      }
      return;
    }
    let client = &self.screen.model.client;
    if client.transcript.is_some() || client.review.is_some() {
      if client.transcript.is_some() {
        self.read_at(app, event);
      } else {
        self.looked_at(app, event);
      }
      // A press there gives the focus to the box that scrolls, which takes no paste: the editor of
      // the input takes it back, which writes the comment that a press on a line starts.
      if matches!(event.kind, MouseEventKind::Down(_)) && self.area.is_visible() {
        app.focus(&self.area);
      }
      return;
    }
    let place = self.screen.model.completion_at(event.x, event.y);
    if let Some(place) = place {
      self.tell(app, |model| model.pointer = Some((event.x, event.y)));
      if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
        self.completed(app, Some(place));
      }
      return;
    }
    if let Some(by) = self.over_feed(event) {
      return self.wheeled(app, event, by);
    }
    if matches!(event.kind, MouseEventKind::Down(_)) {
      let bar = self.screen.model.on_bar(event.x, event.y);
      self.on_bar = bar;
    }
    let change = |model: &mut Model| pointed(model, event);
    let (on, does) = if event.kind == MouseEventKind::Moved {
      self.steadily(app, change)
    } else {
      self.tell(app, change)
    };
    if on {
      self.touched = Some(app.now());
    }
    if let Some(does) = does
      && !self.answered(app, (Some(does), false), None)
    {
      self.act(app, does);
    }
  }

  /// A key that the voice takes, whether the window took it: each space, and escape during a
  /// record, which drops it. A space that is a tap goes where a key goes, to the input or to a part
  /// that takes it, as the bar of the time plays on it; a space that holds the bar records his
  /// voice into the input wherever it shows, once the taps of the hold are undone, and a repeat
  /// holds the record. A list part and the review stand over the input, so they take each space.
  /// Any other key goes on.
  fn voiced(&mut self, app: &App, key: &KeyEvent) -> Option<bool> {
    let recording = self.voice.record.is_some();
    if recording && key.code == KeyCode::Escape {
      self.dropped(app, None);
      return Some(true);
    }
    let over = {
      let model = &self.screen.model;
      model.picker.is_some() || model.client.review.is_some()
    };
    let space = key.code == KeyCode::Char(' ') && key.modifiers.is_empty();
    if !space || over && !recording {
      // Any other key ends a record, with the text so far, and then goes where it goes.
      if recording {
        self.ended(app);
      }
      self.voice.other();
      return None;
    }
    let heard = self.voice.space(app.now());
    Some(match heard {
      // The repeat of a space that waits for its release: a hold, which the bar never hears. A
      // first space starts a new run, so a tap that waits from a run before it is gone.
      Space::Taps(false) if self.voice.waits.take().is_some() => true,
      Space::Taps(first) => {
        if first {
          self.voice.waits = None;
          let parts = self
            .screen
            .model
            .client
            .thread
            .as_ref()
            .map(|one| (one.thread.time.clone(), one.form.clone()))
            .unwrap_or_default();
          self.voice.keep(parts);
        }
        // A terminal that tells each release lets the space of the bar wait for it, so a tap plays
        // or stops the bar at its release and the press of a hold never does.
        if self.voice.releases && self.plays() {
          self.voice.waits = Some(*key);
          return Some(true);
        }
        let parted = self.keyed(app, key);
        self.voice.tapped(parted);
        parted
      }
      Space::Holds => true,
      Space::Starts(run) => {
        let Run { typed, parts, .. } = *run;
        self.tell(app, |model| {
          let Some(opened) = model.client.thread.as_mut() else { return };
          let (time, form) = parts;
          if opened.thread.time != time {
            opened.thread.time = time;
            opened.thread.moved();
          }
          opened.form = form;
        });
        self.recorded(app, typed);
        true
      }
    })
  }

  /// The program of the speech started, where the window has one, and each line that it says heard.
  /// The window starts it with its first frame, so the speech is ready before the first hold.
  fn readied(&mut self, app: &App) -> Result<(), String> {
    let (named, folder, project) = {
      let folder = self.kept.parent().map(Path::to_path_buf).unwrap_or_default();
      (self.listen.clone(), folder, PathBuf::from(&self.screen.model.client.folder))
    };
    // The configs say the language that he speaks and the task, read again at each start.
    let voice = Layers::new(&project, None).values().ok().and_then(|mut all| all.remove(KEY_VOICE));
    let said = |key: &str, unsaid: &str| {
      voice.as_ref().and_then(|one| one.get(key)?.as_str()).unwrap_or(unsaid).to_owned()
    };
    let (language, task) = (said("language", "auto"), said("task", "transcribe"));
    let program = voice::program(named.as_deref(), &folder)
      .map_err(|no| format!("The program of the speech cannot be written: {no}"))?
      .ok_or_else(|| "This window has no speech on this system yet.".to_owned())?;
    let (listener, output) =
      voice::started(&program, &folder.join("voice"), (&language, &task), &app.runtime())
        .map_err(|no| format!("The speech did not start: {no}"))?;
    self.voice.listener = Some(listener);
    self.listening(app, FramedRead::new(output, LinesCodec::new_with_max_length(BOUND)));
    Ok(())
  }

  /// A record of his voice started, once the spaces of the hold left the input: the program of the
  /// speech opens the microphone, and what it hears writes itself at the caret. A system with no
  /// speech, or a program that does not start, says so, and the spaces stay.
  fn recorded(&mut self, app: &App, spaces: usize) {
    let ready = self.voice.listener.is_some() || {
      let readied = self.readied(app);
      readied.map_err(|why| self.tell(app, |model| model.client.notice = Some(why))).is_ok()
    };
    let started = ready && self.voice.listener.as_ref().is_some_and(|one| one.say("start"));
    if !started {
      return;
    }
    let text: Vec<char> = self.area.text().chars().collect();
    let caret = self.caret().min(text.len());
    let typed = text.iter().take(caret).rev().take(spaces).take_while(|one| **one == ' ').count();
    let before = text.iter().take(caret.saturating_sub(typed)).collect();
    let after = text.iter().skip(caret).collect();
    self.voice.record = Some(Record { before, after, last: app.now(), stops: false });
    self.tell(app, |model| {
      model.listening = Some(Vec::new());
      // The notice of the speech said all it had to once he speaks.
      model.client.notice.take_if(|one| one.starts_with(VOICE));
    });
    self.spoken(app, "");
    self.held(app);
  }

  /// The text of the voice so far written in the input at the caret where the record began, with a
  /// blank between it and a word before it.
  fn spoken(&mut self, app: &App, said: &str) {
    let Some((text, caret)) = self.voice.record.as_ref().map(|record| {
      let blank =
        !said.is_empty() && record.before.chars().last().is_some_and(|one| !one.is_whitespace());
      let head = format!("{}{}{said}", record.before, if blank { " " } else { "" });
      let caret = head.chars().count();
      (format!("{head}{}", record.after), caret)
    }) else {
      return;
    };
    self.area.set_text(&text);
    self.area.set_cursor_offset(u32::try_from(caret).unwrap_or(u32::MAX));
    // The editor tells no change that the window makes, so the draft takes the text too.
    self.tell(app, |model| model.client.draft.text = text);
  }

  /// The next line of the program of the speech, heard on the app.
  fn listening(&self, app: &App, mut lines: FramedRead<ChildStdout, LinesCodec>) {
    let hand = self.hand.clone();
    let next = async move { (lines.next().await, lines) };
    drop(app.spawn(next, move |cx, (line, lines)| {
      hand.send(cx.app(), Input::Spoke(line, lines));
    }));
  }

  /// A line of the program of the speech, until its output ends: then the program is gone, and
  /// the next hold starts it again.
  fn spoke(
    &mut self,
    app: &App,
    line: Option<Result<String, LinesCodecError>>,
    lines: FramedRead<ChildStdout, LinesCodec>,
  ) {
    let said = match line {
      Some(Ok(line)) => serde_json::from_str::<Said>(&line)
        .map_err(|no| format!("The speech said what the window cannot read: {no}")),
      Some(Err(no)) => Err(format!("The speech cannot be read: {no}")),
      None => Err("The speech ended.".to_owned()),
    };
    match said {
      Ok(Said::Ready { .. }) => {
        let ready = format!("{VOICE}is ready: hold the space bar to speak.");
        let said = self.tell(app, |model| {
          let loading = model.client.notice.as_deref().is_some_and(|one| one.starts_with(VOICE));
          loading.then(|| model.client.notice = Some(ready.clone())).is_some()
        });
        // The notice leaves by itself a few seconds later, unless another took its place.
        if said {
          self.after(app, READY, Input::Unready(ready));
        }
      }
      // The progress of the voice takes the place of a notice of its own alone, so it never wipes
      // what a command or a key said to the operator.
      Ok(Said::Loading { loading, done }) => self.tell(app, |model| {
        if model.client.notice.as_deref().is_none_or(|one| one.starts_with(VOICE)) {
          model.client.notice = Some(match loading {
            Stage::Download => format!("{VOICE}downloads its model, once: {}%", done.unwrap_or(0)),
            Stage::Prepare => {
              format!("{VOICE}prepares its model, which takes minutes the first time.")
            }
          });
        }
      }),
      Ok(Said::Level { level }) => self.tell(app, |model| {
        if let Some(levels) = model.listening.as_mut() {
          levels.push(level);
          let gone = levels.len().saturating_sub(LEVELS);
          levels.drain(..gone);
        }
      }),
      Ok(Said::Text { text, ends }) => {
        self.spoken(app, &text);
        if ends {
          self.voice.record = None;
          self.tell(app, |model| model.listening = None);
        }
      }
      Ok(Said::Fault { error }) => self.dropped(app, Some(error)),
      Err(why) => {
        self.voice.listener = None;
        return self.dropped(app, Some(why));
      }
    }
    self.listening(app, lines);
  }

  /// The record looked at after each pause of the repeats of the bar.
  fn held(&self, app: &App) {
    self.after(app, RELEASED, Input::Hold);
  }

  /// The record looked at after a pause of the repeats of the bar: once they stopped, he let the
  /// bar go, and the program closes the microphone and gives its final text. A program that gives
  /// no final text never keeps the record: the text so far stays his.
  fn holds(&mut self, app: &App) {
    let now = app.now();
    let Some(record) = self.voice.record.as_mut().filter(|one| !one.stops) else { return };
    if now.saturating_duration_since(record.last) < RELEASED {
      return self.held(app);
    }
    record.stops = true;
    let begun = record.last;
    stop(&self.voice);
    self.after(app, STOPPED, Input::Lost(begun));
  }

  /// The record that stopped at a moment, which its program never ended with a final text.
  fn lost(&mut self, app: &App, begun: Instant) {
    if self.voice.record.as_ref().is_some_and(|one| one.stops && one.last == begun) {
      self.voice.record = None;
      self.tell(app, |model| {
        model.listening = None;
        model.client.notice = Some("The speech gave no final text: the text so far stays.".into());
      });
    }
  }

  /// The record ended at once with the text so far, as a key that is no space ends it: the program
  /// stops, and its final text, which comes to no record, writes nothing.
  fn ended(&mut self, app: &App) {
    let Some(record) = self.voice.record.take() else { return };
    if !record.stops {
      stop(&self.voice);
    }
    self.tell(app, |model| model.listening = None);
  }

  /// The record dropped, with the input as it stood before it, and why when a fault ended it. The
  /// program stops the record, whose text then comes to no record. A fault with no record says
  /// nothing: the program starts with the window, before he asks for it, and the next hold starts
  /// it again and says why it fails then.
  fn dropped(&mut self, app: &App, why: Option<String>) {
    let Some(record) = self.voice.record.take() else { return };
    if !record.stops {
      stop(&self.voice);
    }
    let text = format!("{}{}", record.before, record.after);
    let caret = record.before.chars().count();
    self.area.set_text(&text);
    self.area.set_cursor_offset(u32::try_from(caret).unwrap_or(u32::MAX));
    self.tell(app, |model| {
      model.client.draft.text = text;
      model.listening = None;
      if why.is_some() {
        model.client.notice = why;
      }
    });
  }

  /// The rows that a step of the wheel moves the feed, when it turns over the feed.
  fn over_feed(&mut self, event: MouseEvent) -> Option<isize> {
    let rows = isize::from(WHEEL);
    let by = match event.kind {
      MouseEventKind::ScrollUp => -rows,
      MouseEventKind::ScrollDown => rows,
      _ => return None,
    };
    // The wheel with a modifier zooms the bar of the time, where the pointer stands on its track.
    let zooms = !event.modifiers.is_empty() && {
      let row = self.screen.model.left_rows().get(usize::from(event.y));
      matches!(row.and_then(|one| one.click(usize::from(event.x))), Some(Click::Time(_)))
    };
    (!zooms && !self.screen.model.on_list(event.x)).then_some(by)
  }

  /// A step of the wheel over the feed, which waits for the end of the input that the terminal sent
  /// with it: the steps that come together move the feed in one change, so in one layout and one
  /// frame.
  fn wheeled(&mut self, app: &App, event: MouseEvent, by: isize) {
    let waits = self.wheel.is_some();
    let rows = self.wheel.map_or(0, |one| one.0) + by;
    self.wheel = Some((rows, (event.x, event.y)));
    if !waits {
      self.after(app, Duration::ZERO, Input::Turned);
    }
  }

  /// The steps of the wheel that came together, told as one change of the feed.
  fn turned(&mut self, app: &App) {
    let Some((by, at)) = self.wheel.take() else { return };
    let verb = self.tell(app, |model| {
      model.pointer = Some(at);
      model.pointed_name();
      model.scroll_feed(by)
    });
    self.older(verb);
  }
}

#[cfg(test)]
#[path = "hand.test.rs"]
mod test;
