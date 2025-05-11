### p2‑background

## run a Nushell script in the background with Ctrl‑C cleanup

This example spawns **one** Nushell script in its own thread, tracks the
external processes it launches, and kills everything cleanly when you press
Ctrl‑C.

---

## TL;DR — run it now

```bash
# From the repo root
cargo run -p p2-background -- '^sleep 10; "done!"'
# Press Ctrl‑C before 10 s expires → "done!" never prints, sleep is killed.
```

_(Full source in [`src/main.rs`](./src/main.rs).)_

---

## What this example adds

| Capability                       | Where it happens                                                                                                                                                                                              |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ctrl‑C hook                      | [`setup_ctrlc_handler()`](./src/main.rs#L80-L103) using [`ctrlc`](https://docs.rs/ctrlc/latest/ctrlc/)                                                                                                        |
| Track a background job           | [`ThreadJob::new`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.ThreadJob.html) → add to [`engine_state.jobs`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.EngineState.html) |
| Kill external procs on interrupt | [`jobs.kill_and_remove(job_id)`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.Jobs.html#method.kill_and_remove)                                                                               |
| Non‑blocking wait loop           | Main thread polls `background_thread.is_finished()` every 100 ms                                                                                                                                              |
| `nu-system` process helpers      | [`ForegroundChild`](https://docs.rs/nu-system/latest/nu_system/struct.ForegroundChild.html) ensures the spawned `sleep` is in the job's process group                                                         |

---

## Minimal walkthrough

```rust
// 1. Boot the engine (same as p1‑basic)
let mut engine = nu_cmd_lang::create_default_context();
engine = nu_command::add_shell_command_context(engine);
nu_cli::gather_parent_env_vars(&mut engine, std::env::current_dir()?.as_ref());

// 2. Install a Ctrl‑C handler that toggles `interrupt`
let interrupt = setup_ctrlc_handler(&mut engine);

// 3. Spawn the Nushell script in its own thread + job context
let (job_id, handle) = run_script_in_background(
    Arc::new(engine),        // shared EngineState
    "^sleep 10; 'done!'",    // script to run
)?;

// 4. Poll: if thread ends -> exit; if Ctrl‑C -> kill job + exit
loop {
    if handle.is_finished() { break; }
    if interrupt.load(Ordering::Relaxed) {
        let mut jobs = engine.jobs.lock().unwrap();
        let _ = jobs.kill_and_remove(job_id);
        break;
    }
    std::thread::sleep(Duration::from_millis(100));
}
```

---

## Try these scripts

| Purpose               | Command to pass                                  |
| --------------------- | ------------------------------------------------ |
| Upper‑case a string   | `'"hello!"  \| str upcase'`                      |
| Count files then wait | `'ls \| where type == file \| length; ^sleep 5'` |
| Stress Ctrl‑C cleanup | `'^yes \| head 100000'`                          |

---

## Tests (optional)

```bash
cargo test -p p2-background
```

- Two tests run: one checks normal output, the other sends **SIGINT** and
  asserts that **both** the Rust process **and** its child `sleep` process
  terminate.

---

## What's next?

Need custom commands and multiple concurrent pipelines? →
**[Jump to `p3-the‑works` ›](../p3-the-works/README.md)** _(Missed the basics?
[Back to `p1-basic` ›](../p1-basic/README.md))_

---

## Internals & further reading</summary

- **Jobs & signals** — `nu-protocol`'s
  [`Jobs`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.Jobs.html)
  table,
  [`ThreadJob`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.ThreadJob.html)
  and
  [`Signals`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.Signals.html).
- **Process management** — `nu-system`'s
  [`ForegroundChild`](https://docs.rs/nu-system/latest/nu_system/struct.ForegroundChild.html)
  wrapper.
- **Background jobs in Nushell** — the official shell docs on job semantics.
  [https://www.nushell.sh/book/background_jobs.html](https://www.nushell.sh/book/background_jobs.html)
- **Tokio's `spawn_blocking`** — why we off‑load CPU‑heavy work.
  [https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
