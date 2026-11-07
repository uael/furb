---
name: rust
description: The bar of the Rust of furb. The rules that each Rust file meets, each with a sample, and the lints that hold them. Read it before you write or change a Rust file.
---

# The Rust of furb

Rust here is written as an expert writes it: small, typed, and with one owner for each thing. Simple code is short
code. Do not add a layer, a trait or a type that the code does not need.

The lints in `[workspace.lints]` of `Cargo.toml`, with `clippy.toml`, hold each
rule that a machine can hold. The rules below that no lint holds are for you and for the review.

## The debt

A module that does not meet the lints yet says so at its top, one line for each lint that it breaks:

```rust
#![expect(clippy::let_underscore_must_use, reason = "debt: a let _ drops a value that must be used")]
```

- An `expect` fails when its lint no longer fires. When your change removes the last hit of a lint in a module,
  delete its `expect` in the same change. So the list only gets shorter, and a clean module stays clean.
- Never add a debt `expect` for new code. Write new code to the bar.
- A true exception is an `#[expect(<lint>, reason = "<why>")]` on the item, never an `allow`. An exception that holds
  only for a cfg is a `cfg_attr`, as `#[cfg_attr(not(test), expect(dead_code, reason = "only the tests call it"))]`.
- The gates run clippy on more than one set of features and systems: the hooks on the workspace, and the jobs of the
  crate on macOS, Windows and Linux with the bindings of TypeScript. Run both. Code
  under `cfg(windows)` builds only on the runner of Windows.

  ```sh
  cargo clippy --workspace --all-targets -- -D warnings
  cargo clippy --all-targets --features typescript -- -D warnings
  ```

## The rules

1. An error is an enum of the concern that fails, which `thiserror` derives, never a `String`. `?` converts it. Its
   text is made once, by `Display`, where a person reads it.

   ```rust
   // Not this:
   fn load(path: &Path) -> Result<Catalog, String>
   // This:
   #[derive(Debug, thiserror::Error)]
   pub(crate) enum Error {
     #[error("{} cannot be read", path.display())]
     Read { path: PathBuf, source: io::Error },
     #[error("the catalog is not JSON")]
     Parse(#[from] serde_json::Error),
   }
   fn load(path: &Path) -> Result<Catalog, catalog::Error>
   ```

2. A value from a closed set is an enum, also inside an error. A protocol of JSON is an enum that serde tags, never a
   `Value` that the code matches by its `"type"`.

   ```rust
   // Not this:
   Err(format!("usage limit reached|{until}"))
   // This:
   Err(Retry::UsageLimit { until })

   #[derive(Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   enum Event { Delta { text: String }, Stop { reason: String } }
   ```

3. A state is one enum, not parallel `Option`s or flags that must agree.

   ```rust
   // Not this:
   struct Call { repl: Option<Repl>, answer: Option<Answer>, fault: Option<Fault> }
   // This:
   enum State { Idle(Repl), Answering(Answer), Ended(Fault) }
   ```

4. No `unwrap`. An `expect` only for an invariant that no type can hold, with the invariant as its message. First
   try to make a type hold it.

   ```rust
   let first = parts.first().expect("split gives at least one part");
   ```

5. No error is dropped. No `let _ =` of a `Result`, and no `.ok()` that hides a failure that the caller must see.
   When an error can be ignored, say what happens to it.

   ```rust
   // Not this:
   let _ = fs::remove_file(&stale);
   // This:
   if let Err(no) = fs::remove_file(&stale) {
     log::warn!("{} stays: {no}", stale.display());
   }
   ```

6. The thread of the life and the reactor never block: no `std::thread::sleep`, no `block_on`, and no loop that polls
   `try_wait`. Await the child under a timeout. Sync file work on the thread of the life is only for a small file that
   the life must have before its next fact.

   ```rust
   match tokio::time::timeout(limit, child.wait()).await {
     Ok(status) => status?,
     Err(_) => child.kill().await?,
   }
   ```

