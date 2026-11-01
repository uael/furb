//! furb, the desktop app. Its core starts the host of a life, which opens the projects and the lives as the TUI opens
//! them, on the Workspaces and the Session of the TUI, and it gives the window the address of the socket of that host.
//! The window, the UI of gui/src, sends the calls of the operator to the host, and draws the life that the host sends
//! with the model of a life that the TUI draws too.
//!
//! `furb-gui` opens on the projects and their records. `--record RECORD` goes on with a record, `--cwd DIRECTORY`
//! starts a new life in a project, and `--demo` starts the demo of the TUI, which asks no model. `--model`, `--effort`
//! and `--roster` name the models, as the TUI takes them. `--check` ends the app once its window drew the whole life of
//! `--record` and its stdin ended, and prints the process of its web view and the count of its chains, threads and
//! acts.
//! `FURB_GUI_HIDDEN` starts the window hidden.

mod host;

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;

use std::{
  path::PathBuf,
  process::{Child, ChildStdin},
  sync::{
    Condvar, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};

use tauri::{Manager, webview::PageLoadEvent};

/// The host as the core holds it: the address of its socket once it said it, or why it did not start, and its
/// process, whose stdin the core holds open while the app runs.
#[derive(Default)]
struct Held {
  address: Mutex<Option<Result<String, String>>>,
  said: Condvar,
  child: Mutex<Option<(Child, Option<ChildStdin>)>>,
}

/// Whether the app runs as a check: it ends once its window drew the whole life and its stdin ended, or fails after a
/// while.
static CHECK: AtomicBool = AtomicBool::new(false);

/// Whether the window of a check drew the whole life, after which only the end of its stdin ends the app.
static DREW: AtomicBool = AtomicBool::new(false);

/// When the core of a check started, from which each step of the check says when it came.
static STARTED: OnceLock<Instant> = OnceLock::new();

/// A step of a check, with the seconds since the core started, so a check that does not end says where it stopped.
fn note(step: &str) {
  if CHECK.load(Ordering::Relaxed) {
    let seconds = STARTED.get_or_init(Instant::now).elapsed().as_secs_f32();
    eprintln!("furb-gui: {seconds:.1}s {step}");
  }
}

/// The process of the web content of a web view, which a check prints, so a test measures that process alone.
#[cfg(target_os = "macos")]
fn content(webview: &tauri::Webview) {
  let _ = webview.with_webview(|view| {
    let inner = view.inner().cast::<objc2::runtime::AnyObject>();
    if inner.is_null() {
      return;
    }
    // `_webProcessIdentifier` is the property of WKWebView, private to WebKit, that names its WebContent process.
    let pid: i32 = unsafe { objc2::msg_send![&*inner, _webProcessIdentifier] };
    println!("furb-gui: the web view runs in process {pid}");
  });
}

/// The address of the socket of the host, once the host said it. The window asks for it once its page loaded, so its
/// web content runs then, and a check prints its process.
#[tauri::command(async)]
fn host(webview: tauri::Webview, held: tauri::State<'_, Held>) -> Result<String, String> {
  note("the page asks for the host");
  #[cfg(target_os = "macos")]
  if CHECK.load(Ordering::Relaxed) {
    content(&webview);
  }
  #[cfg(not(target_os = "macos"))]
  let _ = webview;
  let mut address = held.address.lock().map_err(|_| "the core lost its host".to_owned())?;
  while address.is_none() {
    address = held.said.wait(address).map_err(|_| "the core lost its host".to_owned())?;
  }
  address.clone().unwrap_or_else(|| Err("the core lost its host".to_owned()))
}

/// What the window drew of the whole life, which it tells once the host read it: the chains, the threads and the
/// acts, and whether they are those of the log of the life or those that its record kept. A check prints them, then
/// ends the app once its stdin ends, and the app ends its host first, so the log of the life stands whole when the
/// check ends.
#[tauri::command]
fn drew(app: tauri::AppHandle, chains: usize, threads: usize, acts: usize, logged: bool) {
  if CHECK.load(Ordering::Relaxed) {
    let of = if logged { "its log" } else { "its record" };
    println!(
      "furb-gui: the window drew {chains} chains, {threads} threads and {acts} acts of {of}"
    );
    DREW.store(true, Ordering::Relaxed);
    // The web content ends with the app, so the app waits until the one that runs the check ends its stdin, and that
    // one measures the process of the web view after the window drew, while it stands.
    std::thread::spawn(move || {
      let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
      app.exit(0);
    });
  }
}

/// Whether the words of the command line ask for a check, and the words of the host that they ask for. A check needs
/// a record, keeps the list of the projects of its life apart from the list of the operator, which it leaves as it is,
/// and enables no extension that the record does not run, so the life that it draws is the one of the record.
fn asked(words: &[String], scratch: &std::path::Path) -> Result<(bool, Vec<String>), String> {
  let mut out = Vec::new();
  let (mut check, mut record) = (false, false);
  let mut words = words.iter();
  while let Some(word) = words.next() {
    match word.as_str() {
      "--check" => check = true,
      "--demo" => out.push(word.clone()),
      "--record" | "--cwd" | "--model" | "--effort" | "--roster" => {
        let value = words.next().ok_or_else(|| format!("{word} needs a value."))?;
        record |= word == "--record";
        out.extend([word.clone(), value.clone()]);
      }
      other => return Err(format!("{other} is no word of furb-gui.")),
    }
  }
  if check {
    if !record {
      return Err("--check needs --record.".to_owned());
    }
    out.extend(["--list".to_owned(), scratch.join("workspaces.json").display().to_string()]);
    out.push("--no-extensions".to_owned());
  }
  Ok((check, out))
}

fn main() {
  let words: Vec<String> = std::env::args().skip(1).collect();
  let scratch: PathBuf =
    std::env::temp_dir().join(format!("furb-gui-check-{}", std::process::id()));
  let words = match asked(&words, &scratch) {
    Ok((check, words)) => {
      CHECK.store(check, Ordering::Relaxed);
      note("the core starts");
      words
    }
    Err(why) => {
      eprintln!("{why}");
      std::process::exit(2);
    }
  };
  let mut app = tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(move |app| {
      if CHECK.load(Ordering::Relaxed) {
        std::thread::spawn(|| {
          std::thread::sleep(Duration::from_secs(600));
          if !DREW.load(Ordering::Relaxed) {
            eprintln!("furb-gui: the window drew no whole life in 600 seconds");
            std::process::exit(1);
          }
        });
      }
      note("the app is set up");
      app.manage(Held::default());
      // A window that starts hidden stands for a window behind others, where WebKit draws no frame, which a test of
      // the check holds.
      if std::env::var_os("FURB_GUI_HIDDEN").is_some()
        && let Some(window) = app.get_webview_window("main")
      {
        let _ = window.hide();
      }
      let handle = app.handle().clone();
      // The host starts beside the window, which asks for its address and waits for it.
      std::thread::spawn(move || {
        let held = handle.state::<Held>();
        let started = host::start(&words).map(|(mut child, url)| {
          let stdin = child.stdin.take();
          if let Ok(mut kept) = held.child.lock() {
            *kept = Some((child, stdin));
          }
          url
        });
        if let Err(why) = &started {
          eprintln!("furb-gui: {why}");
        }
        note(if started.is_ok() { "the host said its address" } else { "the host did not start" });
        if let Ok(mut address) = held.address.lock() {
          *address = Some(started);
          held.said.notify_all();
        }
      });
      Ok(())
    })
    .on_page_load(|_, payload| {
      note(match payload.event() {
        PageLoadEvent::Started => "the page starts to load",
        PageLoadEvent::Finished => "the page finished to load",
      });
    })
    .invoke_handler(tauri::generate_handler![host, drew])
    .build(tauri::generate_context!())
    .expect("the window of furb opens");
  // A check runs as an accessory of the system from its launch: it has no tile in the Dock, no place in the switcher
  // of apps, and takes the front from no app, so no quit that the Dock sends for the name of the app reaches the app
  // that the operator has open, and a check ends only by its own exit.
  #[cfg(target_os = "macos")]
  if CHECK.load(Ordering::Relaxed) {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
  }
  app.run(|handle, event| {
    // The host ends when its stdin ends, and saves each life that it holds before it ends.
    if let tauri::RunEvent::Exit = event
      && let Ok(mut kept) = handle.state::<Held>().child.lock()
      && let Some((mut child, stdin)) = kept.take()
    {
      drop(stdin);
      for _ in 0..50 {
        if matches!(child.try_wait(), Ok(Some(_))) {
          return;
        }
        std::thread::sleep(Duration::from_millis(100));
      }
      let _ = child.kill();
    }
  });
}
