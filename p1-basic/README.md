````md
# p1‑basic – embed Nushell and run one command from Rust in 15 lines

## TL;DR (30‑second quick‑start)

```bash
# Build & run
cargo run -p p1-basic -- '"hello, nushell" | str upcase'
# → HELLO, NUSHELL
````

▶︎ **Full source:** [`src/main.rs`](./src/main.rs)

---

## What this example adds

| Capability            | Where it happens                                      |
| --------------------- | ----------------------------------------------------- |
| Boot Nushell engine   | `create_default_context()`                            |
| Inherit host env vars | `gather_parent_env_vars()`                            |
| Parse + execute code  | `nu_parser::parse` → `eval_block_with_early_return()` |

---

## Minimal walkthrough

```rust
// 1. Boot engine
let mut engine = create_default_context();

// 2. Inherit env
gather_parent_env_vars(&mut engine, std::env::current_dir()?.as_ref());

// 3. Parse one‑liner from CLI
let mut ws = StateWorkingSet::new(&engine);
let block = nu_parser::parse(&mut ws, None, code.as_bytes(), false);
engine.merge_delta(ws.render())?;

// 4. Run & print
let mut stack = Stack::new();
let out = eval_block_with_early_return::<WithoutDebug>(
    &engine, &mut stack, &block, PipelineData::empty()
)?.into_value(Span::unknown())?;
println!("{out}");
```

*(See the full file for robust error handling.)*

---

## Try these next

```bash
'"hi" | str length'             # string length
"10 + 20 * 3"                   # math
"ls | where type == file | length"   # file count
```

---

## Tests (optional)

```bash
cargo test -p p1-basic
```

> Ensures the engine prints **P1: HELLO WORLD!**

---

## What’s next?

Need background jobs & Ctrl‑C cleanup?
→ **[p2‑background ›](../p2-background/README.md)**

---

## Internals (for the curious)

<details>
<summary>EngineState, Stack, PipelineData …</summary>

* **Nushell book – *How Nushell Code Gets Run*** – in‑depth look at parsing, compilation and evaluation. ([Nushell][1])
* **API docs for `nu‑protocol`** – types like `EngineState`, `Stack`, `Value`, etc.
  [https://docs.rs/nu-protocol/0.104.0/nu\_protocol/](https://docs.rs/nu-protocol/0.104.0/nu_protocol/) ([docs.rs][2])

</details>
```
::contentReference[oaicite:2]{index=2}

[1]: https://www.nushell.sh/book/how_nushell_code_gets_run.html?utm_source=chatgpt.com "How Nushell Code Gets Run"
[2]: https://docs.rs/nu-protocol?utm_source=chatgpt.com "nu_protocol - Rust - Docs.rs"
