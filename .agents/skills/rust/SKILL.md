---
name: rust
description: The bar of the Rust of furb. The rules that each Rust file meets, each with a sample, and the lints that hold them. Read it before you write or change a Rust file.
---

# The Rust of furb

Rust here is written as an expert writes it: small, typed, and with one owner for each thing. Simple code is short
code. Do not add a layer, a trait or a type that the code does not need.

The lints in `[workspace.lints]` of `Cargo.toml` and of `gui/src-tauri/Cargo.toml`, with `clippy.toml`, hold each
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
- The gates run clippy on more than one set of features and systems. Run
  `cargo clippy --workspace --all-targets -- -D warnings`, the same with `--features typescript`, and the same in
  `gui/src-tauri`.

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

14. A rule that a lint can hold is a lint, not a sentence. Add it to `[workspace.lints]` in both workspaces, and give
    each module that breaks it a debt `expect`, as the section above says.

    ```toml
    [workspace.lints.clippy]
    unwrap_used = "deny"
    ```

The width of a line of Rust is 100, as `rustfmt.toml` says. `cargo fmt --all` writes it.
