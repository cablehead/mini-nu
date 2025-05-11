# p1‑basic — embed Nushell and run one command from Rust in 15 lines

A microscopic example that boots the Nushell engine inside a Rust binary and executes **one** command.

---

## TL;DR — run it now

```bash
# From the repo root
cargo run -p p1-basic -- '"hello, nushell" | str upcase'
# → HELLO, NUSHELL
```

*(See the complete source in [`src/main.rs`](./src/main.rs).)*

---

## What this example adds

| Capability | Where it happens |
| ------------- | ------------- |
| Boot Nushell engine | [`create_default_context()`](https://docs.rs/nu-cmd-lang/latest/nu_cmd_lang/fn.create_default_context.html) |
| Inherit host env vars | [`gather_parent_env_vars()`](https://docs.rs/nu-cli/latest/nu_cli/fn.gather_parent_env_vars.html) |
| Parse + execute code | [`nu_parser::parse`](https://docs.rs/nu-parser/latest/nu_parser/fn.parse.html) → [`eval_block_with_early_return()`](https://docs.rs/nu-engine/latest/nu_engine/fn.eval_block_with_early_return.html) |

---

## Minimal walkthrough

```rust
// 1. Boot the engine
let mut engine = nu_cmd_lang::create_default_context();

// 2. Import parent‑process environment
nu_cli::gather_parent_env_vars(&mut engine, std::env::current_dir()?.as_ref());

// 3. Parse user‑supplied Nushell code
let mut ws = nu_protocol::engine::StateWorkingSet::new(&engine);
let block = nu_parser::parse(&mut ws, None, code.as_bytes(), false);
engine.merge_delta(ws.render())?;

// 4. Run the block and print the result
let mut stack = nu_protocol::engine::Stack::new();
let out = nu_engine::eval_block_with_early_return::<nu_protocol::debugger::WithoutDebug>(
    &engine,
    &mut stack,
    &block,
    nu_protocol::PipelineData::empty(),
)?;
println!("{:?}", out.into_value(nu_protocol::Span::unknown())?);
```

---

## Try these next

* `'"hi there" | str length'` – count characters
* `"10 + 20 * 3"` – quick math
* `"ls | where type == file | length"` – count files in the current dir

---

## Tests (optional)

```bash
cargo test -p p1-basic
```

Running tests verifies the engine prints **P1: HELLO WORLD!**

---

## What's next?

Want Ctrl‑C handling and background jobs?
→ **[Continue to `p2-background` ›](../p2-background/README.md)**

---

<details>
<summary>Internals &amp; further reading</summary>

* [How Nushell Code Gets Run](https://www.nushell.sh/book/how_nushell_code_gets_run.html) — deep dive into the pipeline that turns text into executed blocks.
* [nu-protocol API docs](https://docs.rs/nu-protocol/latest/nu_protocol/) — reference for [`EngineState`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.EngineState.html), [`Stack`](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.Stack.html), [`PipelineData`](https://docs.rs/nu-protocol/latest/nu_protocol/struct.PipelineData.html), etc.

</details>