7. A function holds one concern and fits on a screen, 60 lines at most. A loop that dispatches events calls one
   function for each event. A sequence that is written twice is a function.

   ```rust
   match heard {
     Heard::Stand(stand) => self.stand(stand),
     Heard::Reply(reply) => self.reply(reply),
     Heard::Done(done) => self.done(done),
   }
   ```

8. A function of more than 4 parameters takes a struct. A tuple of more than 2 fields that leaves its function is a
   struct with named fields.

   ```rust
   // Not this:
   fn call(model: &str, prompt: &str, tools: &[Tool], effort: Effort, window: u32, stream: bool)
   // This:
   fn call(request: &Request)
   ```

9. Ownership before sharing. No `Arc<Mutex<_>>` for a state that one thread or one ear owns: a channel or `&mut`
   does it. `Rc<RefCell<_>>` only where two owners are real.

   ```rust
   // Not this:
   let seen = Arc::new(Mutex::new(HashMap::new()));
   // This: the ear owns it.
   struct Ear { seen: HashMap<String, Fact> }
   ```

10. Borrow before clone. Take `&str` and `&T`, look up with a borrowed key, and clone only to give ownership away.

    ```rust
    // Not this:
    fn named(names: &HashMap<String, Id>, name: String) -> Option<Id> { names.get(&name).copied() }
    // This:
    fn named(names: &HashMap<String, Id>, name: &str) -> Option<Id> { names.get(name).copied() }
    ```

11. An enum before `dyn` when the set of cases is closed. A generic before `dyn` when the build chooses one type.

    ```rust
    enum Transport { Stdio(Child), Sse(Client), Streamable(Client) }
    ```

12. Visibility is as small as it can be. `pub` is only for the API of the crate, which `src/lib.rs` gives.
    `pub(crate)` or smaller is for the rest.

    ```rust
    pub(crate) fn merged_path(paths: &[PathBuf]) -> OsString
    ```

13. A comment says what the code cannot say, as the section Comments of `AGENTS.md` says. The laws of an ear stand on
    its public type, not on a private closure. No comment says again what a name says.

    ```rust
    // Not this:
    /// Reads the file.
    fn read(path: &Path) -> io::Result<String>
    // This: no comment, or the one fact that a reader would not assume.
    /// A missing file reads as empty, since a new record holds no log.
    fn read(path: &Path) -> io::Result<String>
    ```

14. A rule that a lint can hold is a lint, not a sentence. Add it to `[workspace.lints]` of `Cargo.toml`, the one
    workspace, and give each module that breaks it a debt `expect`, as the section above says.

    ```toml
    [workspace.lints.clippy]
    unwrap_used = "deny"
    ```

15. A block gives its value by its tail: `if` and `match` are expressions, and a long chain of `else if` is one
    expression. An early `return` is only for a guard at the top of a function. `needless_return` holds the tail, and
    the review holds the rest.

    ```rust
    // Not this:
    fn kind(code: i32) -> Kind {
      if code == 0 {
        return Kind::Done;
      }
      if code < 0 {
        return Kind::Killed;
      }
      Kind::Failed
    }
    // This:
    fn kind(code: i32) -> Kind {
      match code {
        0 => Kind::Done,
        code if code < 0 => Kind::Killed,
        _ => Kind::Failed,
      }
    }
    ```

16. A generic parameter that only types one argument is `impl Trait` in that argument. A named generic is for a type
    that two places share, a return type, or a turbofish that a caller writes. A lifetime bound stays.

    ```rust
    // Not this:
    async fn heard<R: AsyncRead + Unpin>(reader: R) -> io::Result<Frame>
    // This:
    async fn heard(reader: impl AsyncRead + Unpin) -> io::Result<Frame>
    ```

17. Each item is imported at the top of its file, never in a function, and is called through a path of 2 segments at
    most. `clippy::absolute_paths`, with
    `absolute-paths-max-segments = 2` in `clippy.toml`, denies a longer path in `src/world.rs`,
    `src/binding.rs` and their modules. The review holds it in the other modules, until they pay it.

    ```rust
    // Not this:
    let name = std::borrow::Cow::Owned(text);
    // This:
    use std::borrow::Cow;
    let name = Cow::Owned(text);
    ```

