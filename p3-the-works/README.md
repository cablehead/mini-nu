### p3‑the‑works

## custom commands, parallel pipelines, and graceful shutdown

The "everything bagel" example: it boots Nushell with a **custom command**
(`warble`), compiles a **user‑supplied closure**, spawns that closure for every
stdin line on its own Tokio task, _and_ shuts everything down cleanly on Ctrl‑C.
Think of it as a miniature Nushell inside your Rust app.

---

## TL;DR

```
# From the repo root
cargo run -p p3-the-works -- '{|n| $"Input ($n): ($in) processed!" }'
# type a few lines, press Ctrl-D → each line prints back with its index
```

_(Full source in [`src/main.rs`](./src/main.rs).)_

---

## What this example adds

| Capability                                  | Where it happens                                                                                                                                                                                                         |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Custom command** `warble`                 | `add_custom_commands()` registers `Warble` with [`StateWorkingSet::add_decl`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.StateWorkingSet.html#method.add_decl)                                         |
| **CLI helpers** (history, completion, etc.) | [`add_cli_context`](https://docs.rs/nu-cli/latest/nu_cli/)                                                                                                                                                               |
| **Closure parsing & caching**               | `parse_closure()` uses [`nu_parser::parse`](https://docs.rs/nu-parser/latest/nu_parser/fn.parse.html) then converts the result to [`Closure`](https://www.nushell.sh/lang-guide/chapters/types/basic_types/closure.html) |
| **Concurrent jobs**                         | Every stdin line is sent down a channel and processed via `tokio::spawn`/`spawn_blocking` (docs: [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html))                                     |
| **Job accounting & clean exit**             | Shared `active_jobs: Arc<Mutex<usize>>` tracks in‑flight work (why [`Arc + Mutex`](https://itsallaboutthebit.com/arc-mutex/))                                                                                            |
| **Process‑group cleanup**                   | Each task clones the current [`ThreadJob`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.ThreadJob.html) so external commands get killed on interrupt                                                     |
| **Ctrl‑C handling**                         | `setup_ctrlc_handler()` with the [`ctrlc` crate](https://crates.io/crates/ctrlc)                                                                                                                                         |
| **Background‑job semantics**                | Mirrors Nushell's own [background‑jobs design](https://www.nushell.sh/book/background_jobs.html)                                                                                                                         |

---

## Minimal architecture walkthrough

```rust
// 1. Engine with core + CLI + custom command -------------------------------
let mut engine = nu_cmd_lang::create_default_context();        // core built‑ins
engine = nu_command::add_shell_command_context(engine);        // OS helpers
engine = nu_cli::add_cli_context(engine);                      // history, prompt
engine = add_custom_commands(engine);                          // our `warble`

// 2. Parse user‑provided closure once --------------------------------------
let closure = parse_closure(&mut engine, closure_snippet)?;    // nu_parser::parse + eval

// 3. Set up Ctrl‑C ---------------------------------------------------------
let interrupt = setup_ctrlc_handler(&mut engine)?;             // ctrlc crate

// 4. Fan‑in stdin lines, fan‑out jobs --------------------------------------
tokio::spawn(async move {
    // For every line:
    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            process_job(&engine, &closure, &line, job_no)      // eval_block*
        }).await.ok();
    });
});

// 5. Main thread waits until active_jobs == 0 or interrupt -----------------
```

_(See code comments in [`src/main.rs`](./src/main.rs) for full context.)_

---

## Try these demos

| Demo                                    | Command to pass                                                                                   |
| --------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Upper‑case each line                    | `'{\|_\| $in \| str upcase }'`                                                                    |
| Use the **custom** `warble` command     | `'{\|_\| warble}'`                                                                                |
| Show task finish order (100‑200‑300 ms) | `'{\|i\| sleep (($in)ms \| into duration); $"job ($i) done"}'` then pipe `100 200 300` into stdin |

---

## Tests (optional)

```
cargo test -p p3-the-works
```

- Suite checks:

1. **Execution order** for staggered sleeps
2. **Custom command** output
3. **SIGINT** kills parent **and** child `sleep` processes

---

## What's next?

Want to create a restricted sandbox environment? →
**[Continue to `p4-sandbox` →](../p4-sandbox/README.md)**

---

## Need a refresher?

- [← Back to `p2-background`](../p2-background/README.md)
- [← Back to `p1-basic`](../p1-basic/README.md)

---

## Further reading

- **EngineState / ThreadJob API** — see
  [`nu-protocol`](https://docs.rs/nu-protocol) docs.
- **Custom commands in Nushell** — official guide.
  ([Nushell](https://www.nushell.sh/book/custom_commands.html))
- **Parsing and type‑directed evaluation** — Nushell parser intro.
  ([Docs.rs](https://docs.rs/nu-parser))
- **Background jobs in Nushell** — why they're implemented as threads, not
  processes. ([Nushell](https://www.nushell.sh/book/background_jobs.html))
- **Tokio's `spawn_blocking`** — when to off‑load CPU work.
  ([Docs.rs](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html))
- **Handling Ctrl‑C in Rust** — the `ctrlc` crate.
  ([Crates.io](https://crates.io/crates/ctrlc))