18. Inside an `impl`, the type is `Self`. `clippy::use_self` denies the name of the type in
    `src/world.rs`, `src/binding.rs` and their modules. The review holds it in the other modules, until
    they pay it.

    ```rust
    // Not this:
    impl Listener {
      fn new(next: Server) -> Listener { Listener { next } }
    }
    // This:
    impl Listener {
      fn new(next: Server) -> Self { Self { next } }
    }
    ```

19. No `dbg!` stays in a change. `clippy::dbg_macro` denies it. Tell a person through the log, and a test through its
    assert.

    ```rust
    // Not this:
    dbg!(&frame);
    // This:
    log::debug!("heard {frame:?}");
    ```

20. No `unsafe` unless safe Rust cannot do it: a call of a C API, of N-API or of Windows, or a type that a host gives
    that only a check at run time keeps on its thread. First look for a safe call in `std` or in a crate that
    `Cargo.lock` holds, as `rustix::process::geteuid` for `libc::geteuid`. The lint `unsafe_code` denies the rest. A
    true site is the smallest item that holds it, with an `#[expect(unsafe_code, reason = "...")]` that says why safe
    Rust cannot do it, and a `// SAFETY:` comment just above each `unsafe`, which `undocumented_unsafe_blocks`
    requires.

    ```rust
    #[expect(unsafe_code, reason = "the database of the users is read only through libc")]
    fn named(uid: u32) -> Result<String, TransportError> {
      // SAFETY: each pointer is valid for the call, which writes at most the length of buffer.
      let code = unsafe { libc::getpwuid_r(uid, entry, buffer, size, &raw mut found) };
      // ...
    }
    ```

21. A file reads as a newspaper: its public and primary items first, then each helper below the first function that
    calls it. A function that grows past rule 7, or that two places repeat, moves to a method of the type that it
    works on, or to a file of its concern, not to the bottom of the file that it came from.

    ```rust
    pub(crate) fn listen(folder: &Path) -> Result<Listener, TransportError> {
      guarded(folder)?;
      bound(&folder.join(SOCKET))
    }

    fn guarded(folder: &Path) -> Result<(), TransportError> { /* ... */ }

    fn bound(socket: &Path) -> Result<Listener, TransportError> { /* ... */ }
    ```

22. A test compares the whole value with `assert_eq!` where the value is the same at each run: a struct, an enum, a
    list, a whole message. `contains` is for membership in a collection, and for a text that a process makes with
    parts that change, as a path or a time. A test that runs on more than one system calls a helper for an API of one
    system, as a symlink or a signal, and a helper that two suites share stands in `tests/common`, since each file of
    `tests/` is a crate of its own.

    ```rust
    // Not this:
    assert!(fault.to_string().contains("cannot be read"));
    // This:
    assert_eq!(fault.to_string(), "/tmp/yard/catalog.json cannot be read");
    ```

## Where furb keeps its own rule

The rules 15 to 22 come from the guidelines of Monty, the interpreter that the engine runs in. Furb keeps its own rule
in three places:

- A docstring is not on each struct, enum and function. The section Comments of `AGENTS.md` rules: a comment says
  only what the code cannot say, and a public type always gets one (rule 13).
- The tests of a module stand beside it as `<module>.test.rs`, as `AGENTS.md` says, not only in `tests/`, so a test
  reaches the private items that it checks.
- An `allow` is never written, with one true exception: `#![allow(dead_code, reason = ...)]` at
  `tests/common/yard.rs:5`. Each suite of `tests/` takes the part of the yard that it needs, so an `expect` there
  fails in the suite that uses each part.

The width of a line of Rust is 100, as `rustfmt.toml` says. `cargo fmt --all` writes it for code and wraps no
comment, so wrap each comment that you write to 100 by hand. `script/rustwidth.py`, which the hooks run, fails on
a comment line that your change adds past it.